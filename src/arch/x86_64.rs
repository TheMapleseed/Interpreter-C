// src/arch/x86_64.rs
//! x86_64 (AMD64) architecture support
//! Provides comprehensive support for x86_64 assembly, including parsing,
//! code generation, and optimization for the AMD64 architecture.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use lazy_static::lazy_static;

use crate::arch::{
    Architecture, ArchitectureSupport, AssemblyParser, ABIHandler,
    InstructionEncoder, FeatureDetector, AssemblyParseError, EncodingError,
    Register, RegisterClass, Operand, MemoryOperand, Instruction,
    AssemblyBlock, AssemblyAST, CallingConvention, StructLayout, CPUFeatures,
};

/// Create x86_64 architecture support
pub fn create_support() -> ArchitectureSupport {
    ArchitectureSupport {
        architecture: Architecture::X86_64,
        asm_parser: Box::new(X86_64AssemblyParser::new()),
        abi_handler: Box::new(X86_64ABIHandler::new()),
        instruction_encoder: Box::new(X86_64InstructionEncoder::new()),
        feature_detector: Box::new(X86_64FeatureDetector::new()),
    }
}

/// x86_64 assembly parser
pub struct X86_64AssemblyParser {
    // Map of register names to registers
    registers: HashMap<String, Register>,
    // Map of instruction mnemonics to their handlers
    instruction_handlers: HashMap<String, InstructionHandler>,
}

type InstructionHandler = fn(&str, &[&str]) -> Result<Instruction, AssemblyParseError>;

impl X86_64AssemblyParser {
    /// Create a new x86_64 assembly parser
    pub fn new() -> Self {
        let mut parser = Self {
            registers: HashMap::new(),
            instruction_handlers: HashMap::new(),
        };
        
        parser.setup_registers();
        parser.setup_instruction_handlers();
        
        parser
    }
    
    /// Set up register definitions
    fn setup_registers(&mut self) {
        // General purpose registers (64-bit)
        let gp_regs = [
            ("rax", 0), ("rbx", 3), ("rcx", 1), ("rdx", 2),
            ("rsi", 6), ("rdi", 7), ("rbp", 5), ("rsp", 4),
            ("r8", 8), ("r9", 9), ("r10", 10), ("r11", 11),
            ("r12", 12), ("r13", 13), ("r14", 14), ("r15", 15),
        ];
        
        for (name, number) in gp_regs.iter() {
            self.registers.insert(name.to_string(), Register {
                name: name.to_string(),
                size: 64,
                number: *number,
                class: RegisterClass::General,
            });
        }
        
        // General purpose registers (32-bit)
        let gp_regs_32 = [
            ("eax", 0), ("ebx", 3), ("ecx", 1), ("edx", 2),
            ("esi", 6), ("edi", 7), ("ebp", 5), ("esp", 4),
            ("r8d", 8), ("r9d", 9), ("r10d", 10), ("r11d", 11),
            ("r12d", 12), ("r13d", 13), ("r14d", 14), ("r15d", 15),
        ];
        
        for (name, number) in gp_regs_32.iter() {
            self.registers.insert(name.to_string(), Register {
                name: name.to_string(),
                size: 32,
                number: *number,
                class: RegisterClass::General,
            });
        }
        
        // General purpose registers (16-bit)
        let gp_regs_16 = [
            ("ax", 0), ("bx", 3), ("cx", 1), ("dx", 2),
            ("si", 6), ("di", 7), ("bp", 5), ("sp", 4),
            ("r8w", 8), ("r9w", 9), ("r10w", 10), ("r11w", 11),
            ("r12w", 12), ("r13w", 13), ("r14w", 14), ("r15w", 15),
        ];
        
        for (name, number) in gp_regs_16.iter() {
            self.registers.insert(name.to_string(), Register {
                name: name.to_string(),
                size: 16,
                number: *number,
                class: RegisterClass::General,
            });
        }
        
        // General purpose registers (8-bit)
        let gp_regs_8 = [
            ("al", 0), ("bl", 3), ("cl", 1), ("dl", 2),
            ("sil", 6), ("dil", 7), ("bpl", 5), ("spl", 4),
            ("r8b", 8), ("r9b", 9), ("r10b", 10), ("r11b", 11),
            ("r12b", 12), ("r13b", 13), ("r14b", 14), ("r15b", 15),
            // Legacy high byte registers
            ("ah", 4), ("bh", 7), ("ch", 5), ("dh", 6),
        ];
        
        for (name, number) in gp_regs_8.iter() {
            self.registers.insert(name.to_string(), Register {
                name: name.to_string(),
                size: 8,
                number: *number,
                class: RegisterClass::General,
            });
        }
        
        // XMM registers
        for i in 0..32 {
            let name = format!("xmm{}", i);
            self.registers.insert(name.clone(), Register {
                name,
                size: 128,
                number: i,
                class: RegisterClass::Vector,
            });
        }
        
        // YMM registers
        for i in 0..32 {
            let name = format!("ymm{}", i);
            self.registers.insert(name.clone(), Register {
                name,
                size: 256,
                number: i,
                class: RegisterClass::Vector,
            });
        }
        
        // ZMM registers
        for i in 0..32 {
            let name = format!("zmm{}", i);
            self.registers.insert(name.clone(), Register {
                name,
                size: 512,
                number: i,
                class: RegisterClass::Vector,
            });
        }
        
        // Control registers
        for i in 0..16 {
            let name = format!("cr{}", i);
            self.registers.insert(name.clone(), Register {
                name,
                size: 64,
                number: i,
                class: RegisterClass::Special,
            });
        }
        
        // Debug registers
        for i in 0..16 {
            let name = format!("dr{}", i);
            self.registers.insert(name.clone(), Register {
                name,
                size: 64,
                number: i,
                class: RegisterClass::Special,
            });
        }
    }
    
