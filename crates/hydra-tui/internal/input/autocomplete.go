package input

import "strings"

// Autocomplete manages slash command autocomplete.
type Autocomplete struct {
	Suggestions []string
	Selected    int
	Visible     bool
}

// NewAutocomplete creates a new autocomplete engine.
func NewAutocomplete() Autocomplete {
	return Autocomplete{}
}

// Update refreshes suggestions based on current input.
func (a *Autocomplete) Update(input string) {
	if len(input) < 2 || input[0] != '/' || strings.Contains(input, " ") {
		a.Visible = false
		a.Suggestions = nil
		return
	}

	prefix := input[1:]
	a.Suggestions = nil
	for _, cmd := range commands {
		if len(cmd) > len(prefix) && strings.HasPrefix(cmd, prefix) {
			a.Suggestions = append(a.Suggestions, cmd)
		}
	}

	a.Visible = len(a.Suggestions) > 0
	if a.Visible {
		a.Selected = 0
	}
}

// Next selects the next suggestion.
func (a *Autocomplete) Next() {
	if len(a.Suggestions) > 0 {
		a.Selected = (a.Selected + 1) % len(a.Suggestions)
	}
}

// Prev selects the previous suggestion.
func (a *Autocomplete) Prev() {
	if len(a.Suggestions) > 0 {
		a.Selected--
		if a.Selected < 0 {
			a.Selected = len(a.Suggestions) - 1
		}
	}
}

// Accept accepts the current selection.
func (a *Autocomplete) Accept() string {
	if a.Selected >= 0 && a.Selected < len(a.Suggestions) {
		cmd := "/" + a.Suggestions[a.Selected]
		a.Visible = false
		a.Suggestions = nil
		return cmd
	}
	return ""
}

// Dismiss hides autocomplete.
func (a *Autocomplete) Dismiss() {
	a.Visible = false
	a.Suggestions = nil
}

var commands = []string{
	"help", "clear", "compact", "cost", "model", "health", "status",
	"profile", "beliefs", "skills", "roi", "knowledge",
	"undo", "changes", "btw", "voice", "diagnostics", "fast",
	"memory", "version", "env", "dream", "obstacles", "threat",
	"autonomy", "trust", "receipts", "sisters", "sister",
	"stats", "fix", "scan", "repair", "goals",
	"files", "open", "edit", "search", "symbols", "impact",
	"diff", "git", "test", "build", "run", "lint", "fmt",
	"deps", "bench", "doc", "deploy", "init",
	"history", "resume", "fork", "rewind", "rename", "export",
	"context", "copy", "tokens", "usage",
	"agents", "commands", "plan", "bashes", "tasks",
	"config", "doctor", "sidebar", "vim", "theme",
	"keybindings", "mcp", "ide", "hooks", "plugin",
	"remote", "ssh", "swarm", "email",
	"approve", "deny", "kill", "log", "debug",
	"quit", "exit",
}
