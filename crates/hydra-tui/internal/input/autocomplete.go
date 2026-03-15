package input

import "strings"

// Autocomplete manages slash command autocomplete.
type Autocomplete struct {
	Suggestions []string
	SugDescs    []string // descriptions for each suggestion
	Selected    int
	Visible     bool
}

// NewAutocomplete creates a new autocomplete engine.
func NewAutocomplete() Autocomplete {
	return Autocomplete{}
}

// Update refreshes suggestions based on current input.
func (a *Autocomplete) Update(input string) {
	if len(input) < 1 || input[0] != '/' || strings.Contains(input, " ") {
		a.Visible = false
		a.Suggestions = nil
		return
	}

	prefix := input[1:] // empty string for just "/"
	a.Suggestions = nil
	a.SugDescs = nil
	for _, cmd := range commandList {
		if prefix == "" || strings.HasPrefix(cmd.Name, prefix) {
			a.Suggestions = append(a.Suggestions, cmd.Name)
			a.SugDescs = append(a.SugDescs, cmd.Desc)
		}
	}

	// Limit visible suggestions to 10
	if len(a.Suggestions) > 10 {
		a.Suggestions = a.Suggestions[:10]
		a.SugDescs = a.SugDescs[:10]
	}

	a.Visible = len(a.Suggestions) > 0
	if a.Visible && a.Selected >= len(a.Suggestions) {
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

// CommandInfo holds a command name and its description.
type CommandInfo struct {
	Name string
	Desc string
}

// All commands with descriptions — shown in autocomplete dropdown.
var commandList = []CommandInfo{
	{"agents", "Manage agent configurations"},
	{"approve", "Approve a pending action"},
	{"bashes", "List background processes"},
	{"beliefs", "Show loaded profile beliefs"},
	{"bench", "Run benchmarks"},
	{"btw", "Ask a side question without interrupting"},
	{"build", "Build the current project"},
	{"changes", "Show file changes this session"},
	{"clear", "Clear conversation history"},
	{"compact", "Compress conversation context"},
	{"config", "Show current configuration"},
	{"copy", "Copy last response to clipboard"},
	{"cost", "Show session cost breakdown"},
	{"debug", "Toggle debug mode"},
	{"deny", "Deny a pending action"},
	{"deploy", "Deploy the current project"},
	{"deps", "List project dependencies"},
	{"diagnostics", "Show system diagnostics"},
	{"diff", "Show git diff"},
	{"doc", "Generate documentation"},
	{"doctor", "Run system health checks"},
	{"dream", "Show Dream State status"},
	{"edit", "Edit a file"},
	{"email", "Draft an email"},
	{"env", "Show environment info"},
	{"export", "Export session to markdown"},
	{"fast", "Toggle fast mode (Haiku)"},
	{"files", "List project files"},
	{"fix", "Repair offline sisters"},
	{"fmt", "Format project code"},
	{"fork", "Fork conversation branch"},
	{"git", "Run git commands"},
	{"goals", "Show active goals"},
	{"health", "System health dashboard"},
	{"help", "Show all available commands"},
	{"history", "Show conversation history"},
	{"hooks", "Show configured hooks"},
	{"ide", "IDE integration status"},
	{"impact", "Show impact analysis for a file"},
	{"improve-sister", "Improve a sister's capabilities"},
	{"init", "Initialize project instructions"},
	{"keybindings", "Show keyboard shortcuts"},
	{"kill", "Kill current operation"},
	{"knowledge", "Show knowledge progress"},
	{"lint", "Lint project code"},
	{"log", "Show recent log entries"},
	{"login", "Set API key"},
	{"mcp", "MCP server management"},
	{"memory", "Set memory capture mode"},
	{"model", "Switch LLM model"},
	{"obstacles", "Show session momentum"},
	{"open", "Open and display a file"},
	{"plan", "Enter plan mode"},
	{"plugin", "Plugin management"},
	{"profile", "Manage operational profiles"},
	{"quit", "Exit Hydra"},
	{"receipts", "Show action receipts"},
	{"remote", "Remote control status"},
	{"rename", "Rename current session"},
	{"repair", "Run self-repair"},
	{"resume", "Resume previous session"},
	{"rewind", "Show changes for selective undo"},
	{"roi", "Show ROI tracking"},
	{"run", "Run the current project"},
	{"scan", "Run omniscience scan"},
	{"search", "Search codebase"},
	{"sidebar", "Toggle sidebar"},
	{"sister", "Show sister details"},
	{"sisters", "Show all sister status"},
	{"skills", "Show loaded profile skills"},
	{"ssh", "SSH to remote host"},
	{"stats", "Show gateway stats"},
	{"status", "Show system status"},
	{"swarm", "Manage agent swarm"},
	{"symbols", "List symbols in a file"},
	{"tasks", "Show background tasks"},
	{"test", "Run project tests"},
	{"theme", "Show current theme"},
	{"threat", "Show threat analysis"},
	{"tokens", "Show token usage"},
	{"trust", "Show trust level"},
	{"undo", "Undo last file change"},
	{"usage", "Show usage stats"},
	{"version", "Show Hydra version"},
	{"vim", "Toggle vim mode"},
	{"voice", "Enable voice input"},
}

// commands is the flat list for backward compat
var commands []string

func init() {
	for _, c := range commandList {
		commands = append(commands, c.Name)
	}
}

// CommandList returns all commands with descriptions.
func CommandList() []CommandInfo {
	return commandList
}
