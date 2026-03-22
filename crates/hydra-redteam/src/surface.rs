//! AttackSurface -- exposed vulnerabilities in a proposed action.

use serde::{Deserialize, Serialize};

/// One identified attack surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackSurface {
    pub id: String,
    pub name: String,
    pub description: String,
    pub exposure: f64, // 0.0-1.0
    pub exploitable: bool,
}

impl AttackSurface {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        exposure: f64,
        exploitable: bool,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            exposure: exposure.clamp(0.0, 1.0),
            exploitable,
        }
    }
}

/// Identify attack surfaces from context.
pub fn identify_surfaces(context: &str) -> Vec<AttackSurface> {
    let lower = context.to_lowercase();
    let mut surfaces = Vec::new();

    if lower.contains("auth") || lower.contains("token") || lower.contains("cert") {
        surfaces.push(AttackSurface::new(
            "Authentication Surface",
            "Credential handling and token validation",
            0.85,
            true,
        ));
    }
    if lower.contains("api") || lower.contains("endpoint") {
        surfaces.push(AttackSurface::new(
            "API Endpoint Surface",
            "External-facing API endpoints",
            0.75,
            true,
        ));
    }
    if lower.contains("deploy") || lower.contains("release") {
        surfaces.push(AttackSurface::new(
            "Deployment Surface",
            "Deployment pipeline and artifact integrity",
            0.70,
            true,
        ));
    }
    if lower.contains("database") || lower.contains("db") || lower.contains("storage") {
        surfaces.push(AttackSurface::new(
            "Data Storage Surface",
            "Database and persistent storage access",
            0.80,
            true,
        ));
    }
    if lower.contains("network") || lower.contains("firewall") || lower.contains("port") {
        surfaces.push(AttackSurface::new(
            "Network Surface",
            "Network exposure and firewall rules",
            0.65,
            true,
        ));
    }

    surfaces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_context_identifies_auth_surface() {
        let surfaces = identify_surfaces("deploy with auth token and cert rotation");
        assert!(surfaces.iter().any(|s| s.name.contains("Authentication")));
    }

    #[test]
    fn deployment_context_identifies_deployment_surface() {
        let surfaces = identify_surfaces("release to production via deployment pipeline");
        assert!(surfaces.iter().any(|s| s.name.contains("Deployment")));
    }
}
