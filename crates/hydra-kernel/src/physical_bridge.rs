//! O43: Physical Bridge — control physical devices via HTTP/MQTT APIs.
//!
//! Generic device connector: any device with an HTTP API can be controlled.
//! Discovery: scan local network for known device types.
//! Skill files define device-specific commands.
//! Hydra treats physical devices like applications — first contact discovers capabilities.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A physical device Hydra can control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalDevice {
    pub name: String,
    pub device_type: DeviceType,
    pub base_url: String,
    pub api_key: Option<String>,
    pub capabilities: Vec<String>,
    pub discovered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    SmartLight,
    Printer3D,
    SmartPlug,
    Camera,
    Thermostat,
    Speaker,
    Display,
    Generic,
}

impl DeviceType {
    pub fn label(&self) -> &str {
        match self {
            Self::SmartLight => "smart-light", Self::Printer3D => "3d-printer",
            Self::SmartPlug => "smart-plug", Self::Camera => "camera",
            Self::Thermostat => "thermostat", Self::Speaker => "speaker",
            Self::Display => "display", Self::Generic => "generic",
        }
    }
}

/// Send a command to a physical device via HTTP.
pub fn send_command(device: &PhysicalDevice, endpoint: &str, body: Option<&str>) -> Result<String, String> {
    let url = format!("{}{}", device.base_url, endpoint);
    eprintln!("hydra-physical: {} → {url}", device.name);

    let handle = tokio::runtime::Handle::try_current()
        .map_err(|_| "No tokio runtime".to_string())?;

    tokio::task::block_in_place(|| {
        handle.block_on(async {
            let client = reqwest::Client::new();
            let mut req = if body.is_some() { client.post(&url) } else { client.get(&url) };
            req = req.timeout(std::time::Duration::from_secs(10));
            if let Some(key) = &device.api_key {
                req = req.header("Authorization", format!("Bearer {key}"));
            }
            if let Some(b) = body {
                req = req.header("Content-Type", "application/json").body(b.to_string());
            }
            let resp = req.send().await.map_err(|e| format!("HTTP: {e}"))?;
            let status = resp.status().as_u16();
            let text = resp.text().await.map_err(|e| format!("Read: {e}"))?;
            if status >= 400 { Err(format!("HTTP {status}: {text}")) }
            else { Ok(text) }
        })
    })
}

/// Discover devices on local network (simple probe).
pub fn discover_devices() -> Vec<PhysicalDevice> {
    let mut devices = Vec::new();

    // Check for common smart home bridges
    let probes = [
        ("http://philips-hue.local/api/config", DeviceType::SmartLight, "Philips Hue"),
        ("http://homeassistant.local:8123/api/", DeviceType::Generic, "Home Assistant"),
        ("http://octopi.local/api/version", DeviceType::Printer3D, "OctoPrint"),
    ];

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        for (url, dtype, name) in &probes {
            let result = tokio::task::block_in_place(|| {
                handle.block_on(async {
                    reqwest::Client::new().get(*url)
                        .timeout(std::time::Duration::from_secs(2))
                        .send().await.ok()
                        .filter(|r| r.status().is_success())
                })
            });
            if result.is_some() {
                let base = url.rsplit_once('/').map(|(b, _)| b).unwrap_or(url);
                devices.push(PhysicalDevice {
                    name: name.to_string(), device_type: dtype.clone(),
                    base_url: base.to_string(), api_key: None,
                    capabilities: vec!["on".into(), "off".into(), "status".into()],
                    discovered_at: chrono::Utc::now().to_rfc3339(),
                });
                eprintln!("hydra-physical: discovered {name} at {base}");
            }
        }
    }

    // Load configured devices from file
    let config_path = dirs::home_dir().unwrap_or_default().join(".hydra/devices.toml");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(table) = content.parse::<toml::Table>() {
            if let Some(devs) = table.get("devices").and_then(|d| d.as_array()) {
                for dev in devs {
                    if let (Some(name), Some(url)) = (
                        dev.get("name").and_then(|v| v.as_str()),
                        dev.get("url").and_then(|v| v.as_str()),
                    ) {
                        devices.push(PhysicalDevice {
                            name: name.into(), device_type: DeviceType::Generic,
                            base_url: url.into(),
                            api_key: dev.get("api_key").and_then(|v| v.as_str()).map(|s| s.into()),
                            capabilities: vec![], discovered_at: String::new(),
                        });
                    }
                }
            }
        }
    }

    devices
}

/// Save discovered devices to config.
pub fn save_devices(devices: &[PhysicalDevice]) {
    let path = dirs::home_dir().unwrap_or_default().join(".hydra/devices.toml");
    let mut content = String::from("# Hydra Physical Devices\n\n");
    for dev in devices {
        content.push_str(&format!(
            "[[devices]]\nname = \"{}\"\nurl = \"{}\"\ntype = \"{}\"\n\n",
            dev.name, dev.base_url, dev.device_type.label()));
    }
    let _ = std::fs::write(path, content);
}
