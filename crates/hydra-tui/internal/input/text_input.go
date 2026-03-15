package input

// TextInput manages multi-line text input with history.
type TextInput struct {
	Content    string
	CursorPos  int
	History    []string
	HistoryIdx int
	Enabled    bool
	Multiline  bool
}

// NewTextInput creates a new text input.
func NewTextInput() TextInput {
	return TextInput{Enabled: true, HistoryIdx: -1}
}

// InsertChar inserts a character at the cursor position.
func (t *TextInput) InsertChar(c rune) {
	if !t.Enabled {
		return
	}
	t.Content = t.Content[:t.CursorPos] + string(c) + t.Content[t.CursorPos:]
	t.CursorPos++
}

// Backspace removes the character before the cursor.
func (t *TextInput) Backspace() {
	if !t.Enabled || t.CursorPos == 0 {
		return
	}
	t.Content = t.Content[:t.CursorPos-1] + t.Content[t.CursorPos:]
	t.CursorPos--
}

// Delete removes the character after the cursor.
func (t *TextInput) Delete() {
	if !t.Enabled || t.CursorPos >= len(t.Content) {
		return
	}
	t.Content = t.Content[:t.CursorPos] + t.Content[t.CursorPos+1:]
}

// Submit returns the content and clears input. Returns empty string if nothing to submit.
func (t *TextInput) Submit() string {
	if !t.Enabled || len(t.Content) == 0 {
		return ""
	}
	content := t.Content
	t.Content = ""
	t.CursorPos = 0
	t.HistoryIdx = -1

	// Add to history
	if len(t.History) == 0 || t.History[len(t.History)-1] != content {
		t.History = append(t.History, content)
	}

	return content
}

// HistoryUp navigates to the previous history entry.
func (t *TextInput) HistoryUp() {
	if len(t.History) == 0 {
		return
	}
	if t.HistoryIdx == -1 {
		t.HistoryIdx = len(t.History) - 1
	} else if t.HistoryIdx > 0 {
		t.HistoryIdx--
	}
	t.Content = t.History[t.HistoryIdx]
	t.CursorPos = len(t.Content)
}

// HistoryDown navigates to the next history entry.
func (t *TextInput) HistoryDown() {
	if t.HistoryIdx == -1 {
		return
	}
	if t.HistoryIdx >= len(t.History)-1 {
		t.HistoryIdx = -1
		t.Content = ""
		t.CursorPos = 0
	} else {
		t.HistoryIdx++
		t.Content = t.History[t.HistoryIdx]
		t.CursorPos = len(t.Content)
	}
}

// Clear clears the input.
func (t *TextInput) Clear() {
	t.Content = ""
	t.CursorPos = 0
	t.HistoryIdx = -1
}

// IsCommand returns true if the input starts with /.
func (t *TextInput) IsCommand() bool {
	return len(t.Content) > 0 && t.Content[0] == '/'
}

// IsBash returns true if the input starts with !.
func (t *TextInput) IsBash() bool {
	return len(t.Content) > 0 && t.Content[0] == '!'
}
