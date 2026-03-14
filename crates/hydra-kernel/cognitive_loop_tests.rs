#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_decision() {
        let result = make_decision("test input");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Decision made based on: test input");
    }

    #[test]
    fn test_make_decision_empty_input() {
        let result = make_decision("");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "Input cannot be empty");
    }
}