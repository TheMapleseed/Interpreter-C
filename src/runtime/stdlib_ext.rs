pub struct StandardLibraryExtensions {
    // New C23 functions
    memccpy_impl: MemccpyImplementation,
    strdup_impl: StrdupImplementation,
    
    // Enhanced string functions
    string_functions: EnhancedStringFunctions,
    
    // Memory functions
    memory_functions: EnhancedMemoryFunctions,
    
    // Atomic operations
    atomic_operations: AtomicOperations,
}

impl StandardLibraryExtensions {
    pub fn initialize(&mut self) -> Result<(), StdlibError> {
        // Initialize all new functions
        self.setup_memory_functions()?;
        self.setup_string_functions()?;
        self.setup_atomic_operations()?;
        
        // Register with function table
        self.register_functions()?;
        
        Ok(())
    }

    fn setup_memory_functions(&mut self) -> Result<(), StdlibError> {
        // Use RAII pattern to prevent leaks
        struct BufferGuard<'a> {
            buffer: *mut u8,
            stdlib: &'a mut StandardLibraryExtensions,
        }

        impl<'a> Drop for BufferGuard<'a> {
            fn drop(&mut self) {
                unsafe {
                    self.stdlib.free_buffer(self.buffer);
                }
            }
        }

        let buf = self.allocate_buffer(size)?;
        let _guard = BufferGuard { buffer: buf, stdlib: self };

        if some_condition {
            return Ok(());  // Buffer will be freed by guard
        }

        Ok(()) // Buffer still freed by guard
    }

    fn setup_string_functions(&mut self) -> Result<(), StdlibError> {
        // strdup/strndup implementation
        self.strdup_impl = StrdupImplementation::new()?;
        
        // Enhanced string functions
        self.string_functions = EnhancedStringFunctions::new(
            self.unicode_support,
            self.locale_handler
        )?;
        
        Ok(())
    }
} 
