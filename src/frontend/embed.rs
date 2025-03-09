use std::path::Path;
use std::fs;
use memmap2::Mmap;

pub struct EmbedHandler {
    // Resource tracking
    resource_map: HashMap<String, ResourceInfo>,
    size_limits: ResourceLimits,
    
    // Memory mapping
    mmap_handler: MmapHandler,
    
    // Resource validation
    validator: ResourceValidator,
    
    // Preprocessing
    preprocessor: EmbedPreprocessor,
}

impl EmbedHandler {
    pub fn handle_embed_directive(
        &mut self,
        directive: &EmbedDirective
    ) -> Result<Vec<u8>, EmbedError> {
        // Validate resource path
        let path = self.validate_resource_path(&directive.path)?;
        
        // Check size limits
        self.check_resource_limits(&path)?;
        
        // Memory map the resource
        let mapped_resource = self.mmap_handler.map_resource(&path)?;
        
        // Process resource data
        let processed_data = self.preprocessor.process_resource(
            &mapped_resource,
            directive.options
        )?;
        
        Ok(processed_data)
    }

    fn validate_resource_path(&self, path: &Path) -> Result<PathBuf, EmbedError> {
        // Security checks
        self.validator.check_path_security(path)?;
        
        // Resolve relative paths
        let absolute_path = self.resolver.resolve_path(path)?;
        
        // Check file exists and is readable
        if !absolute_path.exists() {
            return Err(EmbedError::ResourceNotFound(path.to_path_buf()));
        }
        
        Ok(absolute_path)
    }
} 
