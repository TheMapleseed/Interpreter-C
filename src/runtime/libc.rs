pub struct CStandardLibrary {
    // Standard C library functions
    functions: HashMap<String, NativeFunction>,
    
    // stdio support
    file_handles: HashMap<i32, File>,
    
    // malloc/free support
    memory_allocator: CMemoryAllocator,
    
    // Other libc features
    environment: Environment,
    signal_handlers: SignalHandlers,
} 
