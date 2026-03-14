pub(crate) fn create_project(name: &str) -> Result<Project, String> {
    // Implement project creation logic
    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    // Placeholder for project creation logic
    Ok(Project::new(name))
}