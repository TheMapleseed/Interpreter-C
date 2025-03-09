pub struct ProjectManager {
    // Single manager for all project types
    build_system: BuildSystem,
    runtime: RuntimeEnvironment,
    editor: Editor,
}

impl ProjectManager {
    pub async fn open_project(&mut self, path: &Path) -> Result<(), ProjectError> {
        // Single entry point for all projects
        self.editor.open(path)?;
        let build_output = self.build_system.build_project(path).await?;
        self.runtime.run_project(build_output).await
    }

    pub async fn create_project(&mut self, template: ProjectTemplate) -> Result<(), ProjectError> {
        // Simplified project creation
        self.editor.create_from_template(template)?;
        Ok(())
    }
} 
