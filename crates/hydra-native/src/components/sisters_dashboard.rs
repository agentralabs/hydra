//! MCP Sisters Dashboard (Step 3.2).

use serde::{Deserialize, Serialize};

/// Health status of a connected sister.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SisterStatus {
    Healthy,
    Slow,
    Disconnected,
    Error(String),
}

/// Information about a single tool exposed by a sister.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub call_count: u64,
}

/// Detailed information about a sister.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SisterInfo {
    pub name: String,
    pub tool_count: usize,
    pub status: SisterStatus,
    pub latency_ms: Option<u64>,
    pub last_heartbeat: Option<String>,
    pub tools: Vec<ToolInfo>,
}

impl SisterInfo {
    /// Return a CSS class string based on the sister's status.
    pub fn status_css_class(&self) -> &'static str {
        match &self.status {
            SisterStatus::Healthy => "sister-healthy",
            SisterStatus::Slow => "sister-slow",
            SisterStatus::Disconnected => "sister-disconnected",
            SisterStatus::Error(_) => "sister-error",
        }
    }
}

/// The known 17 sisters in the Hydra ecosystem.
pub const KNOWN_SISTERS: &[&str] = &[
    "Memory",
    "Vision",
    "Codebase",
    "Evolve",
    "Monitor",
    "Sentinel",
    "Pulse",
    "Voice",
    "Scholar",
    "Scribe",
    "Forecast",
    "Orchestrate",
    "Bridge",
    "Ledger",
];

/// Dashboard managing the state and display of all MCP sisters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SistersDashboard {
    pub sisters: Vec<SisterInfo>,
    pub search_query: String,
    pub expanded_sister: Option<String>,
    pub total_tools: usize,
}

impl SistersDashboard {
    /// Create a new dashboard, computing total_tools from the provided sisters.
    pub fn new(sisters: Vec<SisterInfo>) -> Self {
        let total_tools = sisters.iter().map(|s| s.tool_count).sum();
        Self {
            sisters,
            search_query: String::new(),
            expanded_sister: None,
            total_tools,
        }
    }

    /// Create a dashboard with all 14 known sisters in disconnected state.
    pub fn default_14() -> Self {
        let sisters: Vec<SisterInfo> = KNOWN_SISTERS
            .iter()
            .map(|&name| SisterInfo {
                name: name.to_string(),
                tool_count: 0,
                status: SisterStatus::Disconnected,
                latency_ms: None,
                last_heartbeat: None,
                tools: Vec::new(),
            })
            .collect();
        Self::new(sisters)
    }

    /// Return sisters matching the current search query.
    /// Searches across sister names and tool names (case-insensitive).
    pub fn filtered_sisters(&self) -> Vec<&SisterInfo> {
        if self.search_query.is_empty() {
            return self.sisters.iter().collect();
        }

        let query = self.search_query.to_lowercase();
        self.sisters
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query)
                    || s.tools.iter().any(|t| t.name.to_lowercase().contains(&query))
            })
            .collect()
    }

    /// Toggle the expanded state for a sister by name.
    pub fn toggle_expand(&mut self, name: &str) {
        if self.expanded_sister.as_deref() == Some(name) {
            self.expanded_sister = None;
        } else {
            self.expanded_sister = Some(name.to_string());
        }
    }

    /// Return a summary of sister health: (healthy, slow, disconnected) counts.
    /// Sisters with `Error` status are counted as disconnected.
    pub fn health_summary(&self) -> (usize, usize, usize) {
        let mut healthy = 0;
        let mut slow = 0;
        let mut disconnected = 0;

        for s in &self.sisters {
            match &s.status {
                SisterStatus::Healthy => healthy += 1,
                SisterStatus::Slow => slow += 1,
                SisterStatus::Disconnected | SisterStatus::Error(_) => disconnected += 1,
            }
        }

        (healthy, slow, disconnected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sister(name: &str, status: SisterStatus, tools: Vec<ToolInfo>) -> SisterInfo {
        let tool_count = tools.len();
        SisterInfo {
            name: name.to_string(),
            tool_count,
            status,
            latency_ms: None,
            last_heartbeat: None,
            tools,
        }
    }

    #[test]
    fn test_default_14_sisters() {
        let dash = SistersDashboard::default_14();
        assert_eq!(dash.sisters.len(), 14);
        assert_eq!(dash.total_tools, 0);
        for s in &dash.sisters {
            assert_eq!(s.status, SisterStatus::Disconnected);
        }
        let names: Vec<&str> = dash.sisters.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Memory"));
        assert!(names.contains(&"Ledger"));
        assert!(names.contains(&"Voice"));
    }

    #[test]
    fn test_total_tools_computed() {
        let sisters = vec![
            make_sister("A", SisterStatus::Healthy, vec![
                ToolInfo { name: "t1".into(), description: "desc".into(), call_count: 0 },
                ToolInfo { name: "t2".into(), description: "desc".into(), call_count: 0 },
            ]),
            make_sister("B", SisterStatus::Healthy, vec![
                ToolInfo { name: "t3".into(), description: "desc".into(), call_count: 5 },
            ]),
        ];
        let dash = SistersDashboard::new(sisters);
        assert_eq!(dash.total_tools, 3);
    }

    #[test]
    fn test_health_summary() {
        let sisters = vec![
            make_sister("A", SisterStatus::Healthy, vec![]),
            make_sister("B", SisterStatus::Healthy, vec![]),
            make_sister("C", SisterStatus::Slow, vec![]),
            make_sister("D", SisterStatus::Disconnected, vec![]),
            make_sister("E", SisterStatus::Error("timeout".into()), vec![]),
        ];
        let dash = SistersDashboard::new(sisters);
        let (healthy, slow, disconnected) = dash.health_summary();
        assert_eq!(healthy, 2);
        assert_eq!(slow, 1);
        assert_eq!(disconnected, 2); // Disconnected + Error
    }

    #[test]
    fn test_toggle_expand() {
        let mut dash = SistersDashboard::default_14();
        assert!(dash.expanded_sister.is_none());

        dash.toggle_expand("Memory");
        assert_eq!(dash.expanded_sister.as_deref(), Some("Memory"));

        dash.toggle_expand("Memory");
        assert!(dash.expanded_sister.is_none());

        dash.toggle_expand("Vision");
        assert_eq!(dash.expanded_sister.as_deref(), Some("Vision"));

        dash.toggle_expand("Pulse");
        assert_eq!(dash.expanded_sister.as_deref(), Some("Pulse"));
    }

    #[test]
    fn test_filter_by_sister_name() {
        let mut dash = SistersDashboard::default_14();
        dash.search_query = "mem".into();
        let filtered = dash.filtered_sisters();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Memory");
    }

    #[test]
    fn test_filter_by_tool_name() {
        let sisters = vec![
            make_sister("Memory", SisterStatus::Healthy, vec![
                ToolInfo { name: "store_memory".into(), description: "Store".into(), call_count: 0 },
            ]),
            make_sister("Vision", SisterStatus::Healthy, vec![
                ToolInfo { name: "analyze_image".into(), description: "Analyze".into(), call_count: 0 },
            ]),
        ];
        let mut dash = SistersDashboard::new(sisters);
        dash.search_query = "image".into();
        let filtered = dash.filtered_sisters();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Vision");
    }

    #[test]
    fn test_empty_search_returns_all() {
        let dash = SistersDashboard::default_14();
        let filtered = dash.filtered_sisters();
        assert_eq!(filtered.len(), 14);
    }

    #[test]
    fn test_status_css_class() {
        assert_eq!(
            make_sister("A", SisterStatus::Healthy, vec![]).status_css_class(),
            "sister-healthy"
        );
        assert_eq!(
            make_sister("B", SisterStatus::Slow, vec![]).status_css_class(),
            "sister-slow"
        );
        assert_eq!(
            make_sister("C", SisterStatus::Disconnected, vec![]).status_css_class(),
            "sister-disconnected"
        );
        assert_eq!(
            make_sister("D", SisterStatus::Error("x".into()), vec![]).status_css_class(),
            "sister-error"
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let dash = SistersDashboard::default_14();
        let json = serde_json::to_string(&dash).unwrap();
        let back: SistersDashboard = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sisters.len(), 14);
        assert_eq!(back.total_tools, 0);
    }

    #[test]
    fn test_sister_status_serialization() {
        let statuses = vec![
            SisterStatus::Healthy,
            SisterStatus::Slow,
            SisterStatus::Disconnected,
            SisterStatus::Error("network timeout".into()),
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: SisterStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn test_known_sisters_count() {
        assert_eq!(KNOWN_SISTERS.len(), 14);
    }
}
