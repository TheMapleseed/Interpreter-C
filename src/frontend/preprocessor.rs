pub struct CPreprocessor {
    // Macro system
    macro_table: HashMap<String, Macro>,
    macro_expansion_stack: Vec<MacroExpansion>,
    
    // Include system
    include_paths: Vec<PathBuf>,
    system_includes: Vec<PathBuf>,
    included_files: HashSet<PathBuf>,
    
    // Conditional compilation
    if_stack: Vec<IfStackEntry>,
    defined_symbols: HashSet<String>,
    
    // Built-in macros
    compiler_macros: HashMap<String, String>,
    
    // Pragma handling
    pragma_handlers: HashMap<String, Box<dyn PragmaHandler>>,
}

pub trait PreprocessorBase {
    fn handle_includes(&mut self) -> Result<(), PreprocessorError>;
    fn handle_macros(&mut self) -> Result<(), PreprocessorError>;
}

impl CPreprocessor {
    fn handle_includes(&mut self) -> Result<(), PreprocessorError> {
        // Single implementation of include handling
        for path in &self.include_paths {
            self.process_include(path)?;
        }
        Ok(())
    }
    
    fn expand_macros(&mut self) -> Result<(), PreprocessorError> {
        // Full macro expansion
        // Function-like macros
        // Variadic macros
        // Stringification
        // Token pasting
    }
}

// Update C23 preprocessor to use inheritance
impl C23Preprocessor {
    fn handle_includes(&mut self) -> Result<(), PreprocessorError> {
        // First handle base includes
        self.base_preprocessor.handle_includes()?;
        
        // Then handle C23-specific includes
        self.handle_c23_specific_includes()?;
        Ok(())
    }
} 