    /// Set up instruction handlers
    fn setup_instruction_handlers(&mut self) {
        // Register instruction handlers for x86_64
        self.instruction_handlers.insert("mov".to_string(), Self::handle_mov);
        self.instruction_handlers.insert("add".to_string(), Self::handle_add);
        self.instruction_handlers.insert("sub".to_string(), Self::handle_sub);
        self.instruction_handlers.insert("and".to_string(), Self::handle_and);
        self.instruction_handlers.insert("or".to_string(), Self::handle_or);
        self.instruction_handlers.insert("xor".to_string(), Self::handle_xor);
        self.instruction_handlers.insert("cmp".to_string(), Self::handle_cmp);
        self.instruction_handlers.insert("test".to_string(), Self::handle_test);
        self.instruction_handlers.insert("imul".to_string(), Self::handle_imul);
        self.instruction_handlers.insert("idiv".to_string(), Self::handle_idiv);
        self.instruction_handlers.insert("inc".to_string(), Self::handle_inc);
        self.instruction_handlers.insert("dec".to_string(), Self::handle_dec);
        self.instruction_handlers.insert("neg".to_string(), Self::handle_neg);
        self.instruction_handlers.insert("not".to_string(), Self::handle_not);
        self.instruction_handlers.insert("lea".to_string(), Self::handle_lea);
        self.instruction_handlers.insert("push".to_string(), Self::handle_push);
        self.instruction_handlers.insert("pop".to_string(), Self::handle_pop);
        self.instruction_handlers.insert("jmp".to_string(), Self::handle_jmp);
        self.instruction_handlers.insert("je".to_string(), Self::handle_je);
        self.instruction_handlers.insert("jne".to_string(), Self::handle_jne);
        self.instruction_handlers.insert("jl".to_string(), Self::handle_jl);
        self.instruction_handlers.insert("jle".to_string(), Self::handle_jle);
        self.instruction_handlers.insert("jg".to_string(), Self::handle_jg);
        self.instruction_handlers.insert("jge".to_string(), Self::handle_jge);
        self.instruction_handlers.insert("call".to_string(), Self::handle_call);
        self.instruction_handlers.insert("ret".to_string(), Self::handle_ret);
        self.instruction_handlers.insert("syscall".to_string(), Self::handle_syscall);
        
        // SIMD instructions
        self.instruction_handlers.insert("movaps".to_string(), Self::handle_movaps);
        self.instruction_handlers.insert("movups".to_string(), Self::handle_movups);
        self.instruction_handlers.insert("movapd".to_string(), Self::handle_movapd);
        self.instruction_handlers.insert("movupd".to_string(), Self::handle_movupd);
        self.instruction_handlers.insert("addps".to_string(), Self::handle_addps);
        self.instruction_handlers.insert("addpd".to_string(), Self::handle_addpd);
        self.instruction_handlers.insert("subps".to_string(), Self::handle_subps);
        self.instruction_handlers.insert("subpd".to_string(), Self::handle_subpd);
        self.instruction_handlers.insert("mulps".to_string(), Self::handle_mulps);
        self.instruction_handlers.insert("mulpd".to_string(), Self::handle_mulpd);
        self.instruction_handlers.insert("divps".to_string(), Self::handle_divps);
        self.instruction_handlers.insert("divpd".to_string(), Self::handle_divpd);
    }
    
    // Handler functions for instructions
    fn handle_mov(_mnemonic: &str, operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        // Check operand count
        if operands.len() != 2 {
            return Err(AssemblyParseError::SyntaxError(
                format!("MOV instruction requires 2 operands, got {}", operands.len())
            ));
        }
        
        // We're not actually encoding the instruction here, just creating the representation
        let instruction = Instruction {
            mnemonic: "mov".to_string(),
            operands: Vec::new(), // Will be filled in by the parser
            prefixes: Vec::new(),
            suffixes: Vec::new(),
        };
        
        Ok(instruction)
    }
    
    fn handle_add(_mnemonic: &str, operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        // Check operand count
        if operands.len() != 2 {
            return Err(AssemblyParseError::SyntaxError(
                format!("ADD instruction requires 2 operands, got {}", operands.len())
            ));
        }
        
        // We're not actually encoding the instruction here, just creating the representation
        let instruction = Instruction {
            mnemonic: "add".to_string(),
            operands: Vec::new(), // Will be filled in by the parser
            prefixes: Vec::new(),
            suffixes: Vec::new(),
        };
        
        Ok(instruction)
    }
    
