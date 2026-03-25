//! Layer 1: Differential Perception — region-based screen analysis.
//!
//! Instead of full-screen LLM analysis every step, divides the screen into
//! a grid of cells, hashes each, and only re-analyzes CHANGED cells.
//! Like video compression (keyframes + deltas) vs photo sequences.
//! Also handles coordinate space transforms (scale factor, bounds validation).

use std::time::Instant;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const GRID_COLS: usize = 8;
const GRID_ROWS: usize = 6;

/// A cached UI element found in a screen region.
#[derive(Debug, Clone)]
pub struct CachedElement {
    pub label: String,
    pub role: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// State of a single grid cell.
#[derive(Debug, Clone)]
struct RegionState {
    hash: u64,
    elements: Vec<CachedElement>,
    last_analyzed: Instant,
}

/// Coordinate space for display transforms.
#[derive(Debug, Clone)]
pub struct CoordinateSpace {
    pub scale_factor: f64,
    pub window_offset_x: f64,
    pub window_offset_y: f64,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl CoordinateSpace {
    /// Build from current display info.
    pub fn detect() -> Self {
        let info = crate::screen::ScreenCapture::capture_full()
            .map(|(_, i)| i)
            .unwrap_or_else(|_| crate::screen::ScreenshotInfo {
                width: 1920, height: 1080, scale_factor: 1.0,
                bytes_len: 0, format: "png".into(),
            });
        Self {
            scale_factor: info.scale_factor,
            window_offset_x: 0.0,
            window_offset_y: 0.0,
            screen_width: info.width,
            screen_height: info.height,
        }
    }

    /// Convert logical coordinates to physical click coordinates.
    pub fn to_physical(&self, x: f64, y: f64) -> (f64, f64) {
        ((x + self.window_offset_x) / self.scale_factor,
         (y + self.window_offset_y) / self.scale_factor)
    }

    /// Validate coordinates are within screen bounds.
    pub fn validate(&self, x: f64, y: f64) -> bool {
        x >= 0.0 && y >= 0.0
            && x < self.screen_width as f64
            && y < self.screen_height as f64
    }
}

/// What changed between perception cycles.
#[derive(Debug, Clone)]
pub struct PerceptionDelta {
    pub changed_cells: Vec<(usize, usize)>,
    pub total_cells: usize,
    pub change_ratio: f64,
}

/// The perception field — persistent screen awareness.
pub struct PerceptionField {
    grid: Vec<Vec<RegionState>>,
    pub focus: (usize, usize),
    pub space: CoordinateSpace,
    cell_width: u32,
    cell_height: u32,
}

impl PerceptionField {
    /// Create a new perception field for the current display.
    pub fn new() -> Self {
        let space = CoordinateSpace::detect();
        let cell_width = space.screen_width / GRID_COLS as u32;
        let cell_height = space.screen_height / GRID_ROWS as u32;
        let now = Instant::now();
        let grid = (0..GRID_ROWS).map(|_| {
            (0..GRID_COLS).map(|_| RegionState {
                hash: 0, elements: Vec::new(), last_analyzed: now,
            }).collect()
        }).collect();
        Self { grid, focus: (0, 0), space, cell_width, cell_height }
    }

    /// Analyze a screenshot and return only what changed.
    pub fn perceive_delta(&mut self, screenshot: &[u8]) -> PerceptionDelta {
        let new_hashes = self.hash_grid(screenshot);
        let mut changed = Vec::new();
        for r in 0..GRID_ROWS {
            for c in 0..GRID_COLS {
                if new_hashes[r][c] != self.grid[r][c].hash {
                    self.grid[r][c].hash = new_hashes[r][c];
                    self.grid[r][c].last_analyzed = Instant::now();
                    changed.push((r, c));
                }
            }
        }
        let total = GRID_ROWS * GRID_COLS;
        PerceptionDelta {
            change_ratio: changed.len() as f64 / total as f64,
            changed_cells: changed, total_cells: total,
        }
    }

    /// Focus attention on the region containing (x, y).
    pub fn focus_on(&mut self, x: f64, y: f64) {
        let c = (x as u32 / self.cell_width.max(1)).min(GRID_COLS as u32 - 1) as usize;
        let r = (y as u32 / self.cell_height.max(1)).min(GRID_ROWS as u32 - 1) as usize;
        self.focus = (r, c);
    }

    /// Check if a specific region changed since last perception.
    pub fn region_changed(&self, x: f64, y: f64) -> bool {
        let c = (x as u32 / self.cell_width.max(1)).min(GRID_COLS as u32 - 1) as usize;
        let r = (y as u32 / self.cell_height.max(1)).min(GRID_ROWS as u32 - 1) as usize;
        self.grid[r][c].last_analyzed.elapsed().as_millis() < 500
    }

    /// Hash each grid cell by sampling pixel bytes from the screenshot.
    fn hash_grid(&self, screenshot: &[u8]) -> Vec<Vec<u64>> {
        // Simple: hash chunks of the raw bytes corresponding to each grid cell
        let chunk_size = screenshot.len() / (GRID_ROWS * GRID_COLS).max(1);
        (0..GRID_ROWS).map(|r| {
            (0..GRID_COLS).map(|c| {
                let offset = (r * GRID_COLS + c) * chunk_size;
                let end = (offset + chunk_size).min(screenshot.len());
                let mut hasher = DefaultHasher::new();
                if offset < screenshot.len() {
                    screenshot[offset..end].hash(&mut hasher);
                }
                hasher.finish()
            }).collect()
        }).collect()
    }
}

impl Default for PerceptionField {
    fn default() -> Self { Self::new() }
}
