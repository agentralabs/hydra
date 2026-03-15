package chat

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/agentralabs/hydra-tui/internal/client"
	"github.com/agentralabs/hydra-tui/internal/theme"
)

// RenderTable renders a formatted table with box-drawing characters.
func RenderTable(headers []string, rows [][]string, colWidths []int) []string {
	var lines []string

	// Top border
	topParts := make([]string, len(colWidths))
	for i, w := range colWidths {
		topParts[i] = strings.Repeat("─", w+2)
	}
	lines = append(lines, "  "+theme.Dim.Render("┌"+strings.Join(topParts, "┬")+"┐"))

	// Header row
	headerCells := make([]string, len(headers))
	for i, h := range headers {
		headerCells[i] = fmt.Sprintf("│ %-*s ", colWidths[i], h)
	}
	lines = append(lines, "  "+theme.SectionHeader.Render(strings.Join(headerCells, ""))+"│")

	// Separator
	sepParts := make([]string, len(colWidths))
	for i, w := range colWidths {
		sepParts[i] = strings.Repeat("─", w+2)
	}
	lines = append(lines, "  "+theme.Dim.Render("├"+strings.Join(sepParts, "┼")+"┤"))

	// Data rows
	for _, row := range rows {
		cells := make([]string, len(row))
		for i, cell := range row {
			w := 10
			if i < len(colWidths) {
				w = colWidths[i]
			}
			cells[i] = fmt.Sprintf("│ %-*s ", w, cell)
		}
		lines = append(lines, "  "+strings.Join(cells, "")+"│")
	}

	// Bottom border
	botParts := make([]string, len(colWidths))
	for i, w := range colWidths {
		botParts[i] = strings.Repeat("─", w+2)
	}
	lines = append(lines, "  "+theme.Dim.Render("└"+strings.Join(botParts, "┴")+"┘"))

	return lines
}

// RenderProgressBar renders a progress bar.
func RenderProgressBar(progress float64, width int) string {
	filled := int(progress / 100.0 * float64(width))
	if filled > width {
		filled = width
	}
	empty := width - filled

	return lipgloss.NewStyle().Foreground(theme.HydraBlue).Render(strings.Repeat("█", filled)) +
		theme.Dim.Render(strings.Repeat("░", empty)) +
		theme.Dim.Render(fmt.Sprintf("  %.0f%%", progress))
}

// RenderSectionHeader renders a section header with ═══ underline.
func RenderSectionHeader(title string) []string {
	return []string{
		"  " + theme.SectionHeader.Render(title),
		"  " + theme.SectionHeader.Render(strings.Repeat("═", len(title))),
	}
}

// RenderLogo renders the Hydra diamond logo.
func RenderLogo() []string {
	cyan := lipgloss.NewStyle().Foreground(theme.HydraCyan)
	blue := lipgloss.NewStyle().Foreground(theme.HydraBlue)

	return []string{
		"          " + cyan.Render("◉"),
		"        " + blue.Render("╱") + "   " + blue.Render("╲"),
		"       " + cyan.Render("◉") + blue.Render("─────") + cyan.Render("◉"),
		"        " + blue.Render("╲") + "   " + blue.Render("╱"),
		"          " + cyan.Render("◉"),
	}
}

// RenderDiffBlock renders a diff with full-color background bands.
func RenderDiffBlock(filePath string, added, removed int, diffLines []DiffLine) []string {
	var lines []string

	// Header
	lines = append(lines,
		"  "+lipgloss.NewStyle().Foreground(theme.HydraGreen).Render("● ")+
			theme.ToolSisterName.Render("Forge")+
			theme.ToolConnector.Render(" ▸ ")+
			fmt.Sprintf("Edit(%s)", filePath))

	lines = append(lines,
		"  "+theme.ToolConnector.Render("└ ")+
			theme.Dim.Render(fmt.Sprintf("Added %d lines, removed %d lines", added, removed)))

	// Diff lines
	for _, dl := range diffLines {
		var style lipgloss.Style
		prefix := " "
		switch dl.ChangeType {
		case ChangeAdded:
			style = theme.DiffAdded
			prefix = "+"
		case ChangeRemoved:
			style = theme.DiffRemoved
			prefix = "-"
		default:
			style = lipgloss.NewStyle()
		}

		lines = append(lines, fmt.Sprintf("    %s %s %s",
			theme.DiffLineNumber.Render(fmt.Sprintf("%4d", dl.LineNumber)),
			style.Render(prefix+" "+dl.Content),
			""))
	}

	return lines
}

// DiffLine represents a single line in a diff display.
type DiffLine struct {
	LineNumber int
	Content    string
	ChangeType ChangeType
}

// ChangeType for diff lines.
type ChangeType int

const (
	ChangeContext ChangeType = iota
	ChangeAdded
	ChangeRemoved
)

// RenderBriefingCard renders the morning briefing card.
func RenderBriefingCard(items []client.BriefingItem, width int) []string {
	border := lipgloss.NewStyle().Foreground(theme.HydraBlue)
	var lines []string

	lines = append(lines, "  "+border.Render("┌─ Morning Briefing ──────────────────────────┐"))
	lines = append(lines, "  "+border.Render("│")+"                                              "+border.Render("│"))
	lines = append(lines, "  "+border.Render("│")+"  While you were away:                        "+border.Render("│"))
	lines = append(lines, "  "+border.Render("│")+"                                              "+border.Render("│"))

	for _, item := range items {
		var indicator string
		switch item.Priority {
		case client.PriorityUrgent:
			indicator = theme.BriefingUrgent.Render("▲")
		case client.PriorityImportant:
			indicator = theme.BriefingImportant.Render("●")
		default:
			indicator = theme.BriefingInfo.Render("○")
		}
		lines = append(lines, "  "+border.Render("│")+"  "+indicator+" "+item.Text)
	}

	lines = append(lines, "  "+border.Render("│")+"                                              "+border.Render("│"))
	lines = append(lines, "  "+border.Render("└──────────────────────────────────────────────┘"))

	return lines
}

// RenderCompletionReport renders a formatted completion report (Pattern 6).
func RenderCompletionReport(title string, stats map[string]string, filesChanged int, cost float64) []string {
	var lines []string

	lines = append(lines,
		"  "+lipgloss.NewStyle().Foreground(theme.HydraGreen).Render("● ")+
			lipgloss.NewStyle().Bold(true).Render("Task Complete: "+title))
	lines = append(lines, "")

	border := lipgloss.NewStyle().Foreground(theme.HydraBlue)
	lines = append(lines, "  "+border.Render("┌─ Results ────────────────────────────────────┐"))
	lines = append(lines, "  "+border.Render("│")+"                                              "+border.Render("│"))

	for k, v := range stats {
		lines = append(lines, "  "+border.Render("│")+fmt.Sprintf("  %-20s%s", k, v))
	}

	lines = append(lines, "  "+border.Render("│")+
		theme.Dim.Render(fmt.Sprintf("  Cost: $%.4f  Files: %d", cost, filesChanged)))
	lines = append(lines, "  "+border.Render("│")+"                                              "+border.Render("│"))
	lines = append(lines, "  "+border.Render("└──────────────────────────────────────────────┘"))

	return lines
}
