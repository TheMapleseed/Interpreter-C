pub struct BitPreciseInts {
    // Type information
    type_registry: HashMap<u32, BitIntType>,
    
    // Operations
    arithmetic_ops: BitIntArithmetic,
    bitwise_ops: BitIntBitwise,
    comparison_ops: BitIntComparison,
    
    // Range checking
    range_checker: RangeChecker,
}

impl BitPreciseInts {
    pub fn create_type(&mut self, bits: u32) -> Result<BitIntType, TypeError> {
        // Validate bit count
        if bits == 0 || bits > self.max_supported_bits() {
            return Err(TypeError::InvalidBitCount(bits));
        }
        
        // Create new type
        let bit_int_type = BitIntType {
            bits,
            signed: true,
            alignment: self.calculate_alignment(bits),
            max_value: self.calculate_max_value(bits),
            min_value: self.calculate_min_value(bits),
        };
        
        // Register type
        self.type_registry.insert(bits, bit_int_type.clone());
        
        Ok(bit_int_type)
    }

    pub fn perform_operation(
        &self,
        op: BitIntOp,
        lhs: &BitIntValue,
        rhs: &BitIntValue
    ) -> Result<BitIntValue, BitIntError> {
        // Validate operands
        self.validate_operands(op, lhs, rhs)?;
        
        // Perform operation
        match op {
            BitIntOp::Add => self.arithmetic_ops.add(lhs, rhs),
            BitIntOp::Sub => self.arithmetic_ops.sub(lhs, rhs),
            BitIntOp::Mul => self.arithmetic_ops.mul(lhs, rhs),
            BitIntOp::Div => self.arithmetic_ops.div(lhs, rhs),
            BitIntOp::And => self.bitwise_ops.and(lhs, rhs),
            BitIntOp::Or => self.bitwise_ops.or(lhs, rhs),
            BitIntOp::Xor => self.bitwise_ops.xor(lhs, rhs),
            BitIntOp::Shl => self.bitwise_ops.shl(lhs, rhs),
            BitIntOp::Shr => self.bitwise_ops.shr(lhs, rhs),
        }
    }
} 
