package app

import (
	"math/rand"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"

	"github.com/agentralabs/hydra-tui/internal/client"
)

// Update is the Bubble Tea update function.
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.Width = msg.Width
		m.Height = msg.Height
		return m, nil

	case tickMsg:
		return m.handleTick()

	case streamChunkMsg:
		m.handleStreamChunk(client.StreamChunk(msg))
		return m, nil

	case tea.KeyMsg:
		return m.handleKey(msg)
	}

	return m, nil
}

func (m Model) handleTick() (tea.Model, tea.Cmd) {
	m.TickCount++

	// Boot tick
	if m.Mode == ModeBoot {
		TickBoot(&m)
		if m.BootComplete {
			m.Mode = ModeChat
			m.InputEnabled = true
			if m.Connected {
				m.Stream = client.NewStreamReceiver(m.Client.BaseURL)
			}
		}
		return m, tickCmd()
	}

	// SSE drain
	if m.Stream != nil {
		for _, chunk := range m.Stream.Drain() {
			m.handleStreamChunk(chunk)
		}
	}

	// Periodic health poll every ~10s (300 ticks at 33ms)
	if m.TickCount%300 == 0 {
		m.refreshHealth()
	}

	// Streaming reveal
	if m.StreamActive && len(m.StreamBuf) > m.RevealedChars {
		chars := int(3.0 * m.StreamSpeed)
		if chars < 1 {
			chars = 1
		}
		m.RevealedChars += chars
		if m.RevealedChars > len(m.StreamBuf) {
			m.RevealedChars = len(m.StreamBuf)
		}
	}

	// Auto-collapse tool results after 3 seconds
	now := time.Now()
	for i := range m.Messages {
		for j := range m.Messages[i].ToolResults {
			tr := &m.Messages[i].ToolResults[j]
			if tr.Expanded && !tr.ExpandedAt.IsZero() && now.Sub(tr.ExpandedAt) > 3*time.Second {
				tr.Expanded = false
			}
		}
	}

	// Thinking spinner + tip rotation
	if m.Thinking {
		m.SpinnerPhase++
		// Rotate tip every ~15s (450 ticks at 33ms)
		m.ThinkTipTick++
		if m.ThinkTipTick%450 == 0 || m.ThinkTip == "" {
			tips := []string{
				"Use /btw to ask a quick side question without interrupting",
				"Use /beliefs to see what Hydra knows",
				"Use /roi to track value generated",
				"Dream State will test your beliefs overnight",
				"Use /undo to revert the last edit",
				"/compact preserves key decisions, frees context",
				"/profile load <name> switches expertise domains",
				"Ctrl+T shows background tasks",
				"Ctrl+B pushes current work to background",
				"Press Esc to cancel if this takes too long",
			}
			m.ThinkTipIdx = (m.ThinkTipIdx + 1) % len(tips)
			m.ThinkTip = tips[m.ThinkTipIdx]
		}
	}

	return m, tickCmd()
}

func (m Model) handleKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	// Global
	switch msg.String() {
	case "ctrl+c":
		return m, tea.Quit
	}

	switch m.Mode {
	case ModeBoot:
		return m.handleBootKey(msg)
	case ModeOnboarding:
		return m.handleOnboardingKey(msg)
	case ModeChat:
		return m.handleChatKey(msg)
	case ModeStreaming:
		return m.handleStreamingKey(msg)
	case ModeApproval:
		return m.handleApprovalKey(msg)
	}
	return m, nil
}

func (m Model) handleOnboardingKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	quit, profile := OnboardingUpdate(&m.Onboarding, msg)
	if quit {
		return m, tea.Quit
	}
	if profile != nil {
		ApplyProfile(&m, profile)
		m.Mode = ModeBoot
		m.BootStart = time.Now()
	}
	return m, nil
}

func (m Model) handleBootKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "enter", "esc":
		m.BootProgress = 100
		m.BootComplete = true
		m.Mode = ModeChat
		m.InputEnabled = true
	}
	return m, nil
}

func (m Model) handleChatKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	// Autocomplete visible
	if m.ShowAC {
		switch msg.String() {
		case "tab":
			if m.SelIdx >= 0 && m.SelIdx < len(m.Suggestions) {
				m.Input = "/" + m.Suggestions[m.SelIdx]
				m.CursorPos = len(m.Input)
				m.ShowAC = false
			}
			return m, nil
		case "down":
			m.SelIdx = (m.SelIdx + 1) % len(m.Suggestions)
			return m, nil
		case "up":
			m.SelIdx--
			if m.SelIdx < 0 {
				m.SelIdx = len(m.Suggestions) - 1
			}
			return m, nil
		case "esc":
			m.ShowAC = false
			return m, nil
		}
		m.ShowAC = false
	}

	switch msg.String() {
	// Scroll
	case "pgup", "ctrl+u":
		// scroll up
	case "pgdown", "ctrl+d":
		// scroll down
	case "home":
		// scroll top
	case "end":
		// scroll bottom

	// Ctrl shortcuts
	case "ctrl+t":
		m.ShowTasks = !m.ShowTasks
	case "ctrl+b":
		m.pushToBackground()
	case "ctrl+f":
		m.killBackgroundTasks()
	case "ctrl+o":
		m.toggleToolExpand()
	case "ctrl+s":
		m.SidebarVisible = !m.SidebarVisible
	case "shift+tab":
		// Cycle permission mode
		m.PermMode = (m.PermMode + 1) % 3

	// Input editing
	case "enter":
		return m.handleSubmit()
	case "shift+enter":
		m.insertChar('\n')
	case "backspace":
		m.backspace()
		m.updateAutocomplete()
	case "delete":
		m.deleteChar()
	case "left":
		if m.CursorPos > 0 {
			m.CursorPos--
		}
	case "right":
		if m.CursorPos < len(m.Input) {
			m.CursorPos++
		}
	case "up":
		m.historyUp()
	case "down":
		m.historyDown()
	case "ctrl+a":
		m.CursorPos = 0
	case "ctrl+e":
		m.CursorPos = len(m.Input)
	case "ctrl+k":
		m.Input = m.Input[:m.CursorPos]
	case "ctrl+shift+u":
		m.Input = m.Input[m.CursorPos:]
		m.CursorPos = 0
	case "ctrl+w":
		m.killWordBackward()
	case "ctrl+l":
		m.Messages = nil
		m.ChatLines = nil
	case "tab":
		m.updateAutocomplete()

	case "esc":
		if m.BtwActive {
			m.BtwActive = false
		} else if m.Input != "" {
			m.Input = ""
			m.CursorPos = 0
		}

	case " ":
		if !m.BriefingDismissed && len(m.BriefingItems) > 0 {
			m.BriefingDismissed = true
		} else {
			m.insertChar(' ')
			m.updateAutocomplete()
		}

	default:
		if len(msg.String()) == 1 {
			m.insertChar(rune(msg.String()[0]))
			if !m.BriefingDismissed {
				m.BriefingDismissed = true
			}
			m.updateAutocomplete()
		}
	}

	return m, nil
}

func (m Model) handleStreamingKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "esc":
		m.StreamActive = false
		m.Thinking = false
		m.Mode = ModeChat
		m.InputEnabled = true
		if m.StreamRunID != "" {
			_ = m.Client.Cancel(m.StreamRunID)
		}
	default:
		if len(msg.String()) == 1 {
			m.StreamSpeed = 5.0
			m.insertChar(rune(msg.String()[0]))
		}
	}
	return m, nil
}

func (m Model) handleApprovalKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	if m.PendingApproval == nil {
		m.Mode = ModeChat
		return m, nil
	}
	switch msg.String() {
	case "y", "Y", "enter":
		_ = m.Client.Approve(m.PendingApproval.RunID, "approved")
		m.PendingApproval = nil
		m.Mode = ModeChat
		m.addSystemMsg("Approved.")
	case "n", "N", "esc":
		_ = m.Client.Approve(m.PendingApproval.RunID, "denied")
		m.PendingApproval = nil
		m.Mode = ModeChat
		m.addSystemMsg("Denied.")
	case "a", "A":
		m.AutoApprove = true
		_ = m.Client.Approve(m.PendingApproval.RunID, "approved")
		m.PendingApproval = nil
		m.Mode = ModeChat
		m.addSystemMsg("Approved (auto-approving for session).")
	}
	return m, nil
}

func (m Model) handleSubmit() (tea.Model, tea.Cmd) {
	input := m.Input
	if len(input) == 0 {
		return m, nil
	}
	m.Input = ""
	m.CursorPos = 0
	m.HistoryIdx = -1

	// Add to history
	if len(m.History) == 0 || m.History[len(m.History)-1] != input {
		m.History = append(m.History, input)
	}

	// Add user message
	m.Messages = append(m.Messages, client.ChatMessage{
		ID:        time.Now().Format(time.RFC3339Nano),
		Role:      client.RoleUser,
		Content:   input,
		Timestamp: time.Now(),
	})

	// Route
	if input[0] == '/' {
		HandleSlashCommand(&m, input)
		return m, nil
	}
	if input[0] == '!' {
		m.handleBash(input[1:])
		return m, nil
	}

	// Send to server
	m.sendToServer(input)
	return m, nil
}

func (m *Model) sendToServer(input string) {
	m.Mode = ModeStreaming
	m.InputEnabled = false
	m.Thinking = true
	m.ThinkVerb = pickThinkingVerb()
	m.ThinkStart = time.Now()
	m.StreamBuf = ""
	m.RevealedChars = 0
	m.StreamActive = true
	m.StreamSpeed = 1.0

	result, err := m.Client.Run(input)
	if err != nil {
		m.addSystemMsg("Error: " + err.Error())
		m.StreamActive = false
		m.Thinking = false
		m.Mode = ModeChat
		m.InputEnabled = true
		return
	}

	m.StreamRunID = result.RunID
	// If server returned output synchronously, finish immediately
	if result.Output != nil && *result.Output != "" {
		m.StreamBuf = *result.Output
		m.finishResponse()
	}
	// Otherwise, SSE "done" event will call finishResponse()
}

func (m *Model) finishResponse() {
	m.StreamActive = false
	m.Thinking = false
	if m.StreamBuf != "" {
		m.Messages = append(m.Messages, client.ChatMessage{
			ID:        time.Now().Format(time.RFC3339Nano),
			Role:      client.RoleAssistant,
			Content:   m.StreamBuf,
			Timestamp: time.Now(),
		})
	}
	m.Mode = ModeChat
	m.InputEnabled = true
}

func (m *Model) handleStreamChunk(chunk client.StreamChunk) {
	switch chunk.Type {
	case "text":
		if chunk.Content != nil {
			m.StreamBuf += *chunk.Content
			if !m.StreamActive {
				m.StreamActive = true
				m.Mode = ModeStreaming
				m.InputEnabled = false
			}
		}
	case "thinking", "tool_start":
		m.Thinking = true
		if chunk.Sister != nil {
			m.ThinkVerb = pickSisterVerb(*chunk.Sister)
		}
	case "tool_end":
		// tool results handled inline
	case "done":
		m.Thinking = false
		m.finishResponse()
	case "error":
		m.Thinking = false
		m.StreamActive = false
		content := "Unknown error"
		if chunk.Content != nil {
			content = *chunk.Content
		}
		m.addSystemMsg("Error: " + content)
		m.Mode = ModeChat
		m.InputEnabled = true
	}
}

func (m *Model) handleBash(cmd string) {
	// Execute in background would be better, but for now sync
	m.addSystemMsg("$ " + cmd + "\n(Bash execution via hydra-server)")
}

func (m *Model) addSystemMsg(content string) {
	m.Messages = append(m.Messages, client.ChatMessage{
		ID:        time.Now().Format(time.RFC3339Nano),
		Role:      client.RoleSystem,
		Content:   content,
		Timestamp: time.Now(),
	})
}

func (m *Model) addAssistantMsg(content string) {
	m.Messages = append(m.Messages, client.ChatMessage{
		ID:        time.Now().Format(time.RFC3339Nano),
		Role:      client.RoleAssistant,
		Content:   content,
		Timestamp: time.Now(),
	})
}

// Input helpers
func (m *Model) insertChar(c rune) {
	if !m.InputEnabled {
		return
	}
	m.Input = m.Input[:m.CursorPos] + string(c) + m.Input[m.CursorPos:]
	m.CursorPos++
}

func (m *Model) backspace() {
	if m.CursorPos > 0 {
		m.Input = m.Input[:m.CursorPos-1] + m.Input[m.CursorPos:]
		m.CursorPos--
	}
}

func (m *Model) deleteChar() {
	if m.CursorPos < len(m.Input) {
		m.Input = m.Input[:m.CursorPos] + m.Input[m.CursorPos+1:]
	}
}

func (m *Model) killWordBackward() {
	if m.CursorPos == 0 {
		return
	}
	i := m.CursorPos - 1
	for i > 0 && m.Input[i] == ' ' {
		i--
	}
	for i > 0 && m.Input[i] != ' ' {
		i--
	}
	m.Input = m.Input[:i] + m.Input[m.CursorPos:]
	m.CursorPos = i
}

func (m *Model) historyUp() {
	if len(m.History) == 0 {
		return
	}
	if m.HistoryIdx == -1 {
		m.HistoryIdx = len(m.History) - 1
	} else if m.HistoryIdx > 0 {
		m.HistoryIdx--
	}
	m.Input = m.History[m.HistoryIdx]
	m.CursorPos = len(m.Input)
}

func (m *Model) historyDown() {
	if m.HistoryIdx == -1 {
		return
	}
	if m.HistoryIdx >= len(m.History)-1 {
		m.HistoryIdx = -1
		m.Input = ""
		m.CursorPos = 0
	} else {
		m.HistoryIdx++
		m.Input = m.History[m.HistoryIdx]
		m.CursorPos = len(m.Input)
	}
}

func (m *Model) toggleToolExpand() {
	if len(m.Messages) > 0 {
		last := &m.Messages[len(m.Messages)-1]
		if len(last.ToolResults) > 0 {
			tr := &last.ToolResults[len(last.ToolResults)-1]
			tr.Expanded = !tr.Expanded
			if tr.Expanded {
				tr.ExpandedAt = time.Now()
			}
		}
	}
}

func (m *Model) refreshHealth() {
	if !m.Connected {
		m.Connected = m.Client.HealthCheck()
		if !m.Connected {
			m.Online = false
			return
		}
		if m.Stream == nil {
			m.Stream = client.NewStreamReceiver(m.Client.BaseURL)
		}
	}
	health, err := m.Client.Health()
	if err != nil {
		m.Connected = false
		m.Online = false
		return
	}
	m.SistersConn = health.SistersConnected
	if health.SistersTotal > 0 {
		m.SistersTotal = health.SistersTotal
	}
	m.BeliefsLoaded = health.BeliefsLoaded
	if health.ToolsCount > 0 {
		m.ToolsCount = health.ToolsCount
	}
	total := m.SistersTotal
	if total == 0 {
		total = 17
	}
	m.HealthPct = float64(m.SistersConn) / float64(total) * 100
	if health.Model != nil && *health.Model != "" {
		m.ModelName = *health.Model
	}
	if health.Profile != nil && *health.Profile != "" {
		m.ProfileName = *health.Profile
	}
	m.Online = m.SistersConn > 0
}

func (m *Model) pushToBackground() {
	if m.StreamActive {
		m.Tasks = append(m.Tasks, client.BackgroundTask{
			ID:     m.StreamRunID,
			Name:   "Background task",
			Status: client.TaskRunning,
		})
		m.StreamActive = false
		m.Thinking = false
		m.Mode = ModeChat
		m.InputEnabled = true
		m.addSystemMsg("Task pushed to background. Ctrl+T to see status.")
	}
}

func (m *Model) killBackgroundTasks() {
	count := len(m.Tasks)
	for _, t := range m.Tasks {
		_ = m.Client.Cancel(t.ID)
	}
	m.Tasks = nil
	if count > 0 {
		m.addSystemMsg("Killed all background tasks.")
	}
}

func (m *Model) updateAutocomplete() {
	if len(m.Input) < 1 || m.Input[0] != '/' || strings.Contains(m.Input, " ") {
		m.ShowAC = false
		return
	}
	prefix := m.Input[1:] // empty for just "/"
	m.Suggestions = nil
	for _, cmd := range allCommands {
		if prefix == "" || (len(cmd) >= len(prefix) && cmd[:len(prefix)] == prefix) {
			m.Suggestions = append(m.Suggestions, cmd)
		}
	}
	m.ShowAC = len(m.Suggestions) > 0
	m.SelIdx = 0
}

func pickThinkingVerb() string {
	verbs := []string{
		"Thinking", "Cogitating", "Pondering", "Noodling", "Ruminating",
		"Percolating", "Musing", "Deliberating", "Mulling", "Synthesizing",
		"Cerebrating", "Ideating", "Hydrating", "Concocting", "Brewing",
		"Simmering", "Fermenting", "Crystallizing", "Weaving", "Contemplating",
		"Spelunking", "Tinkering", "Unravelling", "Architecting", "Manifesting",
		"Choreographing", "Transmuting", "Harmonizing", "Gallivanting",
		"Flibbertigibbeting", "Discombobulating", "Combobulating",
		"Bootstrapping", "Moonwalking", "Canoodling", "Bloviating",
	}
	return verbs[rand.Intn(len(verbs))]
}

func pickSisterVerb(sister string) string {
	switch sister {
	case "Memory":
		return "Remembering"
	case "Codebase":
		return "Scanning"
	case "Data":
		return "Crunching"
	case "Connect":
		return "Reaching"
	case "Forge":
		return "Forging"
	case "Workflow":
		return "Orchestrating"
	case "Veritas":
		return "Verifying"
	case "Aegis":
		return "Shielding"
	case "Evolve":
		return "Crystallizing"
	case "Vision":
		return "Perceiving"
	case "Identity":
		return "Authenticating"
	case "Time":
		return "Scheduling"
	case "Contract":
		return "Reviewing"
	case "Planning":
		return "Strategizing"
	case "Cognition":
		return "Modeling"
	case "Reality":
		return "Probing"
	case "Comm":
		return "Dispatching"
	default:
		return "Thinking"
	}
}

var allCommands = []string{
	"help", "clear", "compact", "cost", "model", "health", "status",
	"profile", "profile list", "profile load", "profile unload", "profile info",
	"profile show", "profile create", "profile export", "profile validate",
	"profile beliefs", "profile skills",
	"beliefs", "skills", "roi", "knowledge",
	"undo", "changes", "btw", "voice", "diagnostics", "fast",
	"memory", "memory all", "memory facts", "memory none",
	"version", "env", "dream", "obstacles", "threat", "autonomy",
	"trust", "receipts",
	"sisters", "sister", "fix", "scan", "repair", "stats",
	"files", "open", "edit", "search", "symbols", "impact",
	"diff", "git", "test", "build", "run", "lint", "fmt", "deps",
	"bench", "doc", "deploy", "init",
	"history", "resume", "continue", "fork", "rewind", "rename",
	"export", "context", "copy", "tokens", "usage",
	"agents", "commands", "plan", "bashes", "tasks",
	"config", "doctor", "sidebar", "vim", "theme",
	"terminal-setup", "login", "logout", "keybindings",
	"mcp", "ide", "hooks", "plugin", "remote", "remote-control",
	"ssh", "ssh-exec", "ssh-upload", "ssh-download", "ssh-disconnect", "ssh-list",
	"swarm", "swarm spawn", "swarm status", "swarm assign",
	"swarm results", "swarm kill", "swarm kill-all", "swarm scale",
	"improve-sister",
	"email", "email-setup",
	"implement", "log", "debug",
	"approve", "deny", "kill",
	"quit", "exit", "q",
}
