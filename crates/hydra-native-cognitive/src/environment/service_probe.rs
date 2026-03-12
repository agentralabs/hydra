//! Service probe — detects running services by checking listening ports.

use super::os_probe::run_cmd;

/// A detected running service.
#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub port: u16,
    pub protocol: String,
}

impl Service {
    pub fn display(&self) -> String {
        format!("{} :{}", self.name, self.port)
    }
}

/// Known services by their default ports.
const KNOWN_SERVICES: &[(u16, &str)] = &[
    (5432, "PostgreSQL"),
    (3306, "MySQL"),
    (27017, "MongoDB"),
    (6379, "Redis"),
    (11211, "Memcached"),
    (9200, "Elasticsearch"),
    (5672, "RabbitMQ"),
    (9092, "Kafka"),
    (8080, "HTTP Proxy"),
    (8443, "HTTPS Alt"),
    (3000, "Dev Server"),
    (4000, "Dev Server"),
    (5000, "Dev Server"),
    (8000, "Dev Server"),
    (443, "HTTPS"),
    (80, "HTTP"),
    (22, "SSH"),
    (2375, "Docker"),
    (2376, "Docker TLS"),
    (9090, "Prometheus"),
    (3100, "Grafana Loki"),
    (8500, "Consul"),
    (2181, "ZooKeeper"),
    (1883, "MQTT"),
];

/// Probe for running services by checking listening ports.
pub fn probe_services() -> Vec<Service> {
    let output = if cfg!(target_os = "macos") {
        // macOS: use lsof to find listening TCP ports
        run_cmd("lsof", &["-iTCP", "-sTCP:LISTEN", "-P", "-n"])
    } else {
        // Linux: use ss
        run_cmd("ss", &["-tlnp"])
    };

    let output = match output {
        Some(o) => o,
        None => return Vec::new(),
    };

    parse_listening_ports(&output)
}

fn parse_listening_ports(output: &str) -> Vec<Service> {
    let mut seen_ports = std::collections::HashSet::new();
    let mut services = Vec::new();

    for line in output.lines().skip(1) {
        // Skip header
        if let Some(port) = extract_port(line) {
            if seen_ports.insert(port) {
                let name = identify_service(port);
                services.push(Service {
                    name,
                    port,
                    protocol: "TCP".to_string(),
                });
            }
        }
    }

    // Sort by port number
    services.sort_by_key(|s| s.port);

    // Only return known services (skip random ephemeral ports)
    services
        .into_iter()
        .filter(|s| s.name != "Unknown" || s.port < 10000)
        .take(20) // Cap at 20 services
        .collect()
}

fn extract_port(line: &str) -> Option<u16> {
    // lsof format: "... *:5432 ..." or "... 127.0.0.1:5432 ..."
    // ss format: "... 0.0.0.0:5432 ..." or "... *:5432 ..."
    // Only look at words containing ':' to avoid matching PIDs
    for word in line.split_whitespace() {
        if word.contains(':') {
            if let Some(port_str) = word.rsplit(':').next() {
                // Strip trailing non-digit chars like "(LISTEN)"
                let clean = port_str.trim_end_matches(|c: char| !c.is_ascii_digit());
                if let Ok(port) = clean.parse::<u16>() {
                    if port > 0 {
                        return Some(port);
                    }
                }
            }
        }
    }
    None
}

fn identify_service(port: u16) -> String {
    KNOWN_SERVICES
        .iter()
        .find(|(p, _)| *p == port)
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_services_no_crash() {
        // Should not crash, may return empty vec
        let services = probe_services();
        // Just verify it's a valid vec
        assert!(services.len() <= 20);
    }

    #[test]
    fn test_identify_known_service() {
        assert_eq!(identify_service(5432), "PostgreSQL");
        assert_eq!(identify_service(6379), "Redis");
        assert_eq!(identify_service(22), "SSH");
    }

    #[test]
    fn test_identify_unknown_service() {
        assert_eq!(identify_service(59999), "Unknown");
    }

    #[test]
    fn test_service_display() {
        let s = Service { name: "PostgreSQL".into(), port: 5432, protocol: "TCP".into() };
        assert_eq!(s.display(), "PostgreSQL :5432");
    }

    #[test]
    fn test_extract_port() {
        assert_eq!(extract_port("TCP *:5432 (LISTEN)"), Some(5432));
        assert_eq!(extract_port("127.0.0.1:3000"), Some(3000));
    }

    #[test]
    fn test_parse_listening_ports() {
        let output = "COMMAND PID USER FD TYPE\nnode 1234 user 22u IPv4 TCP *:3000 (LISTEN)\npostgres 5678 user 5u IPv4 TCP *:5432 (LISTEN)\n";
        let services = parse_listening_ports(output);
        assert!(services.iter().any(|s| s.port == 3000));
        assert!(services.iter().any(|s| s.port == 5432));
    }
}
