//! Workspace panel container with resize handles (Step 3.6).

use serde::{Deserialize, Serialize};

/// The type of content a panel displays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelType {
    Plan,
    Timeline,
    Evidence,
    Code,
    Diff,
}

/// Tabs available within the evidence panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTab {
    Code,
    Memory,
    Diffs,
    Logs,
}

/// Configuration for a single panel in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    pub id: String,
    pub label: String,
    pub width_percent: f64,
    pub min_width: f64,
    pub collapsed: bool,
    pub panel_type: PanelType,
}

/// Layout of all panels in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelLayout {
    pub panels: Vec<PanelConfig>,
    pub total_width: f64,
}

impl PanelLayout {
    /// Create the standard 3-panel workspace layout:
    /// Plan (25%), Evidence (40%), Timeline (35%).
    pub fn default_workspace() -> Self {
        Self {
            panels: vec![
                PanelConfig {
                    id: "plan".into(),
                    label: "Plan".into(),
                    width_percent: 25.0,
                    min_width: 10.0,
                    collapsed: false,
                    panel_type: PanelType::Plan,
                },
                PanelConfig {
                    id: "evidence".into(),
                    label: "Evidence".into(),
                    width_percent: 40.0,
                    min_width: 15.0,
                    collapsed: false,
                    panel_type: PanelType::Evidence,
                },
                PanelConfig {
                    id: "timeline".into(),
                    label: "Timeline".into(),
                    width_percent: 35.0,
                    min_width: 10.0,
                    collapsed: false,
                    panel_type: PanelType::Timeline,
                },
            ],
            total_width: 100.0,
        }
    }

    /// Resize a panel to `new_percent` width, redistributing the difference
    /// proportionally among the other visible panels while enforcing minimum widths.
    pub fn resize(&mut self, panel_id: &str, new_percent: f64) {
        let idx = match self.panels.iter().position(|p| p.id == panel_id) {
            Some(i) => i,
            None => return,
        };

        if self.panels[idx].collapsed {
            return;
        }

        let old_percent = self.panels[idx].width_percent;
        let clamped = new_percent.max(self.panels[idx].min_width).min(90.0);
        let delta = clamped - old_percent;

        if delta.abs() < 0.001 {
            return;
        }

        // Collect indices of other visible panels
        let others: Vec<usize> = self
            .panels
            .iter()
            .enumerate()
            .filter(|(i, p)| *i != idx && !p.collapsed)
            .map(|(i, _)| i)
            .collect();

        if others.is_empty() {
            return;
        }

        // Total width available from others
        let others_total: f64 = others.iter().map(|&i| self.panels[i].width_percent).sum();

        if others_total.abs() < 0.001 {
            return;
        }

        // Apply resize to the target panel
        self.panels[idx].width_percent = clamped;

        // Distribute the negative delta proportionally among others
        for &i in &others {
            let share = self.panels[i].width_percent / others_total;
            let adjustment = delta * share;
            self.panels[i].width_percent = (self.panels[i].width_percent - adjustment).max(self.panels[i].min_width);
        }

        // Normalize so total is 100%
        self.normalize();
    }

    /// Toggle a panel between collapsed and expanded states.
    /// When collapsed, the panel's width is redistributed among visible panels.
    /// When expanded, width is reclaimed proportionally.
    pub fn toggle_collapse(&mut self, panel_id: &str) {
        let idx = match self.panels.iter().position(|p| p.id == panel_id) {
            Some(i) => i,
            None => return,
        };

        self.panels[idx].collapsed = !self.panels[idx].collapsed;

        if self.panels[idx].collapsed {
            // Redistribute this panel's width to others
            let freed = self.panels[idx].width_percent;
            self.panels[idx].width_percent = 0.0;

            let visible: Vec<usize> = self
                .panels
                .iter()
                .enumerate()
                .filter(|(i, p)| *i != idx && !p.collapsed)
                .map(|(i, _)| i)
                .collect();

            if !visible.is_empty() {
                let share = freed / visible.len() as f64;
                for &i in &visible {
                    self.panels[i].width_percent += share;
                }
            }
        } else {
            // Restore with a fair share
            let visible_count = self.panels.iter().filter(|p| !p.collapsed).count();
            if visible_count > 0 {
                let fair_share = 100.0 / visible_count as f64;
                // Set all visible panels to fair share
                for p in &mut self.panels {
                    if !p.collapsed {
                        p.width_percent = fair_share;
                    }
                }
            }
        }
    }

