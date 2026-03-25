//! O33: Atomic Input Algebra — 6 atoms that compose into every human input.
//!
//! PRESS + RELEASE + MOVE + WHEEL + WAIT + CLIPBOARD = complete input coverage.
//! Every mouse drag, modifier+click, scroll, copy-paste is a sequence of these atoms.
//! This is mathematically complete: it spans the entire input state space.

use crate::errors::DesktopError;
use serde::{Deserialize, Serialize};

/// What to press or release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Target {
    MouseLeft,
    MouseRight,
    MouseMiddle,
    Key(String),
    Modifier(String),
}

/// When to stop waiting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaitCondition {
    Duration(u64),
    ScreenStable { timeout_ms: u64 },
    TextAppears { text: String, timeout_ms: u64 },
}

/// One of the 6 atomic input operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputAtom {
    Press(Target),
    Release(Target),
    Move { x: f64, y: f64 },
    Wheel { dx: i32, dy: i32 },
    Wait(WaitCondition),
    Clipboard(String),
}

/// Executes atom sequences with state tracking.
pub struct InputComposer {
    held: Vec<Target>,
}

impl InputComposer {
    pub fn new() -> Self { Self { held: Vec::new() } }

    pub fn execute(&mut self, atoms: &[InputAtom]) -> Result<(), DesktopError> {
        for atom in atoms { self.execute_one(atom)?; }
        Ok(())
    }

    pub fn execute_one(&mut self, atom: &InputAtom) -> Result<(), DesktopError> {
        match atom {
            InputAtom::Press(target) => {
                crate::input_platform::press(target)?;
                self.held.push(target.clone());
            }
            InputAtom::Release(target) => {
                crate::input_platform::release(target)?;
                self.held.retain(|t| std::mem::discriminant(t) != std::mem::discriminant(target)
                    || match (t, target) {
                        (Target::Key(a), Target::Key(b)) => a != b,
                        (Target::Modifier(a), Target::Modifier(b)) => a != b,
                        _ => false,
                    });
            }
            InputAtom::Move { x, y } => {
                crate::input_platform::mouse_move(*x as i32, *y as i32)?;
            }
            InputAtom::Wheel { dx, dy } => {
                crate::input_platform::scroll_wheel(*dx, *dy)?;
            }
            InputAtom::Wait(condition) => {
                match condition {
                    WaitCondition::Duration(ms) =>
                        std::thread::sleep(std::time::Duration::from_millis(*ms)),
                    WaitCondition::ScreenStable { timeout_ms } => {
                        wait_screen_stable(*timeout_ms);
                    }
                    WaitCondition::TextAppears { text, timeout_ms } => {
                        wait_for_text_impl(text, *timeout_ms);
                    }
                }
            }
            InputAtom::Clipboard(text) => {
                crate::input_platform::clipboard_write(text)?;
            }
        }
        // Small inter-atom delay for reliability
        std::thread::sleep(std::time::Duration::from_millis(5));
        Ok(())
    }

    /// Release all held keys/buttons (safety cleanup).
    pub fn release_all(&mut self) -> Result<(), DesktopError> {
        for target in self.held.drain(..).collect::<Vec<_>>() {
            let _ = crate::input_platform::release(&target);
        }
        Ok(())
    }
}

impl Drop for InputComposer {
    fn drop(&mut self) { let _ = self.release_all(); }
}

// ── Composed Operations (built from atoms) ──

/// Drag from (x1,y1) to (x2,y2) with smooth movement.
pub fn drag(x1: f64, y1: f64, x2: f64, y2: f64) -> Vec<InputAtom> {
    let steps = interpolate_move(x1, y1, x2, y2, 20);
    let mut atoms = vec![
        InputAtom::Move { x: x1, y: y1 },
        InputAtom::Wait(WaitCondition::Duration(50)),
        InputAtom::Press(Target::MouseLeft),
        InputAtom::Wait(WaitCondition::Duration(50)),
    ];
    for (x, y) in steps { atoms.push(InputAtom::Move { x, y }); }
    atoms.push(InputAtom::Wait(WaitCondition::Duration(30)));
    atoms.push(InputAtom::Release(Target::MouseLeft));
    atoms
}

/// Scroll wheel at position.
pub fn scroll(x: f64, y: f64, dy: i32) -> Vec<InputAtom> {
    vec![
        InputAtom::Move { x, y },
        InputAtom::Wheel { dx: 0, dy },
    ]
}

/// Click with a modifier held (Shift+Click, Cmd+Click, etc).
pub fn modifier_click(x: f64, y: f64, modifier: &str) -> Vec<InputAtom> {
    vec![
        InputAtom::Press(Target::Modifier(modifier.into())),
        InputAtom::Move { x, y },
        InputAtom::Press(Target::MouseLeft),
        InputAtom::Release(Target::MouseLeft),
        InputAtom::Release(Target::Modifier(modifier.into())),
    ]
}

