package app

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/agentralabs/hydra-tui/internal/theme"
)

// RenderBootScreen renders the clean boot screen — progress bar only, no logs.
func RenderBootScreen(m Model) string {
	var b strings.Builder
	w := m.Width
	if w < 20 {
		w = 80
	}

	// Center vertically
	padTop := m.Height/2 - 3
	for i := 0; i < padTop; i++ {
		b.WriteString("\n")
	}

	// Logo: ◉ Hydra
	padLeft := strings.Repeat(" ", w/4)
	logo := lipgloss.NewStyle().Foreground(theme.HydraCyan).Render("◉") +
		" " +
		lipgloss.NewStyle().Foreground(theme.HydraBlue).Bold(true).Render("Hydra")
	b.WriteString(padLeft + logo + "\n\n")

	// Progress bar
	barWidth := w / 2
	if barWidth > 52 {
		barWidth = 52
	}
	filled := int(m.BootProgress / 100.0 * float64(barWidth))
	if filled > barWidth {
		filled = barWidth
	}
	empty := barWidth - filled
	bar := lipgloss.NewStyle().Foreground(theme.HydraBlue).Render(strings.Repeat("█", filled)) +
		lipgloss.NewStyle().Foreground(theme.HydraDim).Render(strings.Repeat("░", empty)) +
		lipgloss.NewStyle().Foreground(theme.HydraDim).Render(fmt.Sprintf("  %.0f%%", m.BootProgress))
	b.WriteString(padLeft + bar + "\n\n")

	// Stage text
	stage := m.BootStage
	if m.BootProgress >= 100 {
		stage = "Ready."
	}
	b.WriteString(padLeft + lipgloss.NewStyle().Foreground(theme.HydraDim).Render(stage) + "\n")

	// Error
	if m.BootError != "" {
		b.WriteString("\n" + padLeft +
			lipgloss.NewStyle().Foreground(theme.HydraRed).Render(m.BootError) + "\n")
	}

	return b.String()
}

// TickBoot advances boot progress — called each tick during boot phase.
// Auto-starts hydra-server if not running.
func TickBoot(m *Model) {
	if m.BootComplete {
		return
	}

	elapsed := time.Since(m.BootStart).Milliseconds()

	if elapsed < 500 {
		m.BootProgress = float64(elapsed) / 500.0 * 20.0
		m.BootStage = "Loading configuration..."
	} else if elapsed < 2000 {
		m.BootProgress = 20.0 + float64(elapsed-500)/1500.0*30.0
		// Try connecting to existing server first
		if !m.Connected && !m.ServerStarted {
			m.Connected = m.Client.HealthCheck()
			if !m.Connected {
				m.BootStage = "Starting hydra-server..."
				autoStartServer()
				m.ServerStarted = true
			}
		}
		// Keep retrying connection while server boots
		if !m.Connected {
			m.BootStage = "Waiting for server..."
			m.Connected = m.Client.HealthCheck()
		} else {
			m.BootStage = "Server connected"
		}
	} else if elapsed < 5000 {
		m.BootProgress = 50.0 + float64(elapsed-2000)/3000.0*30.0
		// Keep trying to connect (server may take a few seconds to init sisters)
		if !m.Connected {
			m.Connected = m.Client.HealthCheck()
			m.BootStage = "Waiting for sisters..."
		}
		if m.Connected && m.SistersConn == 0 {
			m.BootStage = "Loading sisters + beliefs..."
			if health, err := m.Client.Health(); err == nil {
				m.SistersConn = health.SistersConnected
				m.SistersTotal = health.SistersTotal
				m.BeliefsLoaded = health.BeliefsLoaded
				if health.Model != nil {
					m.ModelName = *health.Model
				}
				if health.Profile != nil {
					m.ProfileName = *health.Profile
				}
				m.HealthPct = float64(health.SistersConnected) / float64(max(health.SistersTotal, 1)) * 100
				m.Online = true
			}
		}
	} else if elapsed < 7000 {
		m.BootProgress = 80.0 + float64(elapsed-5000)/2000.0*20.0
		m.BootStage = "Preparing..."
		m.CrateCount = detectCrateCount()
		// Final connection attempt
		if !m.Connected {
			m.Connected = m.Client.HealthCheck()
		}
	} else {
		m.BootProgress = 100
		m.BootStage = "Ready."
		if !m.Connected {
			m.BootError = "hydra-server not responding. Start it with: hydra-cli serve"
		}
		m.BootComplete = true
	}
}

// autoStartServer launches hydra-server as a background process.
func autoStartServer() {
	// Find hydra-server binary
	paths := []string{
		"hydra-server",                          // PATH
	}
	if home, err := os.UserHomeDir(); err == nil {
		paths = append(paths, home+"/.local/bin/hydra-server")
		paths = append(paths, home+"/.cargo/bin/hydra-server")
	}
	// Try target/debug from workspace
	if cwd, err := os.Getwd(); err == nil {
		paths = append(paths, cwd+"/target/debug/hydra-server")
	}

	for _, p := range paths {
		if _, err := exec.LookPath(p); err == nil {
			cmd := exec.Command(p)
			cmd.Stdout = nil
			cmd.Stderr = nil
			// Start in background — don't wait
			if err := cmd.Start(); err == nil {
				fmt.Fprintf(os.Stderr, "[hydra-tui] Started hydra-server (pid %d)\n", cmd.Process.Pid)
				go cmd.Wait() // Reap zombie
				return
			}
		}
		// Also try as absolute path
		if fileExists(p) {
			cmd := exec.Command(p)
			cmd.Stdout = nil
			cmd.Stderr = nil
			if err := cmd.Start(); err == nil {
				fmt.Fprintf(os.Stderr, "[hydra-tui] Started hydra-server from %s (pid %d)\n", p, cmd.Process.Pid)
				go cmd.Wait()
				return
			}
		}
	}
	fmt.Fprintf(os.Stderr, "[hydra-tui] Could not find hydra-server binary\n")
}

// fileExists is defined in project.go

func max(a, b uint32) uint32 {
	if a > b {
		return a
	}
	return b
}
