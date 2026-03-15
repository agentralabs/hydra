package chat

import "time"

// PacingTier controls rendering speed for different content types.
type PacingTier int

const (
	TierStreamingText PacingTier = iota // 15-20 chars/frame
	TierToolResult                      // progressive reveal
	TierTable                           // row by row (200ms/row)
	TierSummary                         // section by section
	TierError                           // slower for comprehension
	TierMorningBrief                    // priority-paced (500ms hold)
)

// StreamingState manages human-paced output (Pattern 0).
type StreamingState struct {
	Buffer       string
	Revealed     int
	Active       bool
	LastReveal   time.Time
	Speed        float64 // 1.0=normal, 2.0=fast, 0.5=slow
	Tier         PacingTier
	RunID        string
}

// NewStreamingState creates a new streaming state.
func NewStreamingState() StreamingState {
	return StreamingState{Speed: 1.0, LastReveal: time.Now()}
}

// Start begins streaming a new response.
func (s *StreamingState) Start(runID string) {
	s.Buffer = ""
	s.Revealed = 0
	s.Active = true
	s.LastReveal = time.Now()
	s.Speed = 1.0
	s.Tier = TierStreamingText
	s.RunID = runID
}

// Append adds content from an SSE chunk.
func (s *StreamingState) Append(text string) {
	s.Buffer += text
}

// Stop stops streaming.
func (s *StreamingState) Stop() {
	s.Active = false
	s.Revealed = len(s.Buffer)
}

// CharsToReveal returns how many chars to reveal this frame (~30fps).
func (s *StreamingState) CharsToReveal() int {
	if !s.Active {
		return 0
	}

	elapsed := time.Since(s.LastReveal)
	interval := s.baseInterval()
	if elapsed < interval {
		return 0
	}

	s.LastReveal = time.Now()
	remaining := len(s.Buffer) - s.Revealed
	if remaining == 0 {
		return 0
	}

	baseChars := 3 // ~18 chars/frame at 30fps
	switch s.Tier {
	case TierToolResult:
		baseChars = 5
	case TierTable:
		baseChars = 10
	case TierError:
		baseChars = 2
	}

	chars := int(float64(baseChars) * s.Speed)
	if chars < 1 {
		chars = 1
	}
	if chars > remaining {
		chars = remaining
	}

	// Pause at natural boundaries
	segment := s.Buffer[s.Revealed : s.Revealed+chars]
	for _, c := range segment {
		if c == '.' || c == '\n' {
			// Brief pause after sentences/newlines
			break
		}
	}

	s.Revealed += chars
	return chars
}

func (s *StreamingState) baseInterval() time.Duration {
	switch s.Tier {
	case TierStreamingText:
		return 16 * time.Millisecond
	case TierToolResult:
		return 200 * time.Millisecond
	case TierTable:
		return 200 * time.Millisecond
	case TierSummary:
		return 100 * time.Millisecond
	case TierError:
		return 100 * time.Millisecond
	case TierMorningBrief:
		return 300 * time.Millisecond
	}
	return 33 * time.Millisecond
}

// VisibleText returns the currently revealed portion.
func (s *StreamingState) VisibleText() string {
	if s.Revealed > len(s.Buffer) {
		return s.Buffer
	}
	return s.Buffer[:s.Revealed]
}

// HasPending returns true if there's unrevealed content.
func (s *StreamingState) HasPending() bool {
	return s.Revealed < len(s.Buffer)
}

// Accelerate increases speed (user scrolled or typed).
func (s *StreamingState) Accelerate(factor float64) {
	s.Speed *= factor
	if s.Speed > 10 {
		s.Speed = 10
	}
}

// FinishInstantly reveals all remaining content.
func (s *StreamingState) FinishInstantly() {
	s.Revealed = len(s.Buffer)
}
