package chat

// ScrollState manages virtual scroll for infinite chat history.
type ScrollState struct {
	Offset      int  // offset from bottom (0 = at latest)
	TotalLines  int
	ViewHeight  int
	AutoScroll  bool
	NewMsgShown bool // "↓ New message" indicator
}

// NewScrollState creates a new scroll state.
func NewScrollState() ScrollState {
	return ScrollState{AutoScroll: true, ViewHeight: 20}
}

// ScrollUp scrolls up by n lines.
func (s *ScrollState) ScrollUp(n int) {
	maxOffset := s.TotalLines - s.ViewHeight
	if maxOffset < 0 {
		maxOffset = 0
	}
	s.Offset += n
	if s.Offset > maxOffset {
		s.Offset = maxOffset
	}
	s.AutoScroll = false
}

// ScrollDown scrolls down by n lines.
func (s *ScrollState) ScrollDown(n int) {
	s.Offset -= n
	if s.Offset <= 0 {
		s.Offset = 0
		s.AutoScroll = true
		s.NewMsgShown = false
	}
}

// PageUp scrolls up half a page.
func (s *ScrollState) PageUp() { s.ScrollUp(s.ViewHeight / 2) }

// PageDown scrolls down half a page.
func (s *ScrollState) PageDown() { s.ScrollDown(s.ViewHeight / 2) }

// JumpToTop jumps to the first message.
func (s *ScrollState) JumpToTop() {
	s.Offset = s.TotalLines - s.ViewHeight
	if s.Offset < 0 {
		s.Offset = 0
	}
	s.AutoScroll = false
}

// JumpToBottom jumps to the latest message.
func (s *ScrollState) JumpToBottom() {
	s.Offset = 0
	s.AutoScroll = true
	s.NewMsgShown = false
}

// OnNewContent handles new content arrival.
func (s *ScrollState) OnNewContent(addedLines int) {
	s.TotalLines += addedLines
	if !s.AutoScroll {
		s.NewMsgShown = true
		s.Offset += addedLines
	}
}

// VisibleRange returns the start and end indices of visible lines.
func (s *ScrollState) VisibleRange() (int, int) {
	end := s.TotalLines - s.Offset
	start := end - s.ViewHeight
	if start < 0 {
		start = 0
	}
	if end > s.TotalLines {
		end = s.TotalLines
	}
	return start, end
}
