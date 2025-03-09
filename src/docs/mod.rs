pub struct DocumentationGenerator {
    // Documentation configuration
    config: DocConfig,
    
    // Content generators
    api_docs: ApiDocGenerator,
    test_docs: TestDocGenerator,
    usage_docs: UsageDocGenerator,
    
    // Output formats
    markdown_generator: MarkdownGenerator,
    html_generator: HtmlGenerator,
    pdf_generator: PdfGenerator,
}

impl DocumentationGenerator {
    pub async fn generate_all_docs(&mut self) -> Result<(), DocError> {
        // Generate API documentation
        self.generate_api_docs().await?;
        
        // Generate test documentation
        self.generate_test_docs().await?;
        
        // Generate usage documentation
        self.generate_usage_docs().await?;
        
        Ok(())
    }
} 
