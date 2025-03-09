pub struct ProjectRunner {
    // Build system
    builder: ProjectBuilder,
    
    // Runtime environment
    runtime: CRuntimeEnvironment,
    
    // Debug support
    debugger: Debugger,
    
    // Output handling
    output_handler: OutputHandler,
}

impl ProjectRunner {
    pub async fn run_project(&mut self, project: &Project) -> Result<RunResult, RunError> {
        // Build project
        let build_result = self.builder.build_project(project).await?;
        
        // Initialize runtime
        self.runtime.initialize_for_project(project).await?;
        
        // Setup debugging if needed
        if project.config.debug_mode {
            self.debugger.attach_to_runtime(&mut self.runtime).await?;
        }
        
        // Run project
        let result = self.runtime.execute(build_result).await?;
        
        // Handle output
        self.output_handler.process_output(result.output)?;
        
        Ok(result)
    }

    pub async fn debug_project(&mut self, project: &Project) -> Result<DebugSession, DebugError> {
        // Build with debug info
        let debug_build = self.builder.build_with_debug_info(project).await?;
        
        // Initialize debug session
        let session = self.debugger.create_session(debug_build).await?;
        
        Ok(session)
    }
} 
