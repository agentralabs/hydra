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
	var sections []string

	// 1. Pinned upper frame
	sections = append(sections, m.renderUpperFrame())

	// 2. Chat area
	chatHeight := m.Height - m.FrameHeight() - 3 // 3 for input
	if chatHeight < 3 {
		chatHeight = 3
	}
	sections = append(sections, m.renderChatArea(chatHeight))

	// 3. Input area
	sections = append(sections, m.renderInputArea())

	return strings.Join(sections, "\n")
}

func (m Model) renderUpperFrame() string {
	w := m.Width - 2
	if w < 20 {
		w = 78
	}

	sisterPct := float64(m.SistersConn) / float64(max(m.SistersTotal, 1)) * 100
	sisterColor := theme.HealthColor(sisterPct)
	healthColor := theme.HealthColor(m.HealthPct)
	blue := lipgloss.NewStyle().Foreground(theme.HydraBlue)
	cyan := lipgloss.NewStyle().Foreground(theme.HydraCyan)
	dim := theme.Dim
	green := lipgloss.NewStyle().Foreground(theme.HydraGreen)

	if m.IsNarrow() {
		content := fmt.Sprintf(
			"%s · %s  Sisters %d/%d",
			theme.FrameModel.Render(m.ModelName),
			theme.FrameGitBranch.Render(m.GitBranch),
			m.SistersConn, m.SistersTotal,
		)
		return theme.FrameBorder.Width(w).Render(content)
	}

	// === LEFT COLUMN ===
	var left strings.Builder
	// Welcome
	left.WriteString(fmt.Sprintf("    Welcome back %s!\n", theme.FrameUsername.Render(m.Username)))
	left.WriteString("\n")
	// Diamond logo (matches the screenshot exactly)
	left.WriteString(fmt.Sprintf("            %s\n", cyan.Render("●")))
	left.WriteString(fmt.Sprintf("          %s   %s\n", blue.Render("╱"), blue.Render("╲")))
	left.WriteString(fmt.Sprintf("    %s%s%s\n", cyan.Render("●"), blue.Render("─────────"), cyan.Render("●")))
	left.WriteString(fmt.Sprintf("          %s   %s\n", blue.Render("╲"), blue.Render("╱")))
	left.WriteString(fmt.Sprintf("            %s\n", cyan.Render("●")))
	left.WriteString("\n")
	// Model + git branch
	left.WriteString(fmt.Sprintf("  %s %s · %s\n",
		theme.FrameModel.Render(m.ModelName),
		dim.Render("("+strings.Title(m.ProviderName)+")"),
		theme.FrameGitBranch.Render(m.GitBranch)))
	// Project path
	left.WriteString(fmt.Sprintf("  %s\n", dim.Render(m.ProjectPath)))
	// Project name + crate count
	projName := "project"
	if proj := DetectProject(); proj != nil {
		projName = proj.Name
	}
	if m.CrateCount > 0 {
		left.WriteString(fmt.Sprintf("  %s\n",
			cyan.Bold(true).Render(fmt.Sprintf("%s (%d crates)", projName, m.CrateCount))))
	} else {
		left.WriteString(fmt.Sprintf("  %s\n", cyan.Bold(true).Render(projName)))
	}
	// Memory mode
	memAll := dim.Render("all")
	memFacts := dim.Render("facts")
	memNone := dim.Render("none")
	switch m.MemoryMode {
	case "all":
		memAll = green.Render("all")
	case "facts":
		memFacts = green.Render("facts")
	case "none":
		memNone = green.Render("none")
	}
	left.WriteString(fmt.Sprintf("  /memory %s · %s · %s\n", memAll, memFacts, memNone))

	// === RIGHT COLUMN ===
	var right strings.Builder
	// Tips section
	right.WriteString(theme.SectionHeader.Render("Tips for getting started") + "\n")
	right.WriteString(dim.Render("/memory all · facts · none to change") + "\n")
	right.WriteString(dim.Render("/init to set up project instructions") + "\n")
	right.WriteString(dim.Render("────────────────────────────────────") + "\n")

	// Recent activity (last 2 git commits)
	right.WriteString(theme.SectionHeader.Render("Recent activity") + "\n")
	commits := getRecentCommits(2)
	for _, c := range commits {
		right.WriteString(dim.Render(truncate(c, 40)) + "\n")
	}
	if len(commits) == 0 {
		right.WriteString(dim.Render("(no recent activity)") + "\n")
	}
	right.WriteString(dim.Render("────────────────────────────────────") + "\n")

	// System section
	right.WriteString(theme.SectionHeader.Render("System") + "\n")
	right.WriteString(fmt.Sprintf("Sisters    %s\n",
		lipgloss.NewStyle().Foreground(sisterColor).Render(
			fmt.Sprintf("%d/%d connected", m.SistersConn, m.SistersTotal))))
	if m.ToolsCount > 0 {
		right.WriteString(fmt.Sprintf("Tools      %d+\n", m.ToolsCount))
	} else {
		right.WriteString("Tools      —\n")
	}
	right.WriteString(fmt.Sprintf("Health     %s\n",
		lipgloss.NewStyle().Foreground(healthColor).Render(fmt.Sprintf("%.0f%%", m.HealthPct))))
	modeColor := theme.HydraGreen
	modeStr := "Local"
	if m.Online {
		modeStr = "Online"
	} else {
		modeColor = theme.HydraGreen // green dot even for local (matches screenshot)
	}
	right.WriteString(fmt.Sprintf("Mode       %s %s\n",
		lipgloss.NewStyle().Foreground(modeColor).Render("●"),
		modeStr))

	// Combine columns
	leftW := w * 50 / 100
	rightW := w - leftW
	leftCol := lipgloss.NewStyle().Width(leftW).Render(left.String())
	rightCol := lipgloss.NewStyle().Width(rightW).Render(right.String())
	content := lipgloss.JoinHorizontal(lipgloss.Top, leftCol, rightCol)

	// Frame border
	border := lipgloss.NewStyle().
		BorderStyle(lipgloss.NormalBorder()).
		BorderForeground(theme.HydraBlue).
		Width(w)

	framed := border.Render(content)

	// Title: "─── Hydra v0.2.0 ───" at top
	// Footer: "─── Agentra Labs ───" at bottom
	// Execution context line below frame
	execCtx := m.getExecutionContext()
	footer := ""
	if execCtx != "" {
		footer = "\n" + cyan.Render(execCtx)
	}

	return framed + footer
}

func (m Model) getExecutionContext() string {
	// Show current execution context below the frame (like in the screenshot)
	// e.g., "project-exec-hydra-native-cognitive"
	if m.StreamActive {
		return fmt.Sprintf("streaming-%s", m.StreamRunID)
	}
	if m.Thinking {
		sister := "cognitive"
		if m.ThinkVerb != "Thinking" {
			sister = strings.ToLower(m.ThinkVerb)
		}
		return fmt.Sprintf("thinking-%s", sister)
	}
	return ""
}

func getRecentCommits(n int) []string {
	out, err := exec.Command("git", "log", "--oneline", "--format=%s — …",
		fmt.Sprintf("-%d", n)).Output()
	if err != nil {
		return nil
	}
	lines := strings.Split(strings.TrimSpace(string(out)), "\n")
	var result []string
	for _, l := range lines {
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

func (m Model) renderChatArea(height int) string {
	var lines []string

	// Briefing
	if !m.BriefingDismissed && len(m.BriefingItems) > 0 {
		lines = append(lines, m.renderBriefing()...)
		lines = append(lines, "")
	}

	// Messages
	for _, msg := range m.Messages {
		lines = append(lines, m.renderMessage(msg)...)
		lines = append(lines, "")
	}

	// Streaming
	if m.StreamActive && m.RevealedChars > 0 {
		visible := m.StreamBuf[:m.RevealedChars]
		for _, line := range strings.Split(visible, "\n") {
			lines = append(lines, "  "+line)
		}
	}

	// Thinking indicator — live stats (Pattern 19 addendum)
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
		tokenStr := ""
		if m.ThinkTokens > 0 {
			if m.ThinkTokens > 1000 {
				tokenStr = fmt.Sprintf("↓ %.1fk tokens", float64(m.ThinkTokens)/1000.0)
			} else {
				tokenStr = fmt.Sprintf("↓ %d tokens", m.ThinkTokens)
			}
		}
		costStr := ""
		if m.ThinkCost > 0 {
			costStr = fmt.Sprintf("$%.3f", m.ThinkCost)
		}

		// Build stats string
		stats := ""
		parts := []string{}
		if elapsed != "" {
			parts = append(parts, elapsed)
		}
		if tokenStr != "" {
			parts = append(parts, tokenStr)
		}
		if costStr != "" {
			parts = append(parts, costStr)
		}
		if len(parts) > 0 {
			stats = " (" + strings.Join(parts, " · ") + ")"
		}

		thinkLine := fmt.Sprintf("✱ %s%c%s", m.ThinkVerb, spinner, stats)
		lines = append(lines,
			"  "+lipgloss.NewStyle().Foreground(theme.HydraOrange).Render(thinkLine))

		// Contextual tip (rotates every ~15s)
		if m.ThinkTip != "" {
			lines = append(lines,
				"    "+theme.ToolConnector.Render("└ Tip: ")+theme.Dim.Render(m.ThinkTip))
		}
	}

	// Trim to height (show last N lines)
	if len(lines) > height {
		lines = lines[len(lines)-height:]
	}

	// Pad to height
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
			lines = append(lines,
				"    "+theme.Dim.Render(fmt.Sprintf("… +%d lines (ctrl+o to expand)", len(outLines)-10)))
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
		var indicator, style string
		switch item.Priority {
		case client.PriorityUrgent:
			indicator = "▲"
			style = theme.BriefingUrgent.Render(indicator)
		case client.PriorityImportant:
			indicator = "●"
			style = theme.BriefingImportant.Render(indicator)
		default:
			indicator = "○"
			style = theme.BriefingInfo.Render(indicator)
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
		hint := "  ! bash · / commands · \\ + enter for newline"
		if m.ProfileName != "" {
			hint += fmt.Sprintf(" · %s (%d beliefs)", m.ProfileName, m.BeliefsLoaded)
		}
		content = theme.InputPrompt.Render("> ") +
			lipgloss.NewStyle().Foreground(theme.HydraCyan).Render("█") +
			theme.InputHint.Render(hint)
	} else {
		before := m.Input[:m.CursorPos]
		cursorChar := " "
		after := ""
		if m.CursorPos < len(m.Input) {
			cursorChar = string(m.Input[m.CursorPos])
			after = m.Input[m.CursorPos+1:]
		}
		cursor := lipgloss.NewStyle().Foreground(lipgloss.Color("#000")).
			Background(theme.HydraCyan).Render(cursorChar)
		content = theme.InputPrompt.Render("> ") + before + cursor + after
	}

	return border.Render(content)
}
