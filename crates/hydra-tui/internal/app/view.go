package app

import (
	"fmt"
	"os/exec"
	"strings"
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/agentralabs/hydra-tui/internal/client"
	"github.com/agentralabs/hydra-tui/internal/theme"
)

// View is the Bubble Tea view function.
func (m Model) View() string {
	switch m.Mode {
	case ModeOnboarding:
		return OnboardingView(m.Onboarding, m.Width, m.Height)
	case ModeBoot:
		return RenderBootScreen(m)
	default:
		return m.renderChatView()
	}
}

func (m Model) renderChatView() string {
	frame := m.renderUpperFrame()
	frameLines := strings.Count(frame, "\n") + 1
	input := m.renderInputArea()
	inputLines := strings.Count(input, "\n") + 1

	chatHeight := m.Height - frameLines - inputLines
	if chatHeight < 1 {
		chatHeight = 1
	}

	return frame + "\n" + m.renderChatArea(chatHeight) + "\n" + input
}

func (m Model) renderUpperFrame() string {
	w := m.Width
	if w < 20 {
		w = 80
	}
	innerW := w - 4 // 2 for │ borders + 2 for padding

	blue := lipgloss.NewStyle().Foreground(theme.HydraBlue)
	bb := lipgloss.NewStyle().Foreground(theme.HydraBlue).Bold(true)
	cyan := lipgloss.NewStyle().Foreground(theme.HydraCyan)
	dim := theme.Dim
	green := lipgloss.NewStyle().Foreground(theme.HydraGreen)
	purple := lipgloss.NewStyle().Foreground(theme.HydraPurple)

	sisterPct := float64(m.SistersConn) / float64(max(m.SistersTotal, 1)) * 100
	sisterColor := theme.HealthColor(sisterPct)
	healthColor := theme.HealthColor(m.HealthPct)

	if m.IsNarrow() {
		line := fmt.Sprintf("%s · %s  Sisters %d/%d",
			theme.FrameModel.Render(m.ModelName),
			theme.FrameGitBranch.Render(m.GitBranch),
			m.SistersConn, m.SistersTotal)
		return blue.Render("─── ") + theme.FrameTitle.Render("Hydra") + blue.Render(" ───") + "\n" + line
	}

	// === BUILD LEFT COLUMN LINES ===
	var leftLines []string
	leftLines = append(leftLines, fmt.Sprintf("    Welcome back %s!", theme.FrameUsername.Render(m.Username)))
	leftLines = append(leftLines, "")
	// Diamond logo (Hydra node graph) — matches old Rust TUI
	leftLines = append(leftLines, "            "+cyan.Render("◉"))
	leftLines = append(leftLines, "          "+blue.Render("╱")+"   "+blue.Render("╲"))
	leftLines = append(leftLines, "        "+cyan.Render("◉")+blue.Render("───────")+cyan.Render("◉"))
	leftLines = append(leftLines, "          "+blue.Render("╲")+"   "+blue.Render("╱"))
	leftLines = append(leftLines, "            "+cyan.Render("◉"))
	leftLines = append(leftLines, "")
	// Model + provider + branch
	leftLines = append(leftLines, fmt.Sprintf("  %s %s · %s",
		purple.Render(m.ModelName),
		dim.Render("("+capitalize(m.ProviderName)+")"),
		green.Render(m.GitBranch)))
	// Project path (shorten if needed)
	projPath := m.ProjectPath
	if len(projPath) > innerW/2-4 {
		projPath = "..." + projPath[len(projPath)-(innerW/2-7):]
	}
	leftLines = append(leftLines, "  "+dim.Render(projPath))
	// Project name + crate count
	projName := "project"
	if proj := DetectProject(); proj != nil {
		projName = proj.Name
	}
	if m.CrateCount > 0 {
		leftLines = append(leftLines, "  "+cyan.Bold(true).Render(fmt.Sprintf("%s (%d crates)", projName, m.CrateCount)))
	} else {
		leftLines = append(leftLines, "  "+cyan.Bold(true).Render(projName))
	}
	// Memory mode
	memAll, memFacts, memNone := dim.Render("all"), dim.Render("facts"), dim.Render("none")
	switch m.MemoryMode {
	case "all":
		memAll = green.Render("all")
	case "facts":
		memFacts = green.Render("facts")
	case "none":
		memNone = green.Render("none")
	}
	leftLines = append(leftLines, fmt.Sprintf("  /memory %s · %s · %s", memAll, memFacts, memNone))

	// === BUILD RIGHT COLUMN LINES ===
	var rightLines []string
	rightLines = append(rightLines, theme.SectionHeader.Render("Tips for getting started"))
	rightLines = append(rightLines, dim.Render("/memory all · facts · none to change"))
	rightLines = append(rightLines, dim.Render("/init to set up project instructions"))
	rightLines = append(rightLines, dim.Render("────────────────────────────────────"))
	// Recent activity
	rightLines = append(rightLines, theme.SectionHeader.Render("Recent activity"))
	commits := getRecentCommits(2)
	for _, c := range commits {
		rightLines = append(rightLines, dim.Render(truncate(c, innerW/2-2)))
	}
	if len(commits) == 0 {
		rightLines = append(rightLines, dim.Render("(no recent activity)"))
	}
	rightLines = append(rightLines, dim.Render("────────────────────────────────────"))
	// System
	rightLines = append(rightLines, theme.SectionHeader.Render("System"))
	rightLines = append(rightLines, fmt.Sprintf("Sisters    %s",
		lipgloss.NewStyle().Foreground(sisterColor).Render(
			fmt.Sprintf("%d/%d connected", m.SistersConn, m.SistersTotal))))
	if m.ToolsCount > 0 {
		rightLines = append(rightLines, fmt.Sprintf("Tools      %d+", m.ToolsCount))
	} else {
		rightLines = append(rightLines, "Tools      —")
	}
	rightLines = append(rightLines, fmt.Sprintf("Health     %s",
		lipgloss.NewStyle().Foreground(healthColor).Render(fmt.Sprintf("%.0f%%", m.HealthPct))))
	modeColor := theme.HydraGreen
	modeStr := "Local"
	if m.SistersConn > 0 {
		modeStr = "Local"
		modeColor = theme.HydraGreen
	} else if m.Connected {
		modeStr = "Connecting"
		modeColor = theme.HydraYellow
	} else {
		modeStr = "Offline"
		modeColor = theme.HydraRed
	}
	rightLines = append(rightLines, fmt.Sprintf("Mode       %s %s",
		lipgloss.NewStyle().Foreground(modeColor).Render("●"), modeStr))

	// Pad columns to same height
	for len(leftLines) < len(rightLines) {
		leftLines = append(leftLines, "")
	}
	for len(rightLines) < len(leftLines) {
		rightLines = append(rightLines, "")
	}

	// === ASSEMBLE FRAME ===
	leftW := w/2 - 2
	rightW := w/2 - 2

	var frameLines []string

	// Top border: ─── Hydra v0.2.0 ════════════════════════════
	// Match old Rust TUI: no ┌┐ — just dashes with embedded title
	title := fmt.Sprintf(" Hydra v%s ", m.Version)
	topDashes := w - 3 - len(title)
	if topDashes < 2 { topDashes = 2 }
	frameLines = append(frameLines,
		blue.Render("───")+bb.Render(title)+blue.Render(strings.Repeat("─", topDashes)))

	// Content rows: │ leftcol          │ rightcol         │
	for i := 0; i < len(leftLines); i++ {
		l := leftLines[i]
		r := rightLines[i]
		lPadded := padRight(l, leftW)
		rPadded := padRight(r, rightW-1)
		row := blue.Render("│") + lPadded + blue.Render("│") + rPadded + blue.Render("│")
		frameLines = append(frameLines, row)
	}

	// Bottom border: ─── Agentra Labs ──────────────────────────
	footer := " Agentra Labs "
	botDashes := w - 3 - len(footer)
	if botDashes < 2 { botDashes = 2 }
	frameLines = append(frameLines,
		blue.Render("───")+dim.Render(footer)+blue.Render(strings.Repeat("─", botDashes)))

	result := strings.Join(frameLines, "\n")

	// Execution context line below frame
	execCtx := m.getExecutionContext()
	if execCtx != "" {
		result += "\n" + cyan.Render(execCtx)
	}

	return result
}

// padRight pads a string with spaces to the given visible width.
// Accounts for ANSI escape codes by using lipgloss width measurement.
func padRight(s string, width int) string {
	visibleLen := lipgloss.Width(s)
	if visibleLen >= width {
		return s
	}
	return s + strings.Repeat(" ", width-visibleLen)
}

func (m Model) getExecutionContext() string {
	if m.StreamActive {
		return fmt.Sprintf("streaming-%s", m.StreamRunID)
	}
	if m.Thinking {
		return fmt.Sprintf("thinking-%s", strings.ToLower(m.ThinkVerb))
	}
	// Show project exec context like the old TUI
	if proj := DetectProject(); proj != nil {
		switch proj.Kind {
		case ProjectRust:
			return fmt.Sprintf("project-exec-%s", proj.Name)
		default:
			return fmt.Sprintf("project-%s", proj.Name)
		}
	}
	return ""
}

func (m Model) renderChatArea(height int) string {
	var lines []string

	if !m.BriefingDismissed && len(m.BriefingItems) > 0 {
		lines = append(lines, m.renderBriefing()...)
		lines = append(lines, "")
	}

	for _, msg := range m.Messages {
		lines = append(lines, m.renderMessage(msg)...)
		lines = append(lines, "")
	}

	if m.StreamActive && m.RevealedChars > 0 {
		end := m.RevealedChars
		if end > len(m.StreamBuf) {
			end = len(m.StreamBuf)
		}
		visible := m.StreamBuf[:end]
		for _, line := range strings.Split(visible, "\n") {
			lines = append(lines, "  "+line)
		}
	}

	if m.Thinking {
		spinner := theme.SpinnerChars[m.SpinnerPhase%len(theme.SpinnerChars)]
		elapsed := ""
		if !m.ThinkStart.IsZero() {
			dur := time.Since(m.ThinkStart)
			if dur.Seconds() >= 60 {
				elapsed = fmt.Sprintf("%dm %ds", int(dur.Minutes()), int(dur.Seconds())%60)
			} else {
				elapsed = fmt.Sprintf("%.0fs", dur.Seconds())
			}
		}
		parts := []string{}
		if elapsed != "" {
			parts = append(parts, elapsed)
		}
		if m.ThinkTokens > 0 {
			if m.ThinkTokens > 1000 {
				parts = append(parts, fmt.Sprintf("↓ %.1fk tokens", float64(m.ThinkTokens)/1000.0))
			} else {
				parts = append(parts, fmt.Sprintf("↓ %d tokens", m.ThinkTokens))
			}
		}
		if m.ThinkCost > 0 {
			parts = append(parts, fmt.Sprintf("$%.3f", m.ThinkCost))
		}
		stats := ""
		if len(parts) > 0 {
			stats = " (" + strings.Join(parts, " · ") + ")"
		}
		thinkLine := fmt.Sprintf("✱ %s%c%s", m.ThinkVerb, spinner, stats)
		lines = append(lines,
			"  "+lipgloss.NewStyle().Foreground(theme.HydraOrange).Render(thinkLine))
		if m.ThinkTip != "" {
			lines = append(lines,
				"    "+theme.ToolConnector.Render("└ Tip: ")+theme.Dim.Render(m.ThinkTip))
		}
	}

	if len(lines) > height {
		lines = lines[len(lines)-height:]
	}
	for len(lines) < height {
		lines = append(lines, "")
	}
	return strings.Join(lines, "\n")
}

func (m Model) renderMessage(msg client.ChatMessage) []string {
	var lines []string
	switch msg.Role {
	case client.RoleUser:
		lines = append(lines, "  "+theme.InputPrompt.Render("❯ ")+theme.UserLabel.Render("You"))
		for _, line := range strings.Split(msg.Content, "\n") {
			lines = append(lines, "  "+line)
		}
	case client.RoleAssistant:
		for _, tr := range msg.ToolResults {
			lines = append(lines, m.renderToolResult(tr)...)
		}
		for _, line := range strings.Split(msg.Content, "\n") {
			lines = append(lines, "  "+line)
		}
	case client.RoleSystem:
		for _, line := range strings.Split(msg.Content, "\n") {
			lines = append(lines, "  "+theme.SystemMsg.Render(line))
		}
	}
	return lines
}

func (m Model) renderToolResult(tr client.ToolResult) []string {
	var lines []string
	dotColor := theme.DotColor(theme.DotCategory(tr.DotCategory))
	collapse := "⏵"
	if tr.Expanded {
		collapse = "⏷"
	}
	dur := fmt.Sprintf("%.1fs", float64(tr.DurationMs)/1000.0)
	lines = append(lines,
		"  "+lipgloss.NewStyle().Foreground(dotColor).Render(collapse+" ")+
			theme.ToolSisterName.Render(tr.Sister)+
			theme.ToolConnector.Render(" ▸ ")+
			tr.Action+"  "+
			theme.ToolDuration.Render(dur))
	if tr.Expanded && tr.Output != "" {
		outLines := strings.Split(tr.Output, "\n")
		show := 10
		if len(outLines) < show {
			show = len(outLines)
		}
		for _, l := range outLines[:show] {
			lines = append(lines, "    "+theme.ToolConnector.Render("└ ")+theme.Dim.Render(l))
		}
		if len(outLines) > 10 {
			lines = append(lines, fmt.Sprintf("    "+theme.Dim.Render("… +%d lines (ctrl+o to expand)"), len(outLines)-10))
		}
	}
	return lines
}

func (m Model) renderBriefing() []string {
	var lines []string
	border := lipgloss.NewStyle().Foreground(theme.HydraBlue)
	lines = append(lines, "  "+border.Render("┌─ Morning Briefing ──────────────────────────┐"))
	lines = append(lines, "  "+border.Render("│")+"  While you were away:                        "+border.Render("│"))
	for _, item := range m.BriefingItems {
		var style string
		switch item.Priority {
		case client.PriorityUrgent:
			style = theme.BriefingUrgent.Render("▲")
		case client.PriorityImportant:
			style = theme.BriefingImportant.Render("●")
		default:
			style = theme.BriefingInfo.Render("○")
		}
		lines = append(lines, "  "+border.Render("│")+"  "+style+" "+item.Text)
	}
	lines = append(lines, "  "+border.Render("└──────────────────────────────────────────────┘"))
	return lines
}

func (m Model) renderInputArea() string {
	w := m.Width - 4
	if w < 10 {
		w = 76
	}
	borderColor := theme.HydraBlue
	if !m.InputEnabled {
		borderColor = theme.HydraBorder
	}
	border := lipgloss.NewStyle().
		BorderStyle(lipgloss.RoundedBorder()).
		BorderForeground(borderColor).
		Width(w)

	var content string
	if m.Mode == ModeStreaming && !m.InputEnabled {
		content = theme.StreamingIndicator.Render("  Streaming... press Esc to cancel")
	} else if m.Mode == ModeApproval {
		content = lipgloss.NewStyle().Foreground(theme.HydraOrange).
			Render("  [Y]es  [N]o  [A]llow all this session")
	} else if m.Input == "" {
		hint := "! for bash · / for commands · \\ + enter for newline"
		if m.ProfileName != "" {
			hint += fmt.Sprintf(" · %s (%d beliefs)", m.ProfileName, m.BeliefsLoaded)
		}
		content = theme.InputPrompt.Render("> ") +
			lipgloss.NewStyle().Foreground(theme.HydraCyan).Render("█") +
			"  " + theme.InputHint.Render(hint)
	} else {
		before := m.Input[:m.CursorPos]
		cursorChar := " "
		after := ""
		if m.CursorPos < len(m.Input) {
			cursorChar = string(m.Input[m.CursorPos])
			if m.CursorPos+1 <= len(m.Input) {
				after = m.Input[m.CursorPos+1:]
			}
		}
		cursor := lipgloss.NewStyle().Foreground(lipgloss.Color("#000")).
			Background(theme.HydraCyan).Render(cursorChar)
		content = theme.InputPrompt.Render("> ") + before + cursor + after
	}
	return border.Render(content)
}

func getRecentCommits(n int) []string {
	out, err := exec.Command("git", "log", "--oneline",
		fmt.Sprintf("--format=%%s"), fmt.Sprintf("-%d", n)).Output()
	if err != nil {
		return nil
	}
	var result []string
	for _, l := range strings.Split(strings.TrimSpace(string(out)), "\n") {
		if l != "" {
			result = append(result, l)
		}
	}
	return result
}

func truncate(s string, max int) string {
	if len(s) <= max {
		return s
	}
	return s[:max-1] + "…"
}

func capitalize(s string) string {
	if len(s) == 0 {
		return s
	}
	return strings.ToUpper(s[:1]) + s[1:]
}