    // Other handler functions would be implemented here
    fn handle_sub(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_and(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_or(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_xor(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_cmp(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_test(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_imul(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_idiv(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_inc(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_dec(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_neg(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_not(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_lea(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_push(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_pop(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_jmp(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_je(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_jne(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_jl(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_jle(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_jg(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_jge(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_call(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_ret(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_syscall(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_movaps(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_movups(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_movapd(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_movupd(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_addps(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_addpd(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_subps(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_subpd(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_mulps(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_mulpd(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_divps(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
    
    fn handle_divpd(_mnemonic: &str, _operands: &[&str]) -> Result<Instruction, AssemblyParseError> {
        unimplemented!()
    }
}

impl AssemblyParser for X86_64AssemblyParser {
    fn parse(&self, code: &str) -> Result<AssemblyAST, AssemblyParseError> {
        let mut blocks = Vec::new();
        let mut current_block = AssemblyBlock {
            instructions: Vec::new(),
            labels: Vec::new(),
            comments: Vec::new(),
        };
        
        let mut global_directives = Vec::new();
        
        // Process each line
        for (line_num, line) in code.lines().enumerate() {
            let line_num = line_num + 1; // 1-indexed line numbers for errors
            let line = line.trim();
            
            // Skip empty lines
            if line.is_empty() {
                continue;
            }
            
            // Handle comments
            if line.starts_with(';') || line.starts_with('#') {
                current_block.comments.push(line.to_string());
                continue;
            }
            
            // Check for inline comments
            let code_part = if let Some(comment_idx) = line.find(';') {
                let (code, comment) = line.split_at(comment_idx);
                current_block.comments.push(comment.to_string());
                code.trim()
            } else {
                line
            };
            
            if code_part.is_empty() {
                continue;
            }
            
            // Handle directives
            if code_part.starts_with('.') {
                global_directives.push(code_part.to_string());
                continue;
            }
            
            // Handle labels
            if code_part.ends_with(':') {
                let label = code_part[..code_part.len() - 1].trim().to_string();
                current_block.labels.push(label);
                continue;
            }
            
            // Parse instruction
            let mut parts = code_part.split_whitespace();
            let mnemonic = match parts.next() {
                Some(m) => m.to_lowercase(),
                None => continue, // Skip line if no mnemonic
            };
            
            // Check if mnemonic is supported
            if !self.is_mnemonic_supported(&mnemonic) {
                return Err(AssemblyParseError::UnknownMnemonic(
                    format!("Unknown mnemonic '{}' at line {}", mnemonic, line_num)
                ));
            }
            
            // Parse operands
            let operands_str = parts.collect::<Vec<_>>();
            
            // Use the appropriate instruction handler
            let handler = self.instruction_handlers.get(&mnemonic).unwrap();
            let instruction = handler(&mnemonic, &operands_str)
                .map_err(|e| match e {
                    AssemblyParseError::SyntaxError(msg) => 
                        AssemblyParseError::SyntaxError(format!("{} at line {}", msg, line_num)),
                    AssemblyParseError::InvalidOperand(msg) => 
                        AssemblyParseError::InvalidOperand(format!("{} at line {}", msg, line_num)),
                    _ => e,
                })?;
            
            current_block.instructions.push(instruction);
        }
        
        // Add the final block if it has content
        if !current_block.instructions.is_empty() || !current_block.labels.is_empty() {
            blocks.push(current_block);
        }
        
        Ok(AssemblyAST {
            blocks,
            directives: global_directives,
        })
    }
    
    fn is_mnemonic_supported(&self, mnemonic: &str) -> bool {
        self.instruction_handlers.contains_key(&mnemonic.to_lowercase())
    }
    
    fn parse_register(&self, reg_name: &str) -> Option<Register> {
        self.registers.get(&reg_name.to_lowercase()).cloned()
    }
    
    fn parse_operand(&self, operand: &str) -> Result<Operand, AssemblyParseError> {
        let operand = operand.trim();
        
        // Empty operand
        if operand.is_empty() {
            return Err(AssemblyParseError::InvalidOperand(
                "Empty operand".to_string()
            ));
        }
        
        // Register operand
        if let Some(reg) = self.parse_register(operand) {
            return Ok(Operand::Register(reg));
        }
        
        // Immediate operand (decimal, hex, octal, binary)
        if operand.starts_with('$') || operand.starts_with('#') ||
           (operand.starts_with('-') && operand[1..].chars().all(|c| c.is_digit(10))) ||
           operand.chars().all(|c| c.is_digit(10)) {
            
            let value_str = if operand.starts_with('$') || operand.starts_with('#') {
                &operand[1..]
            } else {
                operand
            };
            
            let value = if value_str.starts_with("0x") || value_str.starts_with("0X") {
                // Hexadecimal
                i64::from_str_radix(&value_str[2..], 16)
            } else if value_str.starts_with("0b") || value_str.starts_with("0B") {
                // Binary
                i64::from_str_radix(&value_str[2..], 2)
            } else if value_str.starts_with('0') && value_str.len() > 1 {
                // Octal
                i64::from_str_radix(&value_str[1..], 8)
            } else {
                // Decimal
                value_str.parse::<i64>()
            };
            
            match value {
                Ok(v) => return Ok(Operand::Immediate(v)),
                Err(_) => return Err(AssemblyParseError::InvalidOperand(
                    format!("Invalid immediate value: {}", operand)
                )),
            }
        }
        
        // Memory operand
        if operand.contains('[') && operand.ends_with(']') {
            return self.parse_memory_operand(operand);
        }
        
        // Label/symbol reference
        if operand.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return Ok(Operand::Label(operand.to_string()));
        }
        
        Err(AssemblyParseError::InvalidOperand(
            format!("Unrecognized operand format: {}", operand)
        ))
    }
}

fn parse_memory_operand(&self, operand: &str) -> Result<Operand, AssemblyParseError> {
    // Parse x86_64 memory operand syntax:
    // [base + index*scale + displacement]
    
    // Extract the part inside brackets
    let start = operand.find('[').unwrap();
    let end = operand.rfind(']').unwrap();
    let inner = &operand[start+1..end].trim();
    
    let mut base = None;
    let mut index = None;
    let mut scale: u8 = 1;
    let mut displacement = 0;
    let mut pc_relative = false;
    
    // Split by + and -
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut positive = true;
    
    for c in inner.chars() {
        if c == '+' {
            if !current.is_empty() {
                parts.push((positive, current.trim().to_string()));
                current = String::new();
            }
            positive = true;
        } else if c == '-' {
            if !current.is_empty() {
                parts.push((positive, current.trim().to_string()));
                current = String::new();
            }
            positive = false;
        } else {
            current.push(c);
        }
    }
    
    if !current.is_empty() {
        parts.push((positive, current.trim().to_string()));
    }
    
    // Process each part
    for (is_positive, part) in parts {
        if part.contains('*') {
            // This is an index*scale component
            let index_scale: Vec<&str> = part.split('*').collect();
            if index_scale.len() != 2 {
                return Err(AssemblyParseError::InvalidAddressingMode(
                    format!("Invalid index*scale format: {}", part)
                ));
            }
            
            let idx_reg = index_scale[0].trim();
            if let Some(reg) = self.parse_register(idx_reg) {
                index = Some(reg);
            } else {
                return Err(AssemblyParseError::InvalidRegister(
                    format!("Invalid index register: {}", idx_reg)
                ));
            }
            
            // Parse scale
            match index_scale[1].trim() {
                "1" => scale = 1,
                "2" => scale = 2,
                "4" => scale = 4,
                "8" => scale = 8,
                _ => return Err(AssemblyParseError::InvalidAddressingMode(
                    format!("Invalid scale factor: {}", index_scale[1])
                )),
            }
        } else if let Some(reg) = self.parse_register(&part) {
            // This is a register (base or index)
            if base.is_none() {
                base = Some(reg);
            } else if index.is_none() {
                index = Some(reg);
            } else {
                return Err(AssemblyParseError::InvalidAddressingMode(
                    format!("Too many registers in memory operand: {}", operand)
                ));
            }
        } else if part.starts_with("rip") {
            // RIP-relative addressing
            pc_relative = true;
        } else {
            // This is a displacement
            let disp_val = match part.parse::<i64>() {
                Ok(v) => v,
                Err(_) => {
                    // Might be a symbolic reference
                    if part.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                        0 // Will be resolved by the linker
                    } else {
                        return Err(AssemblyParseError::InvalidAddressingMode(
                            format!("Invalid displacement: {}", part)
                        ));
                    }
                }
            };
            
            displacement += if is_positive { disp_val } else { -disp_val };
        }
    }
    
    Ok(Operand::Memory(MemoryOperand {
        base,
        index,
        scale,
        displacement,
        pc_relative,
    }))
}

/// x86_64 ABI handler
pub struct X86_64ABIHandler {
    // System V ABI calling convention (default for Unix-like systems)
    system_v_cc: CallingConvention,
    // Microsoft x64 calling convention (Windows)
    ms_x64_cc: CallingConvention,
    // Current calling convention
    current_cc: CallingConvention,
    // Cache for struct layouts
    struct_layout_cache: Arc<RwLock<HashMap<String, StructLayout>>>,
}

impl X86_64ABIHandler {
    /// Create a new x86_64 ABI handler
    pub fn new() -> Self {
        let system_v_cc = Self::create_system_v_calling_convention();
        let ms_x64_cc = Self::create_ms_x64_calling_convention();
        
        Self {
            system_v_cc: system_v_cc.clone(),
            ms_x64_cc,
            current_cc: system_v_cc,
            struct_layout_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create System V calling convention (Linux, macOS, *BSD)
    fn create_system_v_calling_convention() -> CallingConvention {
        // System V AMD64 ABI calling convention
        let mut param_regs = Vec::new();
        let mut return_regs = Vec::new();
        let mut caller_saved = Vec::new();
        let mut callee_saved = Vec::new();
        
        // Integer parameter registers
        let int_param_regs = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
        for (i, &name) in int_param_regs.iter().enumerate() {
            param_regs.push(Register {
                name: name.to_string(),
                size: 64,
                number: i,
                class: RegisterClass::General,
            });
        }
        
        // Floating-point parameter registers
        for i in 0..8 {
            let name = format!("xmm{}", i);
            param_regs.push(Register {
                name,
                size: 128,
                number: i,
                class: RegisterClass::Float,
            });
        }
        
        // Return registers
        return_regs.push(Register {
            name: "rax".to_string(),
            size: 64,
            number: 0,
            class: RegisterClass::General,
        });
        
        return_regs.push(Register {
            name: "rdx".to_string(),
            size: 64,
            number: 2,
            class: RegisterClass::General,
        });
        
        return_regs.push(Register {
            name: "xmm0".to_string(),
            size: 128,
            number: 0,
            class: RegisterClass::Float,
        });
        
        return_regs.push(Register {
            name: "xmm1".to_string(),
            size: 128,
            number: 1,
            class: RegisterClass::Float,
        });
        
        // Caller-saved registers
        let caller_saved_names = [
            "rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"
        ];
        
        for (i, &name) in caller_saved_names.iter().enumerate() {
            caller_saved.push(Register {
                name: name.to_string(),
                size: 64,
                number: i,
                class: RegisterClass::General,
            });
        }
        
        // All XMM registers are caller-saved
        for i in 0..16 {
            let name = format!("xmm{}", i);
            caller_saved.push(Register {
                name,
                size: 128,
                number: i,
                class: RegisterClass::Float,
            });
        }
        
        // Callee-saved registers
        let callee_saved_names = ["rbx", "rsp", "rbp", "r12", "r13", "r14", "r15"];
        
        for (i, &name) in callee_saved_names.iter().enumerate() {
            callee_saved.push(Register {
                name: name.to_string(),
                size: 64,
                number: i,
                class: RegisterClass::General,
            });
        }
        
        CallingConvention {
            name: "System V AMD64 ABI".to_string(),
            parameter_registers: param_regs,
            return_registers: return_regs,
            caller_saved,
            callee_saved,
            stack_parameters: true,
            stack_alignment: 16,
            red_zone_size: 128,
        }
    }
    
    /// Create Microsoft x64 calling convention (Windows)
    fn create_ms_x64_calling_convention() -> CallingConvention {
        // Microsoft x64 calling convention
        let mut param_regs = Vec::new();
        let mut return_regs = Vec::new();
        let mut caller_saved = Vec::new();
        let mut callee_saved = Vec::new();
        
        // Integer parameter registers
        let int_param_regs = ["rcx", "rdx", "r8", "r9"];
        for (i, &name) in int_param_regs.iter().enumerate() {
            param_regs.push(Register {
                name: name.to_string(),
                size: 64,
                number: i,
                class: RegisterClass::General,
            });
        }
        
        // Floating-point parameter registers
        for i in 0..4 {
            let name = format!("xmm{}", i);
            param_regs.push(Register {
                name,
                size: 128,
                number: i,
                class: RegisterClass::Float,
            });
        }
        
        // Return registers
        return_regs.push(Register {
            name: "rax".to_string(),
            size: 64,
            number: 0,
            class: RegisterClass::General,
        });
        
        return_regs.push(Register {
            name: "xmm0".to_string(),
            size: 128,
            number: 0,
            class: RegisterClass::Float,
        });
        
        // Caller-saved registers
        let caller_saved_names = [
            "rax", "rcx", "rdx", "r8", "r9", "r10", "r11"
        ];
        
        for (i, &name) in caller_saved_names.iter().enumerate() {
            caller_saved.push(Register {
                name: name.to_string(),
                size: 64,
                number: i,
                class: RegisterClass::General,
            });
        }
        
        // All XMM registers are caller-saved
        for i in 0..16 {
            let name = format!("xmm{}", i);
            caller_saved.push(Register {
                name,
                size: 128,
                number: i,
                class: RegisterClass::Float,
            });
        }
        
        // Callee-saved registers
        let callee_saved_names = ["rbx", "rsp", "rbp", "rsi", "rdi", "r12", "r13", "r14", "r15"];
        
        for (i, &name) in callee_saved_names.iter().enumerate() {
            callee_saved.push(Register {
                name: name.to_string(),
                size: 64,
                number: i,
                class: RegisterClass::General,
            });
        }
        
        CallingConvention {
            name: "Microsoft x64".to_string(),
            parameter_registers: param_regs,
            return_registers: return_regs,
            caller_saved,
            callee_saved,
            stack_parameters: true,
            stack_alignment: 16,
            red_zone_size: 0, // Microsoft x64 does not have a red zone
        }
    }
    
    /// Switch to Microsoft x64 calling convention (Windows)
    pub fn use_ms_x64_convention(&mut self) {
        self.current_cc = self.ms_x64_cc.clone();
    }
    
    /// Switch to System V calling convention (Linux, macOS, *BSD)
    pub fn use_system_v_convention(&mut self) {
        self.current_cc = self.system_v_cc.clone();
    }
}

impl ABIHandler for X86_64ABIHandler {
    fn calling_convention(&self) -> &CallingConvention {
        &self.current_cc
    }
    
    fn layout_struct(&self, structure: &StructType) -> StructLayout {
        // Check cache first
        {
            let cache = self.struct_layout_cache.read();
            if let Some(layout) = cache.get(&structure.name) {
                return layout.clone();
            }
        }
        
        // Calculate struct layout according to platform ABI rules
        let is_ms_abi = self.current_cc.name == "Microsoft x64";
        
        let mut size = 0;
        let mut alignment = 1;
        let mut field_offsets = Vec::new();
        
        for field in &structure.fields {
            // Calculate field alignment
            let field_align = field.alignment;
            
            // Update struct alignment to the largest field alignment
            alignment = alignment.max(field_align);
            
            // Align the current size to field alignment
            size = (size + field_align - 1) & !(field_align - 1);
            
            // Record the field offset
            field_offsets.push(size);
            
            // Add the field size
            size += field.size;
            
            // Microsoft ABI has special handling for bitfields and certain types,
            // but we'll ignore that complexity for this implementation
        }
        
        // Round the final size up to the alignment
        size = (size + alignment - 1) & !(alignment - 1);
        
        // In Microsoft x64 ABI, structures are always 8-byte aligned at minimum
        if is_ms_abi {
            alignment = alignment.max(8);
            size = (size + 7) & !7;
        }
        
        let layout = StructLayout {
            size,
            alignment,
            field_offsets,
        };
        
        // Cache the result
        {
            let mut cache = self.struct_layout_cache.write();
            cache.insert(structure.name.clone(), layout.clone());
        }
        
        layout
    }
    
    fn parameter_registers(&self) -> &[Register] {
        &self.current_cc.parameter_registers
    }
    
    fn return_registers(&self) -> &[Register] {
        &self.current_cc.return_registers
    }
}

/// x86_64 instruction encoder
pub struct X86_64InstructionEncoder {
    // Encoder tables
    encoding_tables: Arc<EncodingTables>,
}

struct EncodingTables {
    // Tables for instruction encoding
    // Implementation omitted for brevity
}

impl X86_64InstructionEncoder {
    /// Create a new x86_64 instruction encoder
    pub fn new() -> Self {
        Self {
            encoding_tables: Arc::new(EncodingTables::new()),
        }
    }
}

impl EncodingTables {
    /// Create new encoding tables
    fn new() -> Self {
        Self {}
    }
    
    /// Get REX prefix for 64-bit operation
    fn get_rex_prefix(&self, w: bool, r: bool, x: bool, b: bool) -> u8 {
        0x40 | (w as u8) << 3 | (r as u8) << 2 | (x as u8) << 1 | (b as u8)
    }
    
    /// Get ModR/M byte
    fn get_modrm(&self, mod_val: u8, reg: u8, rm: u8) -> u8 {
        (mod_val & 0x3) << 6 | (reg & 0x7) << 3 | (rm & 0x7)
    }
    
    /// Get SIB byte
    fn get_sib(&self, scale: u8, index: u8, base: u8) -> u8 {
        let scale_bits = match scale {
            1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            _ => 0, // Default to scale factor of 1
        };
        
        (scale_bits & 0x3) << 6 | (index & 0x7) << 3 | (base & 0x7)
    }
}

impl InstructionEncoder for X86_64InstructionEncoder {
    fn encode_instruction(&self, instruction: &Instruction) -> Result<Vec<u8>, EncodingError> {
        // This is a simplified encoder that handles only basic instructions
        // A full implementation would handle all x86_64 instructions with their encoding variants
        
        let mut encoded = Vec::new();
        
        match instruction.mnemonic.as_str() {
            "mov" => {
                if instruction.operands.len() != 2 {
                    return Err(EncodingError::InvalidInstruction(
                        "MOV requires 2 operands".to_string()
                    ));
                }
                
                match (&instruction.operands[0], &instruction.operands[1]) {
                    (Operand::Register(dst), Operand::Register(src)) => {
                        // MOV r64, r64
                        if dst.size == 64 && src.size == 64 {
                            // REX.W prefix for 64-bit operation
                            encoded.push(self.encoding_tables.get_rex_prefix(
                                true,  // W=1 for 64-bit
                                (src.number & 0x8) != 0,  // R bit
                                false, // X bit
                                (dst.number & 0x8) != 0   // B bit
                            ));
                            
                            // Opcode for MOV between registers
                            encoded.push(0x89);
                            
                            // ModR/M byte
                            encoded.push(self.encoding_tables.get_modrm(
                                0b11,  // Mod=11 for register direct
                                (src.number & 0x7) as u8,
                                (dst.number & 0x7) as u8
                            ));
                        } else {
                            // Handle other sizes
                            return Err(EncodingError::UnsupportedFeature(
                                "Register size combination not supported".to_string()
                            ));
                        }
                    },
                    (Operand::Register(dst), Operand::Immediate(imm)) => {
                        // MOV r64, imm64
                        if dst.size == 64 {
                            // REX.W prefix for 64-bit operation
                            encoded.push(self.encoding_tables.get_rex_prefix(
                                true,  // W=1 for 64-bit
                                false, // R bit
                                false, // X bit
                                (dst.number & 0x8) != 0  // B bit
                            ));
                            
                            // Opcode for MOV immediate to register
                            encoded.push(0xB8 + (dst.number & 0x7) as u8);
                            
                            // Immediate value (64-bit)
                            let imm_bytes = (*imm as u64).to_le_bytes();
                            encoded.extend_from_slice(&imm_bytes);
                        } else {
                            // Handle other sizes
                            return Err(EncodingError::UnsupportedFeature(
                                "Register size not supported".to_string()
                            ));
                        }
                    },
                    (Operand::Register(dst), Operand::Memory(mem)) => {
                        // MOV r64, [mem]
                        if dst.size == 64 {
                            // REX.W prefix for 64-bit operation
                            encoded.push(self.encoding_tables.get_rex_prefix(
                                true,  // W=1 for 64-bit
                                (dst.number & 0x8) != 0,  // R bit
                                mem.index.as_ref().map_or(false, |r| (r.number & 0x8) != 0),  // X bit
                                mem.base.as_ref().map_or(false, |r| (r.number & 0x8) != 0)    // B bit
                            ));
                            
                            // Opcode for MOV from memory to register
                            encoded.push(0x8B);
                            
                            // Encode the memory operand (simplified)
                            self.encode_memory_operand(&mut encoded, dst.number as u8, mem)?;
                        } else {
                            // Handle other sizes
                            return Err(EncodingError::UnsupportedFeature(
                                "Register size not supported".to_string()
                            ));
                        }
                    },
                    (Operand::Memory(mem), Operand::Register(src)) => {
                        // MOV [mem], r64
                        if src.size == 64 {
                            // REX.W prefix for 64-bit operation
                            encoded.push(self.encoding_tables.get_rex_prefix(
                                true,  // W=1 for 64-bit
                                (src.number & 0x8) != 0,  // R bit
                                mem.index.as_ref().map_or(false, |r| (r.number & 0x8) != 0),  // X bit
                                mem.base.as_ref().map_or(false, |r| (r.number & 0x8) != 0)    // B bit
                            ));
                            
                            // Opcode for MOV from register to memory
                            encoded.push(0x89);
                            
                            // Encode the memory operand (simplified)
                            self.encode_memory_operand(&mut encoded, src.number as u8, mem)?;
                        } else {
                            // Handle other sizes
                            return Err(EncodingError::UnsupportedFeature(
                                "Register size not supported".to_string()
                            ));
                        }
                    },
                    _ => {
                        return Err(EncodingError::InvalidOperand(
                            "Unsupported operand combination for MOV".to_string()
                        ));
                    }
                }
            },
            "add" => {
                if instruction.operands.len() != 2 {
                    return Err(EncodingError::InvalidInstruction(
                        "ADD requires 2 operands".to_string()
                    ));
                }
                
                match (&instruction.operands[0], &instruction.operands[1]) {
                    (Operand::Register(dst), Operand::Register(src)) => {
                        // ADD r64, r64
                        if dst.size == 64 && src.size == 64 {
                            // REX.W prefix for 64-bit operation
                            encoded.push(self.encoding_tables.get_rex_prefix(
                                true,  // W=1 for 64-bit
                                (src.number & 0x8) != 0,  // R bit
                                false, // X bit
                                (dst.number & 0x8) != 0   // B bit
                            ));
                            
                            // Opcode for ADD between registers
                            encoded.push(0x01);
                            
                            // ModR/M byte
                            encoded.push(self.encoding_tables.get_modrm(
                                0b11,  // Mod=11 for register direct
                                (src.number & 0x7) as u8,
                                (dst.number & 0x7) as u8
                            ));
                        } else {
                            // Handle other sizes
                            return Err(EncodingError::UnsupportedFeature(
                                "Register size combination not supported".to_string()
                            ));
                        }
                    },
                    // More ADD variants would be implemented here
                    _ => {
                        return Err(EncodingError::InvalidOperand(
                            "Unsupported operand combination for ADD".to_string()
                        ));
                    }
                }
            },
            // More instructions would be implemented here
            _ => {
                return Err(EncodingError::InvalidInstruction(
                    format!("Instruction {} not implemented", instruction.mnemonic)
                ));
            }
        }
        
        Ok(encoded)
    }
    
    fn encode_asm_block(&self, block: &AssemblyBlock) -> Result<Vec<u8>, EncodingError> {
        let mut encoded = Vec::new();
        
        // This is a simplified implementation that doesn't handle labels and jumps correctly
        // A full implementation would need to resolve labels and calculate jump offsets
        
        for instruction in &block.instructions {
            let inst_bytes = self.encode_instruction(instruction)?;
            encoded.extend_from_slice(&inst_bytes);
        }
        
        Ok(encoded)
    }
    
    fn instruction_size(&self, instruction: &Instruction) -> usize {
        // For simplicity, we'll estimate sizes very approximately
        // A full implementation would calculate exact instruction sizes
        
        match instruction.mnemonic.as_str() {
            // Typically 2-3 bytes for register-register ops, 3-7 for immediate/memory ops
            "mov" | "add" | "sub" | "and" | "or" | "xor" | "cmp" | "test" => {
                match instruction.operands.as_slice() {
                    [Operand::Register(_), Operand::Register(_)] => 3,
                    [Operand::Register(_), Operand::Immediate(_)] => 5,
                    [Operand::Register(_), Operand::Memory(_)] |
                    [Operand::Memory(_), Operand::Register(_)] => 5,
                    _ => 3, // Default estimate
                }
            },
            // Jump instructions
            "jmp" | "je" | "jne" | "jl" | "jle" | "jg" | "jge" => {
                match instruction.operands.as_slice() {
                    [Operand::Label(_)] => 5, // Typically 5 bytes for near jumps
                    _ => 2, // Short jumps
                }
            },
            // Call instruction
            "call" => 5, // Typically 5 bytes
            // String instructions usually 1-3 bytes
            "movs" | "cmps" | "stos" | "lods" | "scas" => 3,
            // Default for other instructions
            _ => 3,
        }
    }
    
    // Helper method to encode memory operands
    fn encode_memory_operand(&self, encoded: &mut Vec<u8>, reg: u8, mem: &MemoryOperand) -> Result<(), EncodingError> {
        // This is a simplified implementation that doesn't handle all addressing modes
        
        if let Some(base) = &mem.base {
            let base_reg = (base.number & 0x7) as u8;
            
            if let Some(index) = &mem.index {
                // [base + index*scale + disp]
                let index_reg = (index.number & 0x7) as u8;
                
                // Always use SIB byte when there's an index register
                
                if mem.displacement == 0 {
                    // [base + index*scale]
                    encoded.push(self.encoding_tables.get_modrm(
                        0b00,    // Mod=00
                        reg & 0x7, // Reg field
                        0b100    // R/M=4 (SIB)
                    ));
                    
                    encoded.push(self.encoding_tables.get_sib(
                        mem.scale,
                        index_reg,
                        base_reg
                    ));
                } else if mem.displacement >= -128 && mem.displacement <= 127 {
                    // [base + index*scale + disp8]
                    encoded.push(self.encoding_tables.get_modrm(
                        0b01,    // Mod=01
                        reg & 0x7, // Reg field
                        0b100    // R/M=4 (SIB)
                    ));
                    
                    encoded.push(self.encoding_tables.get_sib(
                        mem.scale,
                        index_reg,
                        base_reg
                    ));
                    
                    encoded.push(mem.displacement as u8);
                } else {
                    // [base + index*scale + disp32]
                    encoded.push(self.encoding_tables.get_modrm(
                        0b10,    // Mod=10
                        reg & 0x7, // Reg field
                        0b100    // R/M=4 (SIB)
                    ));
                    
                    encoded.push(self.encoding_tables.get_sib(
                        mem.scale,
                        index_reg,
                        base_reg
                    ));
                    
                    // Displacement as 32-bit immediate
                    let disp_bytes = (mem.displacement as i32).to_le_bytes();
                    encoded.extend_from_slice(&disp_bytes);
                }
            } else {
                // [base + disp]
                
                if base_reg == 0b100 {
                    // Special case: need SIB byte when base is RSP/ESP (R/M=4)
                    
                    if mem.displacement == 0 {
                        // [rsp]
                        encoded.push(self.encoding_tables.get_modrm(
                            0b00,    // Mod=00
                            reg & 0x7, // Reg field
                            0b100    // R/M=4 (SIB)
                        ));
                        
                        encoded.push(self.encoding_tables.get_sib(
                            1,      // Scale=1
                            0b100,  // Index=4 (no index)
                            0b100   // Base=RSP
                        ));
                    } else if mem.displacement >= -128 && mem.displacement <= 127 {
                        // [rsp + disp8]
                        encoded.push(self.encoding_tables.get_modrm(
                            0b01,    // Mod=01
                            reg & 0x7, // Reg field
                            0b100    // R/M=4 (SIB)
                        ));
                        
                        encoded.push(self.encoding_tables.get_sib(
                            1,      // Scale=1
                            0b100,  // Index=4 (no index)
                            0b100   // Base=RSP
                        ));
                        
                        encoded.push(mem.displacement as u8);
                    } else {
                        // [rsp + disp32]
                        encoded.push(self.encoding_tables.get_modrm(
                            0b10,    // Mod=10
                            reg & 0x7, // Reg field
                            0b100    // R/M=4 (SIB)
                        ));
                        
                        encoded.push(self.encoding_tables.get_sib(
                            1,      // Scale=1
                            0b100,  // Index=4 (no index)
                            0b100   // Base=RSP
                        ));
                        
                        // Displacement as 32-bit immediate
                        let disp_bytes = (mem.displacement as i32).to_le_bytes();
                        encoded.extend_from_slice(&disp_bytes);
                    }
                } else {
                    // Normal case: no SIB needed
                    
                    if mem.displacement == 0 && base_reg != 0b101 {
                        // [reg]
                        encoded.push(self.encoding_tables.get_modrm(
                            0b00,    // Mod=00
                            reg & 0x7, // Reg field
                            base_reg // R/M field
                        ));
                    } else if mem.displacement >= -128 && mem.displacement <= 127 {
                        // [reg + disp8]
                        encoded.push(self.encoding_tables.get_modrm(
                            0b01,    // Mod=01
                            reg & 0x7, // Reg field
                            base_reg // R/M field
                        ));
                        
                        encoded.push(mem.displacement as u8);
                    } else {
                        // [reg + disp32]
                        encoded.push(self.encoding_tables.get_modrm(
                            0b10,    // Mod=10
                            reg & 0x7, // Reg field
                            base_reg // R/M field
                        ));
                        
                        // Displacement as 32-bit immediate
                        let disp_bytes = (mem.displacement as i32).to_le_bytes();
                        encoded.extend_from_slice(&disp_bytes);
                    }
                }
            }
        } else {
            // [disp] or RIP-relative addressing
            
            if mem.pc_relative {
                // [RIP + disp32] - RIP relative addressing
                encoded.push(self.encoding_tables.get_modrm(
                    0b00,    // Mod=00
                    reg & 0x7, // Reg field
                    0b101    // R/M=5 (RIP-relative)
                ));
                
                // Displacement as 32-bit immediate
                let disp_bytes = (mem.displacement as i32).to_le_bytes();
                encoded.extend_from_slice(&disp_bytes);
            } else {
                // [disp32] - Absolute addressing
                encoded.push(self.encoding_tables.get_modrm(
                    0b00,    // Mod=00
                    reg & 0x7, // Reg field
                    0b100    // R/M=4 (SIB)
                ));
                
                encoded.push(self.encoding_tables.get_sib(
                    1,      // Scale=1
                    0b100,  // Index=4 (no index)
                    0b101   // Base=5 (no base, disp32 only)
                ));
                
                // Displacement as 32-bit immediate
                let disp_bytes = (mem.displacement as i32).to_le_bytes();
                encoded.extend_from_slice(&disp_bytes);
            }
        }
        
        Ok(())
    }
}

/// x86_64 feature detector
pub struct X86_64FeatureDetector {
    // CPU features
    features: CPUFeatures,
}

impl X86_64FeatureDetector {
    /// Create a new x86_64 feature detector
    pub fn new() -> Self {
        Self {
            features: Self::detect_cpu_features(),
        }
    }
    
    /// Detect CPU features
    fn detect_cpu_features() -> CPUFeatures {
        // In a real implementation, we would use CPUID to detect features
        // For this simplified version, we'll just return a set of commonly supported features
        
        let mut extensions = Vec::new();
        let mut features = Vec::new();
        
        // Add common extensions
        extensions.push("sse".to_string());
        extensions.push("sse2".to_string());
        extensions.push("sse3".to_string());
        extensions.push("ssse3".to_string());
        extensions.push("sse4.1".to_string());
        extensions.push("sse4.2".to_string());
        extensions.push("avx".to_string());
        extensions.push("avx2".to_string());
        extensions.push("fma".to_string());
        extensions.push("bmi1".to_string());
        extensions.push("bmi2".to_string());
        extensions.push("aes".to_string());
        extensions.push("pclmulqdq".to_string());
        
        // Add common features
        features.push("mmx".to_string());
        features.push("x87".to_string());
        features.push("cx8".to_string());
        features.push("cmov".to_string());
        features.push("popcnt".to_string());
        features.push("cx16".to_string());
        features.push("movbe".to_string());
        features.push("rdrand".to_string());
        
        CPUFeatures {
            architecture: Architecture::X86_64,
            extensions,
            vector_width: 32, // 256-bit (AVX2)
            cache_line_size: 64, // Common cache line size
            features,
        }
    }
    
    /// Detect if AVX-512 is supported
    fn has_avx512() -> bool {
        // In a real implementation, we would use CPUID to check for AVX-512 support
        // For this simplified version, we'll just return false
        false
    }
    
    /// Get optimization flags for various instruction set extensions
    fn get_optimization_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        
        // Base flags
        flags.push("-march=x86-64".to_string());
        
        // Add flags for detected extensions
        if self.has_feature("avx2") {
            flags.push("-mavx2".to_string());
        }
        
        if self.has_feature("sse4.2") {
            flags.push("-msse4.2".to_string());
        }
        
        if self.has_feature("fma") {
            flags.push("-mfma".to_string());
        }
        
        if self.has_feature("bmi2") {
            flags.push("-mbmi2".to_string());
        }
        
        flags
    }
}

impl FeatureDetector for X86_64FeatureDetector {
    fn detect_features(&self) -> CPUFeatures {
        self.features.clone()
    }
    
    fn has_feature(&self, feature: &str) -> bool {
        self.features.extensions.iter().any(|f| f == feature) || 
        self.features.features.iter().any(|f| f == feature)
    }
    
    fn optimization_flags(&self) -> Vec<String> {
        self.get_optimization_flags()
    }
}

// This struct is referenced but not defined in the module interfaces
pub struct StructType {
    pub name: String,
    pub fields: Vec<StructField>,
    pub attributes: Vec<String>,
}

pub struct StructField {
    pub name: String,
    pub ty: String,
    pub size: usize,
    pub alignment: usize,
} 