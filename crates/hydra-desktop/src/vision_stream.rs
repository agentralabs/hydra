//! O38: Continuous Vision — background screen capture stream.
//!
//! Instead of on-demand screenshots, captures frames continuously at configurable FPS.
//! Feeds into the perception field's differential analysis.
//! Hydra sees the screen like a human — not in snapshots, but as a continuous stream.

use crate::errors::DesktopError;
use crate::screen::ScreenCapture;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A continuously captured frame.
pub struct Frame {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub captured_at: Instant,
}

/// Continuous vision stream — captures at configurable FPS.
pub struct VisionStream {
    latest_frame: Arc<Mutex<Option<Frame>>>,
    fps: u32,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl VisionStream {
    /// Start continuous capture at given FPS (default 2 = every 500ms).
    pub fn start(fps: u32) -> Self {
        let latest = Arc::new(Mutex::new(None));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let frame_ref = Arc::clone(&latest);
        let run_ref = Arc::clone(&running);
        let interval = Duration::from_millis(1000 / fps.max(1) as u64);

        std::thread::spawn(move || {
            eprintln!("hydra-vision: continuous capture started at {fps} FPS");
            while run_ref.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok((bytes, info)) = ScreenCapture::capture_full() {
                    let frame = Frame {
                        bytes, width: info.width, height: info.height,
                        captured_at: Instant::now(),
                    };
                    if let Ok(mut lock) = frame_ref.lock() { *lock = Some(frame); }
                }
                std::thread::sleep(interval);
            }
            eprintln!("hydra-vision: continuous capture stopped");
        });

        Self { latest_frame: latest, fps, running }
    }

    /// Get the latest frame (non-blocking). Returns None if no frame captured yet.
    pub fn latest(&self) -> Option<Frame> {
        self.latest_frame.lock().ok().and_then(|lock| {
            lock.as_ref().map(|f| Frame {
                bytes: f.bytes.clone(), width: f.width, height: f.height,
                captured_at: f.captured_at,
            })
        })
    }

    /// How fresh is the latest frame?
    pub fn frame_age_ms(&self) -> u64 {
        self.latest_frame.lock().ok()
            .and_then(|lock| lock.as_ref().map(|f| f.captured_at.elapsed().as_millis() as u64))
            .unwrap_or(u64::MAX)
    }

    /// Stop the capture thread.
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn fps(&self) -> u32 { self.fps }
}

impl Drop for VisionStream {
    fn drop(&mut self) { self.stop(); }
}

impl Default for VisionStream {
    fn default() -> Self { Self::start(2) }
}
