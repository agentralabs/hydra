// Hydra TUI v2 — Claude Code-quality terminal interface for Hydra.
//
// Connects to hydra-server via JSON-RPC + SSE on localhost:7777.
// Built with Bubble Tea + Lip Gloss.
//
// Install: go install github.com/agentralabs/hydra-tui@latest
// Run:     hydra-tui
package main

import (
	"fmt"
	"os"

	tea "github.com/charmbracelet/bubbletea"

	"github.com/agentralabs/hydra-tui/internal/app"
)

func main() {
	// Redirect stderr to log file
	homeDir, _ := os.UserHomeDir()
	logDir := homeDir + "/.hydra"
	_ = os.MkdirAll(logDir, 0755)
	logFile, err := os.OpenFile(logDir+"/hydra-tui.log", os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err == nil {
		os.Stderr = logFile
		defer logFile.Close()
	}

	model := app.NewModel()

	p := tea.NewProgram(
		model,
		tea.WithAltScreen(),
		// No mouse capture — allows native terminal text selection + copy
	)

	if _, err := p.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "hydra-tui error: %v\n", err)
		os.Exit(1)
	}
}
