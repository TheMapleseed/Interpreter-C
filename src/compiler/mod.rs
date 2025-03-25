// src/compiler/mod.rs
use std::sync::Arc;
use std::collections::HashMap;
use llvm_sys::*;
use llvm_sys::prelude::*;
use llvm_sys::core::*;
use llvm_sys::target::*;
use llvm_sys::execution_engine::*;
use std::ffi::{CString, CStr};

// New imports for architecture support
use crate::arch::{Architecture, ArchitectureRegistry};

pub struct CompilerSystem {
    // Core compilation components
    frontend: Frontend,
    middle_end: MiddleEnd,
    backend: Backend,
    
    // Target information
    target_machine: LLVMTargetMachineRef,
    target_data: LLVMTargetDataRef,
    
    // System interfaces
    runtime: RuntimeSystem,
    linker: Linker,
    
    // ABI handling
    abi_handler: ABIHandler,
    
    // Architecture support
    architecture_registry: Arc<ArchitectureRegistry>,
    current_architecture: Architecture,
}

impl CompilerSystem {
    pub unsafe fn new(target_triple: &str) -> Result<Self, CompilerError> {
        // Initialize LLVM
        LLVM_InitializeAllTargets();
        LLVM_InitializeAllTargetInfos();
        LLVM_InitializeAllTargetMCs();
        LLVM_InitializeAllAsmParsers();
        LLVM_InitializeAllAsmPrinters();
        
        // Create architecture registry
        let architecture_registry = Arc::new(ArchitectureRegistry::new());
        
        // Determine architecture from target triple
        let arch = Self::determine_architecture_from_triple(target_triple)?;
        
        // Create target machine
        let target_machine = Self::create_target_machine(target_triple)?;
        let target_data = LLVMCreateTargetDataLayout(target_machine);
        
        Ok(CompilerSystem {
            frontend: Frontend::new()?,
            middle_end: MiddleEnd::new()?,
            backend: Backend::new(target_machine, arch)?,
            target_machine,
            target_data,
            runtime: RuntimeSystem::new()?,
            linker: Linker::new()?,
            abi_handler: ABIHandler::new(target_data)?,
            architecture_registry,
            current_architecture: arch,
        })
    }

    /// Determine the architecture from the target triple
    unsafe fn determine_architecture_from_triple(target_triple: &str) -> Result<Architecture, CompilerError> {
        if target_triple.starts_with("x86_64") {
            Ok(Architecture::X86_64)
        } else if target_triple.starts_with("aarch64") || target_triple.starts_with("arm64") {
            Ok(Architecture::AArch64)
        } else if target_triple.starts_with("arm") {
            Ok(Architecture::Arm)
        } else {
            Err(CompilerError::UnsupportedArchitecture(target_triple.to_string()))
        }
    }

    pub unsafe fn compile_file(
        &self,
        input_file: &str,
        output_file: &str,
        options: &CompilerOptions
    ) -> Result<(), CompilerError> {
        // Parse input file
        let ast = self.frontend.parse_file(input_file)?;
        
        // Generate IR
        let module = self.middle_end.generate_ir(&ast)?;
        
        // Optimize
        if options.optimization_level > 0 {
            self.middle_end.optimize_module(&module, options.optimization_level)?;
        }
        
        // Generate code
        let obj_file = self.backend.generate_code(&module, output_file)?;
        
        // Link if needed
        if options.link {
            self.linker.link(obj_file, output_file, &options.link_options)?;
        }
        
        Ok(())
    }

    pub unsafe fn jit_compile(
        &self,
        source: &str,
        options: &JITOptions
    ) -> Result<*mut u8, CompilerError> {
        // Parse source
        let ast = self.frontend.parse_string(source)?;
        
        // Generate IR with JIT options
        let module = self.middle_end.generate_ir_for_jit(&ast, options)?;
        
        // Optimize for JIT
        self.middle_end.optimize_for_jit(&module)?;
        
        // JIT compile
        let code_ptr = self.backend.jit_compile(&module)?;
        
        // Setup runtime
        self.runtime.setup_jit_function(code_ptr)?;
        
        Ok(code_ptr)
    }
    
