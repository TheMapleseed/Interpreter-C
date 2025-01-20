// src/jit/encoder.rs
use std::collections::HashMap;

pub struct InstructionEncoder {
    // Encoding tables
    rex_prefix_table: HashMap<Register, u8>,
    opcode_table: HashMap<Opcode, Vec<u8>>,
    mod_rm_table: HashMap<(Register, Register), u8>,
    
    // Encoding state
    needs_rex_w: bool,
    needs_rex_r: bool,
    needs_rex_x: bool,
    needs_rex_b: bool,
}

impl InstructionEncoder {
    pub fn new() -> Self {
        let mut encoder = InstructionEncoder {
            rex_prefix_table: HashMap::new(),
            opcode_table: HashMap::new(),
            mod_rm_table: HashMap::new(),
            needs_rex_w: false,
            needs_rex_r: false,
            needs_rex_x: false,
            needs_rex_b: false,
        };
        
        encoder.initialize_tables();
        encoder
    }

    pub fn encode_instruction(
        &mut self,
        inst: &Instruction,
        buffer: &mut CodeBuffer
    ) -> Result<(), JITError> {
        // Reset state
        self.reset_state();
        
        match inst {
            Instruction::Move(dst, src) => self.encode_mov(dst, src, buffer)?,
            Instruction::Add(dst, src) => self.encode_add(dst, src, buffer)?,
            Instruction::Sub(dst, src) => self.encode_sub(dst, src, buffer)?,
            Instruction::Mul(dst, src) => self.encode_mul(dst, src, buffer)?,
            Instruction::Div(dst) => self.encode_div(dst, buffer)?,
            Instruction::Push(src) => self.encode_push(src, buffer)?,
            Instruction::Pop(dst) => self.encode_pop(dst, buffer)?,
            Instruction::Call(target) => self.encode_call(target, buffer)?,
            Instruction::Ret => self.encode_ret(buffer)?,
            Instruction::Jump(target) => self.encode_jmp(target, buffer)?,
            Instruction::ConditionalJump(cond, target) => self.encode_jcc(cond, target, buffer)?,
            // Add more instructions as needed...
        }
        
        Ok(())
    }

    fn encode_mov(
        &mut self,
        dst: &Operand,
        src: &Operand,
        buffer: &mut CodeBuffer
    ) -> Result<(), JITError> {
        match (dst, src) {
            (Operand::Register(dst_reg), Operand::Register(src_reg)) => {
                // Register to register move
                self.needs_rex_w = true;  // For 64-bit operands
                
                // Encode REX prefix if needed
                self.encode_rex_prefix(buffer)?;
                
                // Encode MOV opcode (0x89 for r/m64, r64)
                buffer.emit_bytes(&[0x89]);
                
                // Encode ModR/M byte
                self.encode_mod_rm(*dst_reg, *src_reg, buffer)?;
            },
            
            (Operand::Register(reg), Operand::Immediate(imm)) => {
                // Immediate to register move
                self.needs_rex_w = true;
                
                // Encode REX prefix
                self.encode_rex_prefix(buffer)?;
                
                // Encode MOV opcode (0xB8 + register code)
                let opcode = 0xB8 + self.get_register_code(*reg);
                buffer.emit_bytes(&[opcode]);
                
                // Encode immediate value
                buffer.emit_u64(*imm);
            },
            
            (Operand::Memory(addr), Operand::Register(reg)) => {
                // Register to memory move
                self.needs_rex_w = true;
                
                // Encode REX prefix
                self.encode_rex_prefix(buffer)?;
                
                // Encode MOV opcode (0x89 for memory store)
                buffer.emit_bytes(&[0x89]);
                
                // Encode ModR/M and SIB bytes for memory addressing
                self.encode_memory_operand(addr, *reg, buffer)?;
            },
            
            (Operand::Register(reg), Operand::Memory(addr)) => {
                // Memory to register move
                self.needs_rex_w = true;
                
                // Encode REX prefix
                self.encode_rex_prefix(buffer)?;
                
                // Encode MOV opcode (0x8B for memory load)
                buffer.emit_bytes(&[0x8B]);
                
                // Encode ModR/M and SIB bytes
                self.encode_memory_operand(addr, *reg, buffer)?;
            },
            
            _ => return Err(JITError::InvalidOperandCombination),
        }
        
        Ok(())
    }

    fn encode_add(
        &mut self,
        dst: &Operand,
        src: &Operand,
        buffer: &mut CodeBuffer
    ) -> Result<(), JITError> {
        match (dst, src) {
            (Operand::Register(dst_reg), Operand::Register(src_reg)) => {
                self.needs_rex_w = true;
                
                // Encode REX prefix
                self.encode_rex_prefix(buffer)?;
                
                // Encode ADD opcode (0x01 for r/m64, r64)
                buffer.emit_bytes(&[0x01]);
                
                // Encode ModR/M byte
                self.encode_mod_rm(*dst_reg, *src_reg, buffer)?;
            },
            
            (Operand::Register(reg), Operand::Immediate(imm)) => {
                self.needs_rex_w = true;
                
                // Encode REX prefix
                self.encode_rex_prefix(buffer)?;
                
                // Encode ADD opcode (0x81 /0 for immediate to register)
                buffer.emit_bytes(&[0x81]);
                
                // Encode ModR/M byte with /0 extension
                self.encode_mod_rm_with_ext(*reg, 0, buffer)?;
                
                // Encode immediate value
                buffer.emit_u32(*imm as u32);
            },
            
            _ => return Err(JITError::InvalidOperandCombination),
        }
        
        Ok(())
    }

