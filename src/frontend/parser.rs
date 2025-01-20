pub struct CParser {
    // Add support for full C parsing
    preprocessor: CPreprocessor,
    type_system: CTypeSystem,
    scope_manager: ScopeManager,
    
    // Track C-specific constructs
    struct_definitions: HashMap<String, StructDefinition>,
    typedef_map: HashMap<String, CType>,
    enum_definitions: HashMap<String, EnumDefinition>,
} 
