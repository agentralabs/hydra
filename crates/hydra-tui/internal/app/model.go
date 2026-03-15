package app

import (
	"os"
	"os/exec"
	"os/user"
	"path/filepath"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/viewport"

	"github.com/agentralabs/hydra-tui/internal/client"
)

// Mode represents the current UI mode.
type Mode int

const (
	ModeBoot Mode = iota
	ModeOnboarding
	ModeChat
	ModeStreaming
	ModeApproval
)

// PermissionMode for action approval.
type PermissionMode int

const (
	PermNormal PermissionMode = iota
	PermAutoAccept
	PermPlan
)

// Model is the top-level Bubble Tea model.
type Model struct {
	// Connection
	Client   *client.HydraRpcClient
	Stream   *client.StreamReceiver
	Connected bool

	// Mode
	Mode          Mode
	PermMode      PermissionMode

	// Frame info
	Version       string
	Username      string
	ModelName     string
	ProviderName  string
	GitBranch     string
	ProjectPath   string
	CrateCount    uint32
	ProfileName   string
	BeliefsLoaded uint32
	SistersConn   uint32
	SistersTotal  uint32
	ToolsCount    uint32
	HealthPct     float64
	SessionCost   float64
	TokensUsed    uint64
	DreamNew      uint32
	Online        bool

	// Chat
	Messages  []client.ChatMessage
	Viewport  viewport.Model
	ChatLines []string // rendered chat lines

	// Streaming
	StreamBuf     string
	RevealedChars int
	StreamActive  bool
	StreamRunID   string
	StreamSpeed   float64

	// Input
	Input       string
	CursorPos   int
	History     []string
	HistoryIdx  int
	InputEnabled bool
	Multiline   bool

	// Autocomplete
	Suggestions []string
	SelIdx      int
	ShowAC      bool

	// Briefing
	BriefingItems     []client.BriefingItem
	BriefingDismissed bool

	// /btw
	BtwActive   bool
	BtwResponse string

	// Boot
	BootProgress float64
	BootStage    string
	BootComplete bool
	BootStart    time.Time
	BootError    string

	// Thinking
	Thinking      bool
	ThinkVerb     string
	SpinnerPhase  int
	ThinkStart    time.Time

	// Terminal
	Width  int
	Height int

	// Background tasks
	Tasks     []client.BackgroundTask
	ShowTasks bool

	// Approval
	PendingApproval *client.PendingApproval
	AutoApprove     bool

	// Session changes
	Changes []client.FileChange

	// Memory capture mode
	MemoryMode string // "all", "facts", "none"

	// Settings
	SidebarVisible bool
	VimMode        bool
	DebugMode      bool
	FastMode       bool

	// Onboarding
	Onboarding OnboardingState

	// Thinking display (live stats)
	ThinkTokens  uint64
	ThinkCost    float64
	ThinkTip     string
	ThinkTipIdx  int
	ThinkTipTick int

	// Server auto-start
	ServerStarted bool

	// Misc
	TickCount   int
	LastSubmit  int
	ReceiptCount int
	MemoryFacts  int
}

// NewModel creates the initial model, loading configs from ~/.hydra/.
func NewModel() Model {
	c := client.NewRpcClient()
	u, _ := user.Current()
	username := "User"
	if u != nil {
		username = u.Username
	}

	m := Model{
		Client:       c,
		Version:      "0.2.0",
		Username:     username,
		ModelName:    "sonnet-4-6",
		ProviderName: "anthropic",
		GitBranch:    detectGitBranch(),
		ProjectPath:  detectProjectPath(),
		SistersTotal: 17,
		StreamSpeed:  1.0,
		HistoryIdx:   -1,
		InputEnabled: true,
		BootStart:    time.Now(),
		BootStage:    "Loading configuration...",
		Mode:         ModeBoot,
		MemoryMode:   "all",
		Width:        120,
		Height:       40,
	}

	// Load persisted profile from ~/.hydra/profile.json
	if profile, err := LoadProfile(); err == nil {
		ApplyProfile(&m, profile)
		if !profile.OnboardingComplete {
			m.Mode = ModeOnboarding
			m.Onboarding = NewOnboarding()
		}
	} else {
		// No profile — trigger onboarding
		m.Mode = ModeOnboarding
		m.Onboarding = NewOnboarding()
	}

	// Load project-level settings from .hydra/settings.json
	settings := LoadSettings()
	ApplySettings(&m, settings)

	// Detect project type
	if proj := DetectProject(); proj != nil {
		m.CrateCount = uint32(proj.CrateCount)
		m.GitBranch = proj.GitBranch
	}

	return m
}

// Init is the Bubble Tea init function.
func (m Model) Init() tea.Cmd {
	return tea.Batch(tickCmd(), tea.WindowSize())
}

// IsWide returns true if terminal >= 120 cols.
func (m Model) IsWide() bool { return m.Width >= 120 }

// IsMedium returns true if 60-119 cols.
func (m Model) IsMedium() bool { return m.Width >= 60 && m.Width < 120 }

// IsNarrow returns true if < 60 cols.
func (m Model) IsNarrow() bool { return m.Width < 60 }

// FrameHeight based on responsive breakpoint.
func (m Model) FrameHeight() int {
	if m.IsNarrow() {
		return 6
	} else if m.IsMedium() {
		return 10
	}
	return 16 // enough for logo + all sections
}

// tickMsg is sent every 33ms (~30fps).
type tickMsg time.Time

func tickCmd() tea.Cmd {
	return tea.Tick(33*time.Millisecond, func(t time.Time) tea.Msg {
		return tickMsg(t)
	})
}

// streamChunkMsg wraps an SSE chunk as a Bubble Tea message.
type streamChunkMsg client.StreamChunk

func detectGitBranch() string {
	out, err := exec.Command("git", "rev-parse", "--abbrev-ref", "HEAD").Output()
	if err != nil {
		return "—"
	}
	return strings.TrimSpace(string(out))
}

func detectProjectPath() string {
	dir, err := os.Getwd()
	if err != nil {
		return "."
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return dir
	}
	if strings.HasPrefix(dir, home) {
		return "~" + dir[len(home):]
	}
	return dir
}

func detectCrateCount() uint32 {
	// Count directories in crates/
	entries, err := os.ReadDir(filepath.Join("crates"))
	if err != nil {
		return 0
	}
	var count uint32
	for _, e := range entries {
		if e.IsDir() {
			count++
		}
	}
	return count
}