    fn encode_call(
        &mut self,
        target: &CallTarget,
        buffer: &mut CodeBuffer
    ) -> Result<(), JITError> {
        match target {
            CallTarget::Direct(offset) => {
                // Encode CALL opcode (0xE8)
                buffer.emit_bytes(&[0xE8]);
                
                // Encode 32-bit relative offset
                buffer.emit_u32(*offset as u32);
            },
            
            CallTarget::Indirect(reg) => {
                // Encode REX prefix if needed
                self.encode_rex_prefix(buffer)?;
                
                // Encode CALL opcode (0xFF /2)
                buffer.emit_bytes(&[0xFF]);
                
                // Encode ModR/M byte with /2 extension
                self.encode_mod_rm_with_ext(*reg, 2, buffer)?;
            },
        }
        
        Ok(())
    }

    fn encode_ret(&mut self, buffer: &mut CodeBuffer) -> Result<(), JITError> {
        // Simple RET instruction (0xC3)
        buffer.emit_bytes(&[0xC3]);
        Ok(())
    }

    fn encode_rex_prefix(&self, buffer: &mut CodeBuffer) -> Result<(), JITError> {
        if self.needs_rex_w || self.needs_rex_r || self.needs_rex_x || self.needs_rex_b {
            let mut rex = 0x40;
            if self.needs_rex_w { rex |= 0x08; }
            if self.needs_rex_r { rex |= 0x04; }
            if self.needs_rex_x { rex |= 0x02; }
            if self.needs_rex_b { rex |= 0x01; }
            buffer.emit_bytes(&[rex]);
        }
        Ok(())
    }

    fn encode_mod_rm(
        &self,
        rm: Register,
        reg: Register,
        buffer: &mut CodeBuffer
    ) -> Result<(), JITError> {
        let mod_bits = 0b11; // Register direct addressing
        let reg_bits = self.get_register_code(reg) & 0x7;
        let rm_bits = self.get_register_code(rm) & 0x7;
        let mod_rm = (mod_bits << 6) | (reg_bits << 3) | rm_bits;
        buffer.emit_bytes(&[mod_rm]);
        Ok(())
    }

    fn encode_memory_operand(
        &self,
        addr: &MemoryAddress,
        reg: Register,
        buffer: &mut CodeBuffer
    ) -> Result<(), JITError> {
        match addr {
            MemoryAddress::BaseDisp(base, disp) => {
                let mod_bits = if *disp == 0 {
                    0b00
                } else if *disp >= -128 && *disp <= 127 {
                    0b01
                } else {
                    0b10
                };
                
                let reg_bits = self.get_register_code(reg) & 0x7;
                let rm_bits = self.get_register_code(*base) & 0x7;
                let mod_rm = (mod_bits << 6) | (reg_bits << 3) | rm_bits;
                
                buffer.emit_bytes(&[mod_rm]);
                
                // Emit displacement if needed
                if mod_bits == 0b01 {
                    buffer.emit_i8(*disp as i8);
                } else if mod_bits == 0b10 {
                    buffer.emit_i32(*disp);
                }
            },
            
            MemoryAddress::SIB { base, index, scale, disp } => {
                // Handle SIB byte encoding...
                todo!("Implement SIB encoding");
            },
        }
        
        Ok(())
    }

    fn get_register_code(&self, reg: Register) -> u8 {
        match reg {
            Register::RAX => 0,
            Register::RCX => 1,
            Register::RDX => 2,
            Register::RBX => 3,
            Register::RSP => 4,
            Register::RBP => 5,
            Register::RSI => 6,
            Register::RDI => 7,
            Register::R8  => 8,
            Register::R9  => 9,
            Register::R10 => 10,
            Register::R11 => 11,
            Register::R12 => 12,
            Register::R13 => 13,
            Register::R14 => 14,
            Register::R15 => 15,
        }
    }

    fn reset_state(&mut self) {
        self.needs_rex_w = false;
        self.needs_rex_r = false;
        self.needs_rex_x = false;
        self.needs_rex_b = false;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Register {
    RAX, RCX, RDX, RBX,
    RSP, RBP, RSI, RDI,
    R8, R9, R10, R11,
    R12, R13, R14, R15,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Register(Register),
    Memory(MemoryAddress),
    Immediate(i64),
}

#[derive(Debug, Clone)]
pub enum MemoryAddress {
    BaseDisp(Register, i32),
    SIB {
        base: Register,
        index: Register,
        scale: u8,
        disp: i32,
    },
}

#[derive(Debug, Clone)]
pub enum CallTarget {
    Direct(i32),  // Relative offset
    Indirect(Register),  // Register containing address
}

#[derive(Debug)]
pub enum JITError {
    InvalidOperandCombination,
    InvalidMemoryAddress,
    BufferOverflow,
    // Add more error types...
}

// Example usage:
/*
fn main() -> Result<(), JITError> {
    let mut encoder = InstructionEncoder::new();
    let mut buffer = CodeBuffer::new();

    // Encode: mov rax, 42
    encoder.encode_instruction(
        &Instruction::Move(
            Operand::Register(Register::RAX),
            Operand::Immediate(42)
        ),
        &mut buffer
    )?;

    // Encode: add rax, rcx
    encoder.encode_instruction(
        &Instruction::Add(
            Operand::Register(Register::RAX),
            Operand::Register(Register::RCX)
        ),
        &mut buffer
    )?;

    // Encode: ret
    encoder.encode_instruction(
        &Instruction::Ret,
        &mut buffer
    )?;

    Ok(())
}
*/