/// Drag with a modifier held (Alt+Drag to duplicate, Shift+Drag to constrain).
pub fn modifier_drag(x1: f64, y1: f64, x2: f64, y2: f64, modifier: &str) -> Vec<InputAtom> {
    let steps = interpolate_move(x1, y1, x2, y2, 15);
    let mut atoms = vec![
        InputAtom::Press(Target::Modifier(modifier.into())),
        InputAtom::Move { x: x1, y: y1 },
        InputAtom::Wait(WaitCondition::Duration(30)),
        InputAtom::Press(Target::MouseLeft),
    ];
    for (x, y) in steps { atoms.push(InputAtom::Move { x, y }); }
    atoms.push(InputAtom::Release(Target::MouseLeft));
    atoms.push(InputAtom::Release(Target::Modifier(modifier.into())));
    atoms
}

/// Write to clipboard then paste (Cmd+V / Ctrl+V).
pub fn clipboard_paste(text: &str) -> Vec<InputAtom> {
    let modifier = if cfg!(target_os = "macos") { "cmd" } else { "ctrl" };
    vec![
        InputAtom::Clipboard(text.into()),
        InputAtom::Wait(WaitCondition::Duration(50)),
        InputAtom::Press(Target::Modifier(modifier.into())),
        InputAtom::Press(Target::Key("v".into())),
        InputAtom::Release(Target::Key("v".into())),
        InputAtom::Release(Target::Modifier(modifier.into())),
    ]
}

/// Right-click at position, then click on menu item by offset.
pub fn context_menu_select(x: f64, y: f64, item_offset_y: f64) -> Vec<InputAtom> {
    vec![
        InputAtom::Move { x, y },
        InputAtom::Press(Target::MouseRight),
        InputAtom::Release(Target::MouseRight),
        InputAtom::Wait(WaitCondition::Duration(300)), // wait for menu
        InputAtom::Move { x, y: y + item_offset_y },
        InputAtom::Press(Target::MouseLeft),
        InputAtom::Release(Target::MouseLeft),
    ]
}

/// Multi-modifier key combo (Cmd+Shift+S, Ctrl+Alt+Del).
pub fn multi_modifier_combo(modifiers: &[&str], key: &str) -> Vec<InputAtom> {
    let mut atoms = Vec::new();
    for m in modifiers { atoms.push(InputAtom::Press(Target::Modifier(m.to_string()))); }
    atoms.push(InputAtom::Press(Target::Key(key.into())));
    atoms.push(InputAtom::Release(Target::Key(key.into())));
    for m in modifiers.iter().rev() { atoms.push(InputAtom::Release(Target::Modifier(m.to_string()))); }
    atoms
}

/// Wait for screen to stop changing.
pub fn wait_stable(timeout_ms: u64) -> Vec<InputAtom> {
    vec![InputAtom::Wait(WaitCondition::ScreenStable { timeout_ms })]
}

/// Wait for specific text to appear on screen.
pub fn wait_text(text: &str, timeout_ms: u64) -> Vec<InputAtom> {
    vec![InputAtom::Wait(WaitCondition::TextAppears { text: text.into(), timeout_ms })]
}

// ── Helpers ──

fn interpolate_move(x1: f64, y1: f64, x2: f64, y2: f64, steps: usize) -> Vec<(f64, f64)> {
    (1..=steps).map(|i| {
        let t = i as f64 / steps as f64;
        let frac = 10.0 * t.powi(3) - 15.0 * t.powi(4) + 6.0 * t.powi(5); // minimum-jerk
        (x1 + (x2 - x1) * frac, y1 + (y2 - y1) * frac)
    }).collect()
}

fn wait_screen_stable(timeout_ms: u64) {
    let start = std::time::Instant::now();
    let mut last_hash: u64 = 0;
    let mut stable_count = 0;
    while start.elapsed().as_millis() < timeout_ms as u128 {
        if let Ok((bytes, _)) = crate::screen::ScreenCapture::capture_full() {
            let hash = simple_hash(&bytes);
            if hash == last_hash { stable_count += 1; } else { stable_count = 0; }
            last_hash = hash;
            if stable_count >= 3 { return; } // 3 consecutive same = stable
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn wait_for_text_impl(target: &str, timeout_ms: u64) {
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < timeout_ms as u128 {
        if let Ok(regions) = crate::ocr::ocr_current_screen() {
            if regions.iter().any(|r| r.text.to_lowercase().contains(&target.to_lowercase())) {
                return;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    eprintln!("hydra-input: wait_for_text '{target}' timed out after {timeout_ms}ms");
}

fn simple_hash(data: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    // Sample every 1000th byte for speed
    for (i, b) in data.iter().enumerate() { if i % 1000 == 0 { b.hash(&mut h); } }
    h.finish()
}

impl Default for InputComposer {
    fn default() -> Self { Self::new() }
}
