#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_project() {
        let result = create_project("New Project");
        assert!(result.is_ok());
        // Further assertions based on expected project
    }

    #[test]
    fn test_create_project_empty_name() {
        let result = create_project("");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "Project name cannot be empty");
    }
}