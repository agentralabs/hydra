#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::notifications::push::*;
    use crate::notifications::push_providers::*;

    fn make_device(name: &str) -> RegisteredDevice {
        RegisteredDevice {
            name: name.to_string(),
            provider_type: "ntfy".to_string(),
            push_token: format!("token-{}", name),
            last_seen: Utc::now(),
            urgency_filter: vec![],
        }
    }

    fn make_device_with_filter(name: &str, filter: Vec<&str>) -> RegisteredDevice {
        RegisteredDevice {
            name: name.to_string(),
            provider_type: "telegram".to_string(),
            push_token: format!("token-{}", name),
            last_seen: Utc::now(),
            urgency_filter: filter.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_add_device() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        assert_eq!(reg.devices.len(), 1);
        assert_eq!(reg.devices[0].name, "phone");
    }

    #[test]
    fn test_add_device_replaces_existing() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        let mut updated = make_device("phone");
        updated.push_token = "new-token".to_string();
        reg.add_device(updated);
        assert_eq!(reg.devices.len(), 1);
        assert_eq!(reg.devices[0].push_token, "new-token");
    }

    #[test]
    fn test_remove_device() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        reg.add_device(make_device("tablet"));
        assert!(reg.remove_device("phone"));
        assert_eq!(reg.devices.len(), 1);
        assert_eq!(reg.devices[0].name, "tablet");
    }

    #[test]
    fn test_remove_nonexistent_device() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        assert!(!reg.remove_device("laptop"));
        assert_eq!(reg.devices.len(), 1);
    }

    #[test]
    fn test_get_device() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        assert!(reg.get_device("phone").is_some());
        assert!(reg.get_device("tablet").is_none());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("devices.json");

        let mut reg = DeviceRegistry::with_path(path.clone());
        reg.add_device(make_device("phone"));
        reg.add_device(make_device("tablet"));
        reg.save().unwrap();

        let loaded = DeviceRegistry::load(path).unwrap();
        assert_eq!(loaded.devices.len(), 2);
        assert_eq!(loaded.devices[0].name, "phone");
        assert_eq!(loaded.devices[1].name, "tablet");
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        let reg = DeviceRegistry::load(path).unwrap();
        assert!(reg.devices.is_empty());
    }

    #[test]
    fn test_devices_for_urgency_no_filter() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device("phone"));
        // Empty filter means accept all urgencies
        let matching = reg.devices_for_urgency("high");
        assert_eq!(matching.len(), 1);
    }

    #[test]
    fn test_devices_for_urgency_with_filter() {
        let mut reg = DeviceRegistry::new();
        reg.add_device(make_device_with_filter("phone", vec!["high", "normal"]));
        reg.add_device(make_device_with_filter("tablet", vec!["high"]));

        let high = reg.devices_for_urgency("high");
        assert_eq!(high.len(), 2);

        let normal = reg.devices_for_urgency("normal");
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0].name, "phone");

        let low = reg.devices_for_urgency("low");
        assert!(low.is_empty());
    }

    #[test]
    fn test_push_message_serialization() {
        let msg = PushMessage {
            title: "Test".to_string(),
            body: "Hello".to_string(),
            urgency: "normal".to_string(),
            action_url: Some("https://example.com".to_string()),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["title"], "Test");
        assert_eq!(json["action_url"], "https://example.com");
    }

    #[test]
    fn test_registered_device_serialization() {
        let dev = make_device("phone");
        let json = serde_json::to_string(&dev).unwrap();
        let restored: RegisteredDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "phone");
        assert_eq!(restored.provider_type, "ntfy");
    }

    #[test]
    fn test_ntfy_priority_mapping() {
        assert_eq!(urgency_to_ntfy_priority("high"), "urgent");
        assert_eq!(urgency_to_ntfy_priority("urgent"), "urgent");
        assert_eq!(urgency_to_ntfy_priority("low"), "low");
        assert_eq!(urgency_to_ntfy_priority("normal"), "default");
        assert_eq!(urgency_to_ntfy_priority("unknown"), "default");
    }

    #[test]
    fn test_list_devices() {
        let mut reg = DeviceRegistry::new();
        assert!(reg.list_devices().is_empty());
        reg.add_device(make_device("a"));
        reg.add_device(make_device("b"));
        assert_eq!(reg.list_devices().len(), 2);
    }

    #[test]
    fn test_email_provider_construction() {
        let provider = EmailProvider::new(
            "smtp.gmail.com".to_string(),
            "hydra@example.com".to_string(),
            "user@example.com".to_string(),
        );
        assert_eq!(provider.provider_name(), "email");
        assert_eq!(provider.smtp_host, "smtp.gmail.com");
        assert_eq!(provider.smtp_port, 587);
        assert_eq!(provider.username, "hydra@example.com");

        let provider2 = EmailProvider::with_credentials(
            "smtp.outlook.com".to_string(), 587,
            "user@outlook.com".to_string(), "app-password".to_string(),
            "user@outlook.com".to_string(), "dest@example.com".to_string(),
        );
        assert_eq!(provider2.smtp_port, 587);
        assert_eq!(provider2.password, "app-password");
    }

    #[test]
    fn test_default_registry() {
        let reg = DeviceRegistry::default();
        assert!(reg.devices.is_empty());
    }
}
