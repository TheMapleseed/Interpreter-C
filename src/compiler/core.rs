// src/compiler/core.rs
use std::sync::Arc;
use llvm_sys::*;
use llvm_sys::prelude::*;
use llvm_sys::core::*;
use std::ffi::{CString, CStr};
use parking_lot::RwLock;

pub struct CompilerCore {
    // LLVM context and core components
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    
    // Target information
    target_machine: LLVMTargetMachineRef,
    target_data: LLVMTargetDataRef,
    
    // Optimization pipeline
    pass_manager: LLVMPassManagerRef,
    
    // ABI handler
    abi_handler: Arc<ABIHandler>,
}

impl CompilerCore {
    pub unsafe fn new(target_triple: &str) -> Result<Self, CompilerError> {
        // Initialize LLVM
        LLVM_InitializeNativeTarget();
        LLVM_InitializeNativeAsmPrinter();
        
        // Create core components
        let context = LLVMContextCreate();
        let module = LLVMModuleCreateWithNameInContext(
            b"jit_module\0".as_ptr() as *const _,
            context
        );
        let builder = LLVMCreateBuilderInContext(context);
        
        // Setup target
        let target_triple = CString::new(target_triple)?;
        let mut target = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();
        
        if LLVMGetTargetFromTriple(
            target_triple.as_ptr(),
            &mut target,
            &mut error
        ) != 0 {
            let error_str = CStr::from_ptr(error as *const _)
                .to_string_lossy()
                .into_owned();
            LLVMDisposeMessage(error);
            return Err(CompilerError::TargetInitialization(error_str));
        }

        // Create target machine
        let cpu = CString::new("generic")?;
        let features = CString::new("")?;
        let target_machine = LLVMCreateTargetMachine(
            target,
            target_triple.as_ptr(),
            cpu.as_ptr(),
            features.as_ptr(),
            LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
            LLVMRelocMode::LLVMRelocPIC,
            LLVMCodeModel::LLVMCodeModelDefault,
        );

        let target_data = LLVMCreateTargetDataLayout(target_machine);
        LLVMSetModuleDataLayout(module, target_data);

        // Create pass manager
        let pass_manager = LLVMCreatePassManager();
        LLVMAddInstructionCombiningPass(pass_manager);
        LLVMAddReassociatePass(pass_manager);
        LLVMAddGVNPass(pass_manager);
        LLVMAddCFGSimplificationPass(pass_manager);
        
        Ok(CompilerCore {
            context,
            module,
            builder,
            target_machine,
            target_data,
            pass_manager,
            abi_handler: Arc::new(ABIHandler::new(target_data)?),
        })
    }

    pub unsafe fn compile_function(
        &self,
        name: &str,
        args: &[Type],
        return_type: Type,
        body: &[Instruction],
    ) -> Result<*mut u8, CompilerError> {
        // Create function type
        let func_type = self.create_function_type(args, return_type)?;
        
        // Create function
        let name = CString::new(name)?;
        let function = LLVMAddFunction(
            self.module,
            name.as_ptr(),
            func_type
        );
        
        // Create entry block
        let entry = LLVMAppendBasicBlockInContext(
            self.context,
            function,
            b"entry\0".as_ptr() as *const _
        );
        LLVMPositionBuilderAtEnd(self.builder, entry);
        
        // Generate instructions
        for instruction in body {
            self.generate_instruction(instruction)?;
        }
        
        // Verify function
        let mut error = std::ptr::null_mut();
        if LLVMVerifyFunction(function, LLVMVerifierFailureAction::LLVMPrintMessageAction) != 0 {
            let error_str = CStr::from_ptr(error as *const _)
                .to_string_lossy()
                .into_owned();
            LLVMDisposeMessage(error);
            return Err(CompilerError::FunctionVerification(error_str));
        }

        // Optimize
        LLVMRunPassManager(self.pass_manager, self.module);

        // Generate code
        let mut error = std::ptr::null_mut();
        let mut size = 0;
        let code_ptr = LLVMCreateMCJITCompilerForModule(
            &mut self.execution_engine,
            self.module,
            &mut self.jit_options,
            &mut error
        );

        if code_ptr.is_null() {
            let error_str = CStr::from_ptr(error as *const _)
                .to_string_lossy()
                .into_owned();
            LLVMDisposeMessage(error);
            return Err(CompilerError::CodeGeneration(error_str));
        }

        Ok(code_ptr as *mut u8)
    }

