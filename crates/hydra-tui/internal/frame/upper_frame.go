package frame

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/agentralabs/hydra-tui/internal/theme"
)

// FrameData holds data for rendering the upper frame.
type FrameData struct {
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
	MemoryMode    string
	PermMode      int // 0=Normal, 1=AutoAccept, 2=Plan
	Width         int
}

// RenderUpperFrame renders the pinned upper frame.
func RenderUpperFrame(d FrameData) string {
	w := d.Width - 2
	if w < 20 {
		w = 78
	}

	sisterColor := theme.HealthColor(float64(d.SistersConn) / float64(maxU(d.SistersTotal, 1)) * 100)
	healthColor := theme.HealthColor(d.HealthPct)

	// Left column
	var left strings.Builder
	left.WriteString(theme.Dim.Render("Welcome back, ") + theme.FrameUsername.Render(d.Username) + "!\n\n")
	left.WriteString(theme.FrameModel.Render(d.ModelName) +
		theme.Dim.Render(" ("+d.ProviderName+") · ") +
		theme.FrameGitBranch.Render(d.GitBranch) + "\n")
	left.WriteString(theme.Dim.Render(d.ProjectPath) + "\n")
	if d.CrateCount > 0 {
		left.WriteString(theme.Dim.Render(fmt.Sprintf("%d crates · ", d.CrateCount)))
	}
	profile := d.ProfileName
	if profile == "" {
		profile = "no profile"
	}
	left.WriteString(lipgloss.NewStyle().Foreground(theme.HydraCyan).Render(profile) + "\n")
	left.WriteString(theme.Dim.Render(fmt.Sprintf("/memory %s · $%.3f · %dK tokens",
		d.MemoryMode, d.SessionCost, d.TokensUsed/1000)))

	// Right column: stats
	var right strings.Builder
	right.WriteString(theme.SectionHeader.Render("Session") + "\n")
	right.WriteString(theme.Dim.Render("Sisters    ") +
		lipgloss.NewStyle().Foreground(sisterColor).Render(
			fmt.Sprintf("%d/%d  ●", d.SistersConn, d.SistersTotal)) + "\n")
	right.WriteString(theme.Dim.Render("Tools      ") + fmt.Sprintf("%d\n", d.ToolsCount))
	right.WriteString(theme.Dim.Render("Health     ") +
		lipgloss.NewStyle().Foreground(healthColor).Render(
			fmt.Sprintf("%.0f%%", d.HealthPct)) + "\n")
	right.WriteString(theme.Dim.Render("Beliefs    ") +
		lipgloss.NewStyle().Foreground(theme.HydraCyan).Render(
			fmt.Sprintf("%d", d.BeliefsLoaded)) + "\n")

	modeColor := theme.HydraGreen
	modeStr := "Online"
	if !d.Online {
		modeColor = theme.HydraRed
		modeStr = "Local"
	}
	right.WriteString(theme.Dim.Render("Mode       ") +
		lipgloss.NewStyle().Foreground(modeColor).Render("● "+modeStr) + "\n")

	if d.DreamNew > 0 {
		right.WriteString(theme.Dim.Render("Dream      ") +
			lipgloss.NewStyle().Foreground(theme.HydraPurple).Render(
				fmt.Sprintf("%d new", d.DreamNew)) + "\n")
	}

	permLabels := []string{"Normal", "AutoAccept", "Plan"}
	right.WriteString(theme.Dim.Render("Perm       ") + permLabels[d.PermMode%3])

	leftCol := lipgloss.NewStyle().Width(w * 55 / 100).Render(left.String())
	rightCol := lipgloss.NewStyle().Width(w * 45 / 100).Render(right.String())
	content := lipgloss.JoinHorizontal(lipgloss.Top, leftCol, rightCol)

	border := lipgloss.NewStyle().
		BorderStyle(lipgloss.RoundedBorder()).
		BorderForeground(theme.HydraBlue).
		Width(w)

	return border.Render(content)
}

func maxU(a, b uint32) uint32 {
	if a > b {
		return a
	}
	return b
}
