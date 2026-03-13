use super::*;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn integration_test_factory_validation() {
        let config = FactoryConfig { /* valid fields */ };
        assert!(validate_factory_configuration(&config).is_ok());

        let invalid_config = FactoryConfig { /* invalid fields */ };
        assert!(validate_factory_configuration(&invalid_config).is_err());
    }
}