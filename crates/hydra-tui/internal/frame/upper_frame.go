package frame

import (
	"fmt"
	"os"
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
	ProjectName   string
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
	RecentCommits []string // last 2 git commits
	Width         int
}

// RenderUpperFrame renders the pinned upper frame matching the old Rust TUI exactly.
// Layout: ┌─── Hydra v0.2.0 ──────────────────────────────────┐
//         │ Left: welcome, logo, model    │ Right: tips, activity, system │
//         └─── Agentra Labs ──────────────────────────────────┘
func RenderUpperFrame(d FrameData) string {
	w := d.Width
	if w < 40 {
		w = 80
	}

	bs := lipgloss.NewStyle().Foreground(theme.HydraBlue)
	bb := lipgloss.NewStyle().Foreground(theme.HydraBlue).Bold(true)
	dim := theme.Dim
	cyan := lipgloss.NewStyle().Foreground(theme.HydraCyan)
	cyanB := lipgloss.NewStyle().Foreground(theme.HydraCyan).Bold(true)
	green := lipgloss.NewStyle().Foreground(theme.HydraGreen)
	yellow := lipgloss.NewStyle().Foreground(theme.HydraYellow)
	red := lipgloss.NewStyle().Foreground(theme.HydraRed)
	purple := lipgloss.NewStyle().Foreground(theme.HydraPurple)

	sp := w * 45 / 100 // split point

	// Helper: build a row │ left (padded) │ right (padded) │
	row := func(left, right string) string {
		ll := lipgloss.Width(left)
		rl := lipgloss.Width(right)
		lpad := sp - ll - 1
		if lpad < 0 { lpad = 0 }
		rpad := w - sp - rl - 2
		if rpad < 0 { rpad = 0 }
		return bs.Render("│") + left + strings.Repeat(" ", lpad) +
			bs.Render("│") + right + strings.Repeat(" ", rpad) + bs.Render("│")
	}

	var lines []string

	// ┌─── Hydra v0.2.0 ───────────────────────────────┐
	title := fmt.Sprintf(" Hydra v%s ", d.Version)
	ld := 3
	rd := w - 2 - ld - len(title)
	if rd < 0 { rd = 0 }
	lines = append(lines, bs.Render("┌")+bs.Render(strings.Repeat("─", ld))+
		bb.Render(title)+bs.Render(strings.Repeat("─", rd))+bs.Render("┐"))

	// Welcome | Tips header
	welcome := "      Welcome back " + cyanB.Render(d.Username) + dim.Render("!")
	tips := " " + bb.Render("Tips for getting started")
	lines = append(lines, row(welcome, tips))

	// (empty) | tip lines
	lines = append(lines, row("", " "+dim.Render("/memory all · facts · none to change")))
	lines = append(lines, row("", " "+dim.Render("/init to set up project instructions")))

	// Logo ◉ | separator
	sepW := w - sp - 4
	if sepW > 45 { sepW = 45 }
	lines = append(lines, row("           "+cyan.Render("◉"),
		" "+dim.Render(strings.Repeat("─", sepW))))

	// Logo ╱╲ | Recent activity header
	lines = append(lines, row("         "+bs.Render("╱   ╲"),
		" "+bb.Render("Recent activity")))

	// Logo ◉──◉ | activity 1
	act1 := ""
	if len(d.RecentCommits) > 0 {
		act1 = " " + dim.Render(truncate(d.RecentCommits[0], w-sp-5))
	}
	lines = append(lines, row("        "+bs.Render("◉─────◉"), act1))

	// Logo ╲╱ | activity 2
	act2 := ""
	if len(d.RecentCommits) > 1 {
		act2 = " " + dim.Render(truncate(d.RecentCommits[1], w-sp-5))
	}
	lines = append(lines, row("         "+bs.Render("╲   ╱"), act2))

	// Logo ◉ | separator
	lines = append(lines, row("           "+cyan.Render("◉"),
		" "+dim.Render(strings.Repeat("─", sepW))))

	// (empty) | System header
	lines = append(lines, row("", " "+bb.Render("System")))

	// Model + branch | Sisters
	sisterColor := green
	if d.SistersConn < d.SistersTotal { sisterColor = yellow }
	if d.SistersConn == 0 { sisterColor = red }
	modelLine := "  " + purple.Render(d.ModelName)
	if d.ProviderName != "" {
		modelLine += dim.Render(" ("+d.ProviderName+")")
	}
	if d.GitBranch != "" {
		modelLine += dim.Render(" · ") + green.Render(d.GitBranch)
	}
	lines = append(lines, row(modelLine,
		" "+dim.Render("Sisters    ")+
			lipgloss.NewStyle().Foreground(sisterColor.GetForeground()).Render(
				fmt.Sprintf("%d/%d connected", d.SistersConn, d.SistersTotal))))

	// Path | Tools
	shortPath := shortenPath(d.ProjectPath, sp-5)
	toolsStr := "—"
	if d.ToolsCount > 0 { toolsStr = fmt.Sprintf("%d+", d.ToolsCount) }
	lines = append(lines, row("  "+dim.Render(shortPath),
		" "+dim.Render("Tools      ")+dim.Render(toolsStr)))

	// Project | Health
	healthColor := green
	if d.HealthPct < 90 { healthColor = yellow }
	if d.HealthPct < 50 { healthColor = red }
	projLine := ""
	if d.ProjectName != "" {
		if d.CrateCount > 0 {
			projLine = "  " + bb.Render(fmt.Sprintf("%s (%d crates)", d.ProjectName, d.CrateCount))
		} else {
			projLine = "  " + bb.Render(d.ProjectName)
		}
	}
	lines = append(lines, row(projLine,
		" "+dim.Render("Health     ")+
			lipgloss.NewStyle().Foreground(healthColor.GetForeground()).Render(
				fmt.Sprintf("%.0f%%", d.HealthPct))))

	// Memory mode | Mode
	memActive := d.MemoryMode
	if memActive == "" { memActive = "all" }
	memStyle := green
	if memActive == "none" { memStyle = red }
	modeColor := green
	modeStr := "Local"
	if d.Online { modeStr = "Local" } // Always "Local" for TUI (direct sister connection)
	if !d.Online && d.SistersConn == 0 { modeColor = red; modeStr = "Offline" }
	lines = append(lines, row(
		"  "+dim.Render("/memory ")+
			lipgloss.NewStyle().Foreground(memStyle.GetForeground()).Render(memActive)+
			dim.Render(" · facts · none"),
		" "+dim.Render("Mode       ")+
			lipgloss.NewStyle().Foreground(modeColor.GetForeground()).Render("● "+modeStr)))

	// Empty row
	lines = append(lines, row("", ""))

	// └─── Agentra Labs ───────────────────────────────┘
	brand := " Agentra Labs "
	bld := 3
	brd := w - 2 - bld - len(brand)
	if brd < 0 { brd = 0 }
	lines = append(lines, bs.Render("└")+bs.Render(strings.Repeat("─", bld))+
		dim.Render(brand)+bs.Render(strings.Repeat("─", brd))+bs.Render("┘"))

	return strings.Join(lines, "\n")
}

func truncate(s string, max int) string {
	if len(s) <= max { return s }
	if max < 4 { return s[:max] }
	return s[:max-1] + "…"
}

func shortenPath(path string, max int) string {
	if len(path) <= max { return path }
	// Try replacing home with ~
	if home := homeDir(); home != "" && strings.HasPrefix(path, home) {
		path = "~" + path[len(home):]
	}
	if len(path) <= max { return path }
	return "..." + path[len(path)-max+3:]
}

func homeDir() string {
	if h, ok := lookupEnv("HOME"); ok { return h }
	if h, ok := lookupEnv("USERPROFILE"); ok { return h }
	return ""
}

func lookupEnv(key string) (string, bool) {
	return os.LookupEnv(key)
}

func maxU(a, b uint32) uint32 {
	if a > b { return a }
	return b
}
