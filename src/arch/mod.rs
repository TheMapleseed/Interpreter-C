// src/arch/mod.rs
//! Architecture-specific definitions and implementations
//! This module provides comprehensive support for various CPU architectures,
//! including assembly parsing, code generation, and optimization.

pub mod aarch64;  // ARM64/Apple Silicon
pub mod x86_64;   // AMD64
pub mod arm;      // ARM (32-bit)

use std::fmt;
use std::str::FromStr;

/// Supported CPU architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Architecture {
    /// x86-64 architecture (AMD64)
    X86_64,
    /// AArch64 architecture (ARM64, Apple Silicon)
    AArch64,
    /// ARM architecture (32-bit)
    Arm,
}

impl Architecture {
    /// Get the default target triple for this architecture
    pub fn default_target_triple(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64-unknown-linux-gnu",
            Architecture::AArch64 => "aarch64-unknown-linux-gnu",
            Architecture::Arm => "arm-unknown-linux-gnueabihf",
        }
    }
    
    /// Get Apple-specific target triple if applicable
    pub fn apple_target_triple(&self) -> Option<&'static str> {
        match self {
            Architecture::X86_64 => Some("x86_64-apple-darwin"),
            Architecture::AArch64 => Some("aarch64-apple-darwin"),
            Architecture::Arm => None, // Apple doesn't use 32-bit ARM anymore
        }
    }
    
    /// Check if this architecture is big endian by default
    pub fn is_big_endian(&self) -> bool {
        match self {
            Architecture::X86_64 => false,
            Architecture::AArch64 => false,
            Architecture::Arm => false, // ARM supports both but defaults to little-endian
        }
    }
    
    /// Get the native word size for this architecture
    pub fn word_size(&self) -> usize {
        match self {
            Architecture::X86_64 => 8,   // 64-bit
            Architecture::AArch64 => 8,  // 64-bit
            Architecture::Arm => 4,      // 32-bit
        }
    }
    
    /// Get the maximum vector width supported by this architecture
    pub fn max_vector_width(&self) -> usize {
        match self {
            Architecture::X86_64 => 64,   // 512-bit (AVX-512)
            Architecture::AArch64 => 16,  // 128-bit (NEON)
            Architecture::Arm => 16,      // 128-bit (NEON)
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Architecture::X86_64 => write!(f, "x86_64"),
            Architecture::AArch64 => write!(f, "aarch64"),
            Architecture::Arm => write!(f, "arm"),
        }
    }
}

impl FromStr for Architecture {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "x86_64" | "amd64" | "x64" => Ok(Architecture::X86_64),
            "aarch64" | "arm64" | "applesilicon" => Ok(Architecture::AArch64),
            "arm" | "armv7" => Ok(Architecture::Arm),
            _ => Err(format!("Unknown architecture: {}", s)),
        }
    }
}

/// Registry of supported architectures with their features
pub struct ArchitectureRegistry {
    architectures: Vec<ArchitectureSupport>,
}

impl ArchitectureRegistry {
    /// Create a new architecture registry with all supported architectures
    pub fn new() -> Self {
        let mut registry = Self {
            architectures: Vec::new(),
        };
        
        // Register x86_64 support
        registry.register(x86_64::create_support());
        
        // Register AArch64 support
        registry.register(aarch64::create_support());
        
        // Register ARM support
        registry.register(arm::create_support());
        
        registry
    }
    
    /// Register support for a specific architecture
    pub fn register(&mut self, support: ArchitectureSupport) {
        self.architectures.push(support);
    }
    
    /// Get support for a specific architecture
    pub fn get_support(&self, arch: Architecture) -> Option<&ArchitectureSupport> {
        self.architectures.iter().find(|s| s.architecture == arch)
    }
}

/// Support for a specific architecture
pub struct ArchitectureSupport {
    /// The architecture being supported
    pub architecture: Architecture,
    /// Assembly parser for this architecture
    pub asm_parser: Box<dyn AssemblyParser>,
    /// ABI handler for this architecture
    pub abi_handler: Box<dyn ABIHandler>,
    /// Instruction encoder for this architecture
    pub instruction_encoder: Box<dyn InstructionEncoder>,
    /// Feature detection for this architecture
    pub feature_detector: Box<dyn FeatureDetector>,
}

/// Trait for assembly parsers
pub trait AssemblyParser: Send + Sync {
    /// Parse assembly code into an AST
    fn parse(&self, code: &str) -> Result<AssemblyAST, AssemblyParseError>;
    
    /// Check if a given assembly mnemonic is supported
    fn is_mnemonic_supported(&self, mnemonic: &str) -> bool;
    
    /// Parse register references
    fn parse_register(&self, reg_name: &str) -> Option<Register>;
    
    /// Parse an operand
    fn parse_operand(&self, operand: &str) -> Result<Operand, AssemblyParseError>;
}

/// Trait for ABI handlers
pub trait ABIHandler: Send + Sync {
    /// Get the calling convention for this architecture
    fn calling_convention(&self) -> &CallingConvention;
    
    /// Handle struct layout
    fn layout_struct(&self, structure: &StructType) -> StructLayout;
    
    /// Get parameter registers
    fn parameter_registers(&self) -> &[Register];
    
    /// Get return registers
    fn return_registers(&self) -> &[Register];
}

/// Trait for instruction encoders
pub trait InstructionEncoder: Send + Sync {
    /// Encode an instruction into machine code
    fn encode_instruction(&self, instruction: &Instruction) -> Result<Vec<u8>, EncodingError>;
    
