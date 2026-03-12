const SPINNER_FRAMES: &[&str] = &[
    "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}",
    "\u{2826}", "\u{2827}", "\u{2807}", "\u{280f}",
];

pub struct Spinner {
    frames: Vec<&'static str>,
    message: String,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        Self {
            frames: SPINNER_FRAMES.to_vec(),
            message: message.to_string(),
        }
    }

    pub fn with_message(message: &str) -> Self {
        Self::new(message)
    }

    pub fn frames() -> &'static [&'static str] {
        SPINNER_FRAMES
    }

    pub fn frame_at(&self, index: usize) -> &str {
        self.frames[index % self.frames.len()]
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_has_10_frames() {
        assert_eq!(Spinner::frames().len(), 10);
    }

    #[test]
    fn frame_at_wraps_around() {
        let spinner = Spinner::new("loading");
        assert_eq!(spinner.frame_at(0), spinner.frame_at(10));
        assert_eq!(spinner.frame_at(3), spinner.frame_at(13));
    }

    #[test]
    fn with_message_stores_message() {
        let spinner = Spinner::with_message("thinking");
        assert_eq!(spinner.message(), "thinking");
    }
}
