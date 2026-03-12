//! LLM token streaming support.
//!
//! Provides a buffered token accumulator that batches rapid-fire tokens
//! before flushing to the UI, reducing render churn while maintaining
//! perceived responsiveness.

use std::time::Instant;

/// Configuration for the streaming buffer.
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Minimum milliseconds between flushes.
    pub buffer_ms: u16,
    /// Maximum tokens to accumulate before a forced flush.
    pub max_batch_size: usize,
    /// Whether the UI should auto-scroll to follow new content.
    pub scroll_follow: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            buffer_ms: 16,
            max_batch_size: 64,
            scroll_follow: true,
        }
    }
}

/// Current state of the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Idle,
    Streaming,
    Cancelled,
    Complete,
}

/// A buffer that accumulates streamed tokens and flushes them in batches.
#[derive(Debug)]
pub struct StreamBuffer {
    config: StreamingConfig,
    /// All text accumulated since the stream started (the full output so far).
    accumulated: String,
    /// Tokens received since the last flush.
    pending: Vec<String>,
    /// When the last flush occurred (or when streaming started).
    last_flush: Instant,
    /// Current stream state.
    state: StreamState,
}

impl StreamBuffer {
    /// Create a new buffer with the given configuration.
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config,
            accumulated: String::new(),
            pending: Vec::new(),
            last_flush: Instant::now(),
            state: StreamState::Idle,
        }
    }

    /// Push a new token into the buffer.
    ///
    /// Transitions from `Idle` to `Streaming` on the first token.
    pub fn push_token(&mut self, token: &str) {
        if self.state == StreamState::Idle {
            self.state = StreamState::Streaming;
            self.last_flush = Instant::now();
        }
        if self.state != StreamState::Streaming {
            return;
        }
        self.pending.push(token.to_owned());
    }

    /// Whether the buffer should be flushed.
    ///
    /// Returns `true` if enough time has elapsed (buffer_ms) or the
    /// pending token count has reached `max_batch_size`.
    pub fn should_flush(&self) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        if self.pending.len() >= self.config.max_batch_size {
            return true;
        }
        self.last_flush.elapsed().as_millis() >= self.config.buffer_ms as u128
    }

    /// Flush pending tokens into the accumulated text and return the
    /// newly flushed text.
    pub fn flush(&mut self) -> String {
        let chunk: String = self.pending.drain(..).collect();
        self.accumulated.push_str(&chunk);
        self.last_flush = Instant::now();
        chunk
    }

    /// Cancel the stream, clearing all pending tokens.
    ///
    /// Accumulated text is preserved (the user may want to see partial output).
    pub fn cancel(&mut self) {
        self.pending.clear();
        self.state = StreamState::Cancelled;
    }

    /// Mark the stream as complete, flushing any remaining tokens.
    pub fn complete(&mut self) -> String {
        let remainder = self.flush();
        self.state = StreamState::Complete;
        remainder
    }

    /// Whether the buffer is actively streaming.
    pub fn is_active(&self) -> bool {
        self.state == StreamState::Streaming
    }

    /// The full accumulated text so far.
    pub fn text(&self) -> &str {
        &self.accumulated
    }

    /// The current stream state.
    pub fn stream_state(&self) -> StreamState {
        self.state
    }

    /// Number of pending (unflushed) tokens.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn default_buffer() -> StreamBuffer {
        StreamBuffer::new(StreamingConfig::default())
    }

    #[test]
    fn test_default_config() {
        let cfg = StreamingConfig::default();
        assert_eq!(cfg.buffer_ms, 16);
        assert_eq!(cfg.max_batch_size, 64);
        assert!(cfg.scroll_follow);
    }

    #[test]
    fn test_initial_state_is_idle() {
        let buf = default_buffer();
        assert_eq!(buf.stream_state(), StreamState::Idle);
        assert!(!buf.is_active());
    }

    #[test]
    fn test_push_token_transitions_to_streaming() {
        let mut buf = default_buffer();
        buf.push_token("Hello");
        assert!(buf.is_active());
        assert_eq!(buf.stream_state(), StreamState::Streaming);
    }

    #[test]
    fn test_push_and_flush() {
        let mut buf = default_buffer();
        buf.push_token("Hello");
        buf.push_token(" ");
        buf.push_token("world");
        let flushed = buf.flush();
        assert_eq!(flushed, "Hello world");
        assert_eq!(buf.text(), "Hello world");
    }

    #[test]
    fn test_flush_clears_pending() {
        let mut buf = default_buffer();
        buf.push_token("a");
        buf.push_token("b");
        buf.flush();
        assert_eq!(buf.pending_count(), 0);
        assert!(!buf.should_flush());
    }

    #[test]
    fn test_multiple_flushes_accumulate() {
        let mut buf = default_buffer();
        buf.push_token("Hello");
        buf.flush();
        buf.push_token(" world");
        buf.flush();
        assert_eq!(buf.text(), "Hello world");
    }

    #[test]
    fn test_should_flush_max_batch_size() {
        let cfg = StreamingConfig {
            max_batch_size: 3,
            buffer_ms: 10_000, // very long, so only batch size triggers
            ..Default::default()
        };
        let mut buf = StreamBuffer::new(cfg);
        buf.push_token("a");
        buf.push_token("b");
        assert!(!buf.should_flush());
        buf.push_token("c");
        assert!(buf.should_flush());
    }

    #[test]
    fn test_should_flush_time_elapsed() {
        let cfg = StreamingConfig {
            buffer_ms: 1, // 1ms — will elapse quickly
            max_batch_size: 1000,
            ..Default::default()
        };
        let mut buf = StreamBuffer::new(cfg);
        buf.push_token("a");
        // Sleep just enough for the timer
        thread::sleep(Duration::from_millis(5));
        assert!(buf.should_flush());
    }

    #[test]
    fn test_should_flush_empty_pending_is_false() {
        let mut buf = default_buffer();
        buf.push_token("a");
        buf.flush();
        assert!(!buf.should_flush());
    }

    #[test]
    fn test_cancel() {
        let mut buf = default_buffer();
        buf.push_token("Hello");
        buf.push_token(" world");
        buf.flush();
        buf.push_token(" more");
        buf.cancel();
        assert_eq!(buf.stream_state(), StreamState::Cancelled);
        assert!(!buf.is_active());
        assert_eq!(buf.pending_count(), 0);
        // Accumulated text from before cancel is preserved
        assert_eq!(buf.text(), "Hello world");
    }

    #[test]
    fn test_push_after_cancel_is_noop() {
        let mut buf = default_buffer();
        buf.push_token("before");
        buf.cancel();
        buf.push_token("after");
        assert_eq!(buf.pending_count(), 0);
    }

    #[test]
    fn test_complete() {
        let mut buf = default_buffer();
        buf.push_token("Hello");
        buf.push_token(" world");
        let remainder = buf.complete();
        assert_eq!(remainder, "Hello world");
        assert_eq!(buf.stream_state(), StreamState::Complete);
        assert!(!buf.is_active());
        assert_eq!(buf.text(), "Hello world");
    }

    #[test]
    fn test_complete_flushes_remaining() {
        let mut buf = default_buffer();
        buf.push_token("a");
        buf.flush();
        buf.push_token("b");
        let remainder = buf.complete();
        assert_eq!(remainder, "b");
        assert_eq!(buf.text(), "ab");
    }

    #[test]
    fn test_push_after_complete_is_noop() {
        let mut buf = default_buffer();
        buf.push_token("done");
        buf.complete();
        buf.push_token("extra");
        assert_eq!(buf.pending_count(), 0);
        assert_eq!(buf.text(), "done");
    }

    #[test]
    fn test_stream_state_transitions() {
        let mut buf = default_buffer();
        assert_eq!(buf.stream_state(), StreamState::Idle);
        buf.push_token("x");
        assert_eq!(buf.stream_state(), StreamState::Streaming);
        buf.complete();
        assert_eq!(buf.stream_state(), StreamState::Complete);
    }

    #[test]
    fn test_text_empty_initially() {
        let buf = default_buffer();
        assert_eq!(buf.text(), "");
    }

    #[test]
    fn test_flush_empty_returns_empty() {
        let mut buf = default_buffer();
        let flushed = buf.flush();
        assert_eq!(flushed, "");
    }

    #[test]
    fn test_custom_config() {
        let cfg = StreamingConfig {
            buffer_ms: 100,
            max_batch_size: 128,
            scroll_follow: false,
        };
        let buf = StreamBuffer::new(cfg);
        assert!(!buf.is_active());
        assert_eq!(buf.text(), "");
    }
}
