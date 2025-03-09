pub struct CABIHandler {
    // Platform-specific ABI rules
    calling_convention: CallingConvention,
    
    // Struct layout rules
    struct_layout_cache: HashMap<StructType, StructLayout>,
    
    // Register allocation for C ABI
    parameter_registers: Vec<Register>,
    return_registers: Vec<Register>,
    
    // Stack frame management
    stack_alignment: usize,
    red_zone_size: usize,
} 