    /// Compile assembly code directly
    pub unsafe fn compile_assembly(
        &self,
        asm_code: &str,
        output_file: &str,
        options: &AssemblyOptions
    ) -> Result<(), CompilerError> {
        // Get architecture support
        let arch_support = self.architecture_registry.get_support(options.target_architecture.unwrap_or(self.current_architecture))
            .ok_or_else(|| CompilerError::UnsupportedArchitecture(format!("{:?}", options.target_architecture)))?;
        
        // Parse assembly code
        let asm_ast = arch_support.asm_parser.parse(asm_code)
            .map_err(|e| CompilerError::AssemblyParsingError(format!("{:?}", e)))?;
        
        // Encode assembly into machine code
        let encoded = arch_support.instruction_encoder.encode_asm_block(&asm_ast.blocks[0])
            .map_err(|e| CompilerError::AssemblyEncodingError(format!("{:?}", e)))?;
        
        // Create object file
        let obj_file = self.backend.create_object_file_from_machine_code(&encoded, output_file)?;
        
        // Link if needed
        if options.link {
            self.linker.link(obj_file, output_file, &options.link_options)?;
        }
        
        Ok(())
    }

    unsafe fn create_target_machine(
        target_triple: &str
    ) -> Result<LLVMTargetMachineRef, CompilerError> {
        let target_triple = CString::new(target_triple)
            .map_err(|_| CompilerError::InvalidTargetTriple)?;
            
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

        let cpu = CString::new("generic").unwrap();
        let features = CString::new("").unwrap();
        
        let machine = LLVMCreateTargetMachine(
            target,
            target_triple.as_ptr(),
            cpu.as_ptr(),
            features.as_ptr(),
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
            LLVMRelocMode::LLVMRelocPIC,
            LLVMCodeModel::LLVMCodeModelDefault,
        );

        if machine.is_null() {
            return Err(CompilerError::TargetMachineCreation);
        }

        Ok(machine)
    }
}

#[derive(Debug)]
pub struct CompilerOptions {
    pub optimization_level: u32,
    pub link: bool,
    pub link_options: LinkOptions,
    pub debug_info: bool,
    pub target_features: Vec<String>,
    pub target_architecture: Option<Architecture>,
}

#[derive(Debug)]
pub struct JITOptions {
    pub optimization_level: u32,
    pub enable_fast_isel: bool,
    pub enable_guard_pages: bool,
    pub stack_size: usize,
    pub target_architecture: Option<Architecture>,
}

#[derive(Debug)]
pub struct AssemblyOptions {
    pub link: bool,
    pub link_options: LinkOptions,
    pub target_architecture: Option<Architecture>,
}

#[derive(Debug)]
pub struct LinkOptions {
    pub libraries: Vec<String>,
    pub library_paths: Vec<String>,
    pub static_link: bool,
    pub strip_symbols: bool,
}

#[derive(Debug)]
pub enum CompilerError {
    InvalidTargetTriple,
    TargetInitialization(String),
    TargetMachineCreation,
    UnsupportedArchitecture(String),
    AssemblyParsingError(String),
    AssemblyEncodingError(String),
    Frontend(FrontendError),
    MiddleEnd(MiddleEndError),
    Backend(BackendError),
    Runtime(RuntimeError),
    Linker(LinkerError),
    ABI(ABIError),
}

// Example usage:
/*
fn main() -> Result<(), CompilerError> {
    unsafe {
        let compiler = CompilerSystem::new("x86_64-unknown-linux-gnu")?;

        // Static compilation
        let options = CompilerOptions {
            optimization_level: 2,
            link: true,
            link_options: LinkOptions {
                libraries: vec!["c".to_string()],
                library_paths: vec![],
                static_link: false,
                strip_symbols: false,
            },
            debug_info: true,
            target_features: vec!["+sse4.2".to_string()],
            target_architecture: None,
        };

        compiler.compile_file("input.c", "output", &options)?;

        // JIT compilation
        let jit_options = JITOptions {
            optimization_level: 2,
            enable_fast_isel: true,
            enable_guard_pages: true,
            stack_size: 8 * 1024 * 1024,
            target_architecture: None,
        };

        let code = r#"
            int add(int a, int b) {
                return a + b;
            }
        "#;

        let func_ptr = compiler.jit_compile(code, &jit_options)?;
        
        // Use the JIT compiled function
        let add_func: extern "C" fn(i32, i32) -> i32 = std::mem::transmute(func_ptr);
        println!("Result: {}", add_func(2, 3));

        // Direct assembly compilation
        let asm_options = AssemblyOptions {
            link: true,
            link_options: LinkOptions {
                libraries: vec!["c".to_string()],
                library_paths: vec![],
                static_link: false,
                strip_symbols: false,
            },
            target_architecture: Some(Architecture::X86_64),
        };

        let asm_code = r#"
            .text
            .global add
            .type add, @function
        add:
            add %edi, %esi
            mov %esi, %eax
            ret
        "#;

        compiler.compile_assembly(asm_code, "asm_output", &asm_options)?;

        Ok(())
    }
}
*/