    /// Return actual widths for each panel, accounting for collapsed panels.
    /// Collapsed panels have zero width.
    pub fn panel_widths(&self) -> Vec<(String, f64)> {
        self.panels
            .iter()
            .map(|p| {
                let width = if p.collapsed { 0.0 } else { p.width_percent };
                (p.id.clone(), width)
            })
            .collect()
    }

    /// Normalize panel widths so visible panels sum to 100%.
    fn normalize(&mut self) {
        let visible_total: f64 = self
            .panels
            .iter()
            .filter(|p| !p.collapsed)
            .map(|p| p.width_percent)
            .sum();

        if visible_total.abs() < 0.001 {
            return;
        }

        let scale = 100.0 / visible_total;
        for p in &mut self.panels {
            if !p.collapsed {
                p.width_percent *= scale;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_workspace_layout() {
        let layout = PanelLayout::default_workspace();
        assert_eq!(layout.panels.len(), 3);
        assert!((layout.panels[0].width_percent - 25.0).abs() < 0.01);
        assert!((layout.panels[1].width_percent - 40.0).abs() < 0.01);
        assert!((layout.panels[2].width_percent - 35.0).abs() < 0.01);
        assert_eq!(layout.panels[0].panel_type, PanelType::Plan);
        assert_eq!(layout.panels[1].panel_type, PanelType::Evidence);
        assert_eq!(layout.panels[2].panel_type, PanelType::Timeline);
    }

    #[test]
    fn test_widths_sum_to_100() {
        let layout = PanelLayout::default_workspace();
        let total: f64 = layout.panel_widths().iter().map(|(_, w)| w).sum();
        assert!((total - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_resize_redistributes() {
        let mut layout = PanelLayout::default_workspace();
        layout.resize("plan", 40.0);

        // Plan should be ~40%
        let widths = layout.panel_widths();
        let plan_width = widths.iter().find(|(id, _)| id == "plan").unwrap().1;
        assert!((plan_width - 40.0).abs() < 1.0);

        // Total should still be ~100%
        let total: f64 = widths.iter().map(|(_, w)| w).sum();
        assert!((total - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_collapse_panel() {
        let mut layout = PanelLayout::default_workspace();
        layout.toggle_collapse("plan");

        let widths = layout.panel_widths();
        let plan_width = widths.iter().find(|(id, _)| id == "plan").unwrap().1;
        assert!((plan_width - 0.0).abs() < 0.01);

        // Remaining panels should sum to 100%
        let visible_total: f64 = widths.iter().filter(|(id, _)| id != "plan").map(|(_, w)| w).sum();
        assert!((visible_total - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_expand_collapsed_panel() {
        let mut layout = PanelLayout::default_workspace();
        layout.toggle_collapse("plan");
        layout.toggle_collapse("plan");

        // All panels visible again, should sum to 100%
        let widths = layout.panel_widths();
        let total: f64 = widths.iter().map(|(_, w)| w).sum();
        assert!((total - 100.0).abs() < 0.01);

        // Plan should have non-zero width
        let plan_width = widths.iter().find(|(id, _)| id == "plan").unwrap().1;
        assert!(plan_width > 5.0);
    }

    #[test]
    fn test_resize_nonexistent_panel() {
        let mut layout = PanelLayout::default_workspace();
        let before: Vec<f64> = layout.panels.iter().map(|p| p.width_percent).collect();
        layout.resize("nonexistent", 50.0);
        let after: Vec<f64> = layout.panels.iter().map(|p| p.width_percent).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn test_evidence_tab_serialization() {
        let tabs = [EvidenceTab::Code, EvidenceTab::Memory, EvidenceTab::Diffs, EvidenceTab::Logs];
        for tab in &tabs {
            let json = serde_json::to_string(tab).unwrap();
            let back: EvidenceTab = serde_json::from_str(&json).unwrap();
            assert_eq!(*tab, back);
        }
    }

    #[test]
    fn test_panel_type_serialization() {
        let types = [PanelType::Plan, PanelType::Timeline, PanelType::Evidence, PanelType::Code, PanelType::Diff];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let back: PanelType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, back);
        }
    }

    #[test]
    fn test_layout_serialization_roundtrip() {
        let layout = PanelLayout::default_workspace();
        let json = serde_json::to_string(&layout).unwrap();
        let back: PanelLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(back.panels.len(), 3);
        assert!((back.panels[0].width_percent - 25.0).abs() < 0.01);
    }
}
