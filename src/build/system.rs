pub struct BuildSystem {
    preprocessor: Preprocessor,
    compiler: Compiler,
    cache: BuildCache,
}

impl BuildSystem {
    pub async fn build_project(&mut self, path: &Path) -> Result<BuildOutput, BuildError> {
        // Check cache first
        if let Some(output) = self.cache.get_for_path(path)? {
            return Ok(output);
        }

        // Process all source files
        let source_files = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "c"));

        let mut compiled_objects = Vec::new();
        for source in source_files {
            // Preprocess
            let preprocessed = self.preprocessor.process_file(&source.path())?;
            
            // Compile
            let object = self.compiler.compile(preprocessed)?;
            compiled_objects.push(object);
        }

        // Link objects into final output
        let output = self.compiler.link(compiled_objects)?;
        
        // Cache the result
        self.cache.store(path, &output)?;
        
        Ok(output)
    }
}

// Simplified compiler that handles everything internally
struct Compiler {
    target: Target,
    optimization_level: OptLevel,
}

impl Compiler {
    fn compile(&self, source: PreprocessedSource) -> Result<CompiledObject, BuildError> {
        // Direct compilation using our own compiler
        // No external build system needed
        self.compile_to_object(source)
    }

    fn link(&self, objects: Vec<CompiledObject>) -> Result<BuildOutput, BuildError> {
        // Direct linking using our own linker
        self.link_objects(objects)
    }
} 
