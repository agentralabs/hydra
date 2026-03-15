package chat

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/agentralabs/hydra-tui/internal/client"
	"github.com/agentralabs/hydra-tui/internal/theme"
)

// RenderMessage renders a chat message to display lines.
func RenderMessage(msg client.ChatMessage, width int) []string {
	switch msg.Role {
	case client.RoleUser:
		return renderUserMessage(msg)
	case client.RoleAssistant:
		return renderAssistantMessage(msg, width)
	case client.RoleSystem:
		return renderSystemMessage(msg)
	}
	return nil
}

func renderUserMessage(msg client.ChatMessage) []string {
	lines := []string{
		"  " + theme.InputPrompt.Render("❯ ") + theme.UserLabel.Render("You"),
	}
	for _, line := range strings.Split(msg.Content, "\n") {
		lines = append(lines, "  "+line)
	}
	return lines
}

func renderAssistantMessage(msg client.ChatMessage, _ int) []string {
	var lines []string

	// Tool results first
	for _, tr := range msg.ToolResults {
		lines = append(lines, RenderToolResult(tr)...)
	}

	// Markdown-aware rendering
	lines = append(lines, RenderMarkdown(msg.Content)...)

	// Belief citations
	for _, b := range msg.BeliefsCited {
		lines = append(lines, RenderBeliefBox(b.Text, b.Confidence, b.TimesTested)...)
	}

	return lines
}

func renderSystemMessage(msg client.ChatMessage) []string {
	var lines []string
	for _, line := range strings.Split(msg.Content, "\n") {
		lines = append(lines, "  "+theme.SystemMsg.Render(line))
	}
	return lines
}

// RenderToolResult renders a tool/sister call result.
func RenderToolResult(tr client.ToolResult) []string {
	dotColor := theme.DotColor(theme.DotCategory(tr.DotCategory))
	collapse := "⏵"
	if tr.Expanded {
		collapse = "⏷"
	}
	dur := fmt.Sprintf("%.1fs", float64(tr.DurationMs)/1000.0)

	lines := []string{
		"  " + lipgloss.NewStyle().Foreground(dotColor).Render(collapse+" ") +
			theme.ToolSisterName.Render(tr.Sister) +
			theme.ToolConnector.Render(" ▸ ") +
			tr.Action + "  " +
			theme.ToolDuration.Render(dur),
	}

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

// RenderBeliefBox renders a belief citation box (Hydra Pattern 1).
func RenderBeliefBox(text string, confidence float64, timesTested uint32) []string {
	borderColor := theme.ConfidenceColor(confidence)
	border := lipgloss.NewStyle().Foreground(borderColor)

	header := fmt.Sprintf("─ Belief (%.2f, tested %dx) ", confidence, timesTested)
	pad := 50 - len(header)
	if pad < 0 {
		pad = 0
	}

	return []string{
		"  " + border.Render(fmt.Sprintf("┌%s%s┐", header, strings.Repeat("─", pad))),
		"  " + border.Render("│ ") + lipgloss.NewStyle().Italic(true).Render(text),
		"  " + border.Render(fmt.Sprintf("└%s┘", strings.Repeat("─", 50))),
	}
}

// RenderMarkdown renders basic markdown to styled lines.
func RenderMarkdown(text string) []string {
	var lines []string
	inCode := false

	for _, raw := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(raw)

		// Code blocks
		if strings.HasPrefix(trimmed, "```") {
			inCode = !inCode
			lines = append(lines, "  "+theme.Dim.Render(raw))
			continue
		}
		if inCode {
			lines = append(lines, "  "+raw)
			continue
		}

		// Headers
		if strings.HasPrefix(trimmed, "### ") {
			lines = append(lines, "  "+theme.SectionHeader.Render(trimmed[4:]))
		} else if strings.HasPrefix(trimmed, "## ") {
			lines = append(lines, "  "+theme.SectionHeader.Render(trimmed[3:]))
		} else if strings.HasPrefix(trimmed, "# ") {
			lines = append(lines, "  "+lipgloss.NewStyle().Foreground(theme.HydraBlue).Bold(true).Render(trimmed[2:]))
		} else if strings.HasPrefix(trimmed, "> ") {
			// Blockquote
			lines = append(lines, "  "+theme.Dim.Render("│ ")+lipgloss.NewStyle().Italic(true).Render(trimmed[2:]))
		} else if strings.HasPrefix(trimmed, "- ") || strings.HasPrefix(trimmed, "* ") {
			// Bullet
			indent := len(raw) - len(trimmed)
			lines = append(lines, strings.Repeat(" ", 2+indent)+
				lipgloss.NewStyle().Foreground(theme.HydraCyan).Render("• ")+trimmed[2:])
		} else if trimmed == "---" || trimmed == "***" {
			lines = append(lines, "  "+theme.Dim.Render("────────────────────────────────"))
		} else {
			lines = append(lines, "  "+raw)
		}
	}

	return lines
}