    unsafe fn create_function_type(
        &self,
        args: &[Type],
        return_type: Type,
    ) -> Result<LLVMTypeRef, CompilerError> {
        let mut param_types: Vec<LLVMTypeRef> = Vec::with_capacity(args.len());
        
        for arg_type in args {
            param_types.push(self.convert_type(arg_type)?);
        }
        
        let return_type = self.convert_type(&return_type)?;
        
        Ok(LLVMFunctionType(
            return_type,
            param_types.as_mut_ptr(),
            param_types.len() as u32,
            0 // Not vararg
        ))
    }

    unsafe fn convert_type(&self, ty: &Type) -> Result<LLVMTypeRef, CompilerError> {
        match ty {
            Type::Void => Ok(LLVMVoidTypeInContext(self.context)),
            Type::Int8 => Ok(LLVMInt8TypeInContext(self.context)),
            Type::Int16 => Ok(LLVMInt16TypeInContext(self.context)),
            Type::Int32 => Ok(LLVMInt32TypeInContext(self.context)),
            Type::Int64 => Ok(LLVMInt64TypeInContext(self.context)),
            Type::Float => Ok(LLVMFloatTypeInContext(self.context)),
            Type::Double => Ok(LLVMDoubleTypeInContext(self.context)),
            Type::Pointer(inner) => {
                let inner_type = self.convert_type(inner)?;
                Ok(LLVMPointerType(inner_type, 0))
            },
            Type::Array(inner, size) => {
                let inner_type = self.convert_type(inner)?;
                Ok(LLVMArrayType(inner_type, *size))
            },
            Type::Struct(fields) => {
                let mut field_types: Vec<LLVMTypeRef> = Vec::with_capacity(fields.len());
                for field in fields {
                    field_types.push(self.convert_type(field)?);
                }
                Ok(LLVMStructTypeInContext(
                    self.context,
                    field_types.as_mut_ptr(),
                    field_types.len() as u32,
                    0 // Not packed
                ))
            },
        }
    }
}

impl Drop for CompilerCore {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposePassManager(self.pass_manager);
            LLVMDisposeTargetData(self.target_data);
            LLVMDisposeTargetMachine(self.target_machine);
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }
}

#[derive(Debug)]
pub enum CompilerError {
    TargetInitialization(String),
    FunctionVerification(String),
    CodeGeneration(String),
    TypeConversion(String),
    ABIViolation(String),
}

// Core types that match C ABI
#[derive(Debug, Clone)]
pub enum Type {
    Void,
    Int8,
    Int16,
    Int32,
    Int64,
    Float,
    Double,
    Pointer(Box<Type>),
    Array(Box<Type>, u32),
    Struct(Vec<Type>),
}

// Instructions that map to LLVM IR
#[derive(Debug)]
pub enum Instruction {
    Return(Option<Value>),
    Load(Value),
    Store(Value, Value),
    Call(String, Vec<Value>),
    Add(Value, Value),
    Sub(Value, Value),
    Mul(Value, Value),
    Div(Value, Value),
    Alloca(Type),
    GetElementPtr(Value, Vec<Value>),
    BitCast(Value, Type),
    ICmp(ICmpOp, Value, Value),
    FCmp(FCmpOp, Value, Value),
    Branch(Value, String, String),
    Phi(Type, Vec<(Value, String)>),
}

#[derive(Debug)]
pub enum Value {
    Constant(i64),
    Float(f64),
    Register(u32),
    Global(String),
}

// Example usage:
/*
fn main() -> Result<(), CompilerError> {
    unsafe {
        let compiler = CompilerCore::new("x86_64-unknown-linux-gnu")?;

        // Define a simple add function
        let code = compiler.compile_function(
            "add",
            &[Type::Int32, Type::Int32],
            Type::Int32,
            &[
                Instruction::Add(
                    Value::Register(0),
                    Value::Register(1)
                ),
                Instruction::Return(Some(Value::Register(2)))
            ]
        )?;

        // Cast to function pointer and call
        let add_fn: extern "C" fn(i32, i32) -> i32 = std::mem::transmute(code);
        println!("Result: {}", add_fn(2, 3));

        Ok(())
    }
}
*/
