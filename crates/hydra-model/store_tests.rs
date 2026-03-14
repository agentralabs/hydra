#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_get_user_preferences() {
        let result = get_user_preferences("user123");
        assert!(result.is_ok());
        // Further assertions based on expected preferences
    }

    #[test]
    fn test_get_user_preferences_empty_id() {
        let result = get_user_preferences("");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "User ID cannot be empty");
    }
}