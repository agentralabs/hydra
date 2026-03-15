package theme

import "github.com/charmbracelet/lipgloss"

// Frame styles
var (
	FrameBorder = lipgloss.NewStyle().
			BorderStyle(lipgloss.RoundedBorder()).
			BorderForeground(HydraBlue)

	FrameTitle = lipgloss.NewStyle().
			Foreground(HydraBlue).
			Bold(true)

	FrameUsername = lipgloss.NewStyle().
			Foreground(HydraCyan).
			Bold(true)

	FrameModel = lipgloss.NewStyle().
			Foreground(HydraPurple)

	FrameGitBranch = lipgloss.NewStyle().
			Foreground(HydraGreen)
)

// Chat styles
var (
	UserLabel = lipgloss.NewStyle().
			Foreground(HydraCyan).
			Bold(true)

	AssistantMsg = lipgloss.NewStyle()

	SystemMsg = lipgloss.NewStyle().
			Foreground(HydraDim)

	SectionHeader = lipgloss.NewStyle().
			Foreground(HydraBlue).
			Bold(true)
)

// Input styles
var (
	InputPrompt = lipgloss.NewStyle().
			Foreground(HydraCyan).
			Bold(true)

	InputHint = lipgloss.NewStyle().
			Foreground(HydraDim)

	InputBorderActive = lipgloss.NewStyle().
				BorderStyle(lipgloss.RoundedBorder()).
				BorderForeground(HydraBlue)

	InputBorderDisabled = lipgloss.NewStyle().
				BorderStyle(lipgloss.RoundedBorder()).
				BorderForeground(HydraBorder)
)

// Tool result styles
var (
	ToolSisterName = lipgloss.NewStyle().
			Foreground(HydraCyan)

	ToolDuration = lipgloss.NewStyle().
			Foreground(HydraDim)

	ToolConnector = lipgloss.NewStyle().
			Foreground(HydraDim)
)

// Diff styles
var (
	DiffRemoved = lipgloss.NewStyle().
			Background(DiffRedBG)

	DiffAdded = lipgloss.NewStyle().
			Background(DiffGreenBG)

	DiffLineNumber = lipgloss.NewStyle().
			Foreground(HydraDim)
)

// Status indicators
var (
	StatusOk   = lipgloss.NewStyle().Foreground(HydraGreen)
	StatusWarn = lipgloss.NewStyle().Foreground(HydraYellow)
	StatusErr  = lipgloss.NewStyle().Foreground(HydraRed)
	Dim        = lipgloss.NewStyle().Foreground(HydraDim)
	Bold       = lipgloss.NewStyle().Bold(true)
)

// Briefing
var (
	BriefingUrgent    = lipgloss.NewStyle().Foreground(HydraRed)
	BriefingImportant = lipgloss.NewStyle().Foreground(HydraYellow)
	BriefingInfo      = lipgloss.NewStyle().Foreground(HydraDim)
)

// Streaming
var (
	StreamingIndicator = lipgloss.NewStyle().Foreground(HydraPurple)
)

// Approval risk colors
func RiskStyle(level string) lipgloss.Style {
	switch level {
	case "LOW":
		return lipgloss.NewStyle().Foreground(HydraGreen)
	case "MEDIUM":
		return lipgloss.NewStyle().Foreground(HydraYellow)
	case "HIGH":
		return lipgloss.NewStyle().Foreground(HydraOrange)
	case "CRITICAL":
		return lipgloss.NewStyle().Foreground(HydraRed)
	default:
		return Dim
	}
}
