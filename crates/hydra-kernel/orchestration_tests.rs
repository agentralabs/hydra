#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrate_tasks() {
        let tasks = vec![Task::new()]; // Assuming Task::new() is valid
        let result = orchestrate_tasks(tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn test_orchestrate_tasks_empty() {
        let result = orchestrate_tasks(vec![]);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "Task list cannot be empty");
    }
}