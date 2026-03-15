package chat

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/agentralabs/hydra-tui/internal/client"
	"github.com/agentralabs/hydra-tui/internal/theme"
)

// numberedListRe matches lines starting with "1.", "2.", etc.
var numberedListRe = regexp.MustCompile(`^(\s*)(\d+)\.\s+(.*)$`)

// rustKeywords for basic syntax highlighting inside code blocks.
var rustKeywords = map[string]bool{
	"fn": true, "let": true, "pub": true, "use": true, "impl": true,
	"struct": true, "enum": true, "match": true, "if": true, "else": true,
	"for": true, "while": true, "return": true, "async": true, "await": true,
	"mut": true, "self": true, "super": true, "crate": true, "mod": true,
	"trait": true, "type": true, "where": true, "loop": true, "break": true,
	"continue": true, "move": true, "ref": true, "const": true, "static": true,
	"unsafe": true, "extern": true, "dyn": true, "as": true, "in": true,
}

// highlightCodeLine applies basic syntax coloring to a line of code.
// Supports Rust keywords (HYDRA_BLUE), strings (HYDRA_GREEN), comments (HYDRA_DIM).
func highlightCodeLine(line string, lang string) string {
	trimmed := strings.TrimSpace(line)

	// Comment line (// for Rust/Go/JS/TS/C/C++, # for Python/Ruby/Shell)
	if strings.HasPrefix(trimmed, "//") || strings.HasPrefix(trimmed, "#") {
		return lipgloss.NewStyle().Foreground(theme.HydraDim).Render(line)
	}

	kwStyle := lipgloss.NewStyle().Foreground(theme.HydraBlue)
	strStyle := lipgloss.NewStyle().Foreground(theme.HydraGreen)
	commentStyle := lipgloss.NewStyle().Foreground(theme.HydraDim)

	// Check for inline comment
	commentIdx := -1
	inStr := false
	for i := 0; i < len(line)-1; i++ {
		ch := line[i]
		if ch == '"' && (i == 0 || line[i-1] != '\\') {
			inStr = !inStr
		}
		if !inStr && ch == '/' && line[i+1] == '/' {
			commentIdx = i
			break
		}
	}

	codePart := line
	commentPart := ""
	if commentIdx >= 0 {
		codePart = line[:commentIdx]
		commentPart = line[commentIdx:]
	}

	// Highlight strings in codePart
	var result strings.Builder
	i := 0
	for i < len(codePart) {
		if codePart[i] == '"' {
			// Find closing quote
			end := i + 1
			for end < len(codePart) {
				if codePart[end] == '"' && codePart[end-1] != '\\' {
					end++
					break
				}
				end++
			}
			result.WriteString(strStyle.Render(codePart[i:end]))
			i = end
			continue
		}

		// Check for keyword at word boundary
		if isWordBoundary(codePart, i) {
			found := false
			for kw := range rustKeywords {
				if i+len(kw) <= len(codePart) && codePart[i:i+len(kw)] == kw && isWordEnd(codePart, i+len(kw)) {
					result.WriteString(kwStyle.Render(kw))
					i += len(kw)
					found = true
					break
				}
			}
			if found {
				continue
			}
		}

		result.WriteByte(codePart[i])
		i++
	}

	if commentPart != "" {
		result.WriteString(commentStyle.Render(commentPart))
	}

	return result.String()
}

func isWordBoundary(s string, i int) bool {
	if i == 0 {
		return true
	}
	c := s[i-1]
	return !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_')
}

func isWordEnd(s string, i int) bool {
	if i >= len(s) {
		return true
	}
	c := s[i]
	return !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_')
}

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
	codeLang := ""
	numStyle := lipgloss.NewStyle().Foreground(theme.HydraCyan).Bold(true)

	for _, raw := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(raw)

		// Code blocks
		if strings.HasPrefix(trimmed, "```") {
			if !inCode {
				codeLang = strings.TrimPrefix(trimmed, "```")
				codeLang = strings.TrimSpace(codeLang)
			}
			inCode = !inCode
			if !inCode {
				codeLang = ""
			}
			lines = append(lines, "  "+theme.Dim.Render(raw))
			continue
		}
		if inCode {
			lines = append(lines, "  "+highlightCodeLine(raw, codeLang))
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
		} else if m := numberedListRe.FindStringSubmatch(raw); m != nil {
			// Numbered list: m[1]=indent, m[2]=number, m[3]=text
			indent := m[1]
			lines = append(lines, "  "+indent+numStyle.Render(m[2]+".")+
				" "+m[3])
		} else if trimmed == "---" || trimmed == "***" {
			lines = append(lines, "  "+theme.Dim.Render("────────────────────────────────"))
		} else {
			lines = append(lines, "  "+raw)
		}
	}

	return lines
}