    /// Encode a full assembly block
    fn encode_asm_block(&self, block: &AssemblyBlock) -> Result<Vec<u8>, EncodingError>;
    
    /// Get the size of an encoded instruction
    fn instruction_size(&self, instruction: &Instruction) -> usize;
}

/// Trait for CPU feature detection
pub trait FeatureDetector: Send + Sync {
    /// Detect available CPU features for the architecture
    fn detect_features(&self) -> CPUFeatures;
    
    /// Check if a specific feature is available
    fn has_feature(&self, feature: &str) -> bool;
    
    /// Get architecture-specific optimization flags
    fn optimization_flags(&self) -> Vec<String>;
}

/// Error that can occur when parsing assembly
#[derive(Debug)]
pub enum AssemblyParseError {
    /// Syntax error in assembly code
    SyntaxError(String),
    /// Unknown mnemonic
    UnknownMnemonic(String),
    /// Invalid operand
    InvalidOperand(String),
    /// Invalid register
    InvalidRegister(String),
    /// Invalid addressing mode
    InvalidAddressingMode(String),
}

/// Error that can occur when encoding instructions
#[derive(Debug)]
pub enum EncodingError {
    /// Invalid instruction
    InvalidInstruction(String),
    /// Invalid operand
    InvalidOperand(String),
    /// Unsupported feature
    UnsupportedFeature(String),
    /// Operand out of range
    OperandOutOfRange(String),
}

/// Register in a CPU
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Register {
    /// Name of the register
    pub name: String,
    /// Size of the register in bits
    pub size: usize,
    /// Register number/ID
    pub number: usize,
    /// Register class (e.g. general, vector, floating point)
    pub class: RegisterClass,
}

/// Register classes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterClass {
    /// General purpose register
    General,
    /// Floating point register
    Float,
    /// Vector register
    Vector,
    /// Special/control register
    Special,
}

/// Assembly instruction operand
#[derive(Debug, Clone)]
pub enum Operand {
    /// Immediate value
    Immediate(i64),
    /// Register operand
    Register(Register),
    /// Memory operand
    Memory(MemoryOperand),
    /// Label reference
    Label(String),
}

/// Memory operand
#[derive(Debug, Clone)]
pub struct MemoryOperand {
    /// Base register
    pub base: Option<Register>,
    /// Index register
    pub index: Option<Register>,
    /// Scaling factor for index
    pub scale: u8,
    /// Displacement/offset
    pub displacement: i64,
    /// Whether this is a PC-relative reference
    pub pc_relative: bool,
}

/// Assembly instruction
#[derive(Debug, Clone)]
pub struct Instruction {
    /// Instruction mnemonic
    pub mnemonic: String,
    /// Instruction operands
    pub operands: Vec<Operand>,
    /// Instruction prefixes (architecture specific)
    pub prefixes: Vec<String>,
    /// Instruction suffixes (architecture specific)
    pub suffixes: Vec<String>,
}

/// Assembly block
#[derive(Debug, Clone)]
pub struct AssemblyBlock {
    /// Instructions in this block
    pub instructions: Vec<Instruction>,
    /// Labels in this block
    pub labels: Vec<String>,
    /// Comments in this block
    pub comments: Vec<String>,
}

/// Abstract syntax tree for assembly code
#[derive(Debug, Clone)]
pub struct AssemblyAST {
    /// Blocks in this AST
    pub blocks: Vec<AssemblyBlock>,
    /// Global directives
    pub directives: Vec<String>,
}

/// Calling convention
#[derive(Debug, Clone)]
pub struct CallingConvention {
    /// Name of the calling convention
    pub name: String,
    /// Parameter passing registers
    pub parameter_registers: Vec<Register>,
    /// Return value registers
    pub return_registers: Vec<Register>,
    /// Caller-saved registers
    pub caller_saved: Vec<Register>,
    /// Callee-saved registers
    pub callee_saved: Vec<Register>,
    /// Whether to use the stack for parameters
    pub stack_parameters: bool,
    /// Stack alignment in bytes
    pub stack_alignment: usize,
    /// Size of red zone in bytes (if supported)
    pub red_zone_size: usize,
}

/// Struct layout information
#[derive(Debug, Clone)]
pub struct StructLayout {
    /// Total size of the struct in bytes
    pub size: usize,
    /// Alignment of the struct in bytes
    pub alignment: usize,
    /// Offsets of fields in bytes
    pub field_offsets: Vec<usize>,
}

/// CPU features
#[derive(Debug, Clone)]
pub struct CPUFeatures {
    /// Architecture of this CPU
    pub architecture: Architecture,
    /// Available instruction set extensions
    pub extensions: Vec<String>,
    /// Vector unit width in bits
    pub vector_width: usize,
    /// Cache line size in bytes
    pub cache_line_size: usize,
    /// Available instruction set features
    pub features: Vec<String>,
}

/// Structure type for ABI layout
#[derive(Debug, Clone)]
pub struct StructType {
    /// Name of the struct
    pub name: String,
    /// Fields of the struct
    pub fields: Vec<StructField>,
    /// Struct-wide attributes
    pub attributes: Vec<String>,
}

/// Field in a structure
#[derive(Debug, Clone)]
pub struct StructField {
    /// Name of the field
    pub name: String,
    /// Type of the field
    pub ty: String,
    /// Size of the field in bytes
    pub size: usize,
    /// Alignment of the field in bytes
    pub alignment: usize,
} 