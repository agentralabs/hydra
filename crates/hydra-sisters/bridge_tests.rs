use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_factory_configuration_valid() {
        let config = FactoryConfig { /* valid fields */ };
        assert!(validate_factory_configuration(&config).is_ok());
    }

    #[test]
    fn test_validate_factory_configuration_invalid() {
        let config = FactoryConfig { /* invalid fields */ };
        assert!(validate_factory_configuration(&config).is_err());
    }
}