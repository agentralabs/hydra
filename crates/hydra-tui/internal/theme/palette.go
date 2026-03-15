package theme

import "github.com/charmbracelet/lipgloss"

// Hydra brand palette — 7 semantic colors + utility
var (
	HydraBlue       = lipgloss.Color("#6495ED") // Primary accent, borders, headers
	HydraCyan       = lipgloss.Color("#00D2D2") // Username, keywords, sister working
	HydraGreen      = lipgloss.Color("#50C878") // Success, connected, healthy
	HydraRed        = lipgloss.Color("#DC5050") // Error, offline, critical
	HydraYellow     = lipgloss.Color("#F0C850") // Warning, partial, uncertain
	HydraOrange     = lipgloss.Color("#F0A03C") // Decide phase, action needed
	HydraPurple     = lipgloss.Color("#A078DC") // Model name, Learn phase
	HydraDim        = lipgloss.Color("#808080") // Labels, paths, subtle text
	HydraBorder     = lipgloss.Color("#808080") // Inactive borders
	HydraBorderActive = lipgloss.Color("#6495ED") // Active = HYDRA_BLUE

	// Diff backgrounds
	DiffRedBG  = lipgloss.Color("#501414")
	DiffGreenBG = lipgloss.Color("#143C14")
)

// DotCategory represents the type of activity for colored dots.
type DotCategory int

const (
	DotThinking DotCategory = iota
	DotSisterWorking
	DotSuccess
	DotError
	DotWarning
	DotCognitive
	DotApproval
)

// DotColor returns the color for a given dot category.
func DotColor(cat DotCategory) lipgloss.Color {
	switch cat {
	case DotThinking:
		return HydraDim
	case DotSisterWorking:
		return HydraCyan
	case DotSuccess:
		return HydraGreen
	case DotError:
		return HydraRed
	case DotWarning:
		return HydraYellow
	case DotCognitive:
		return HydraPurple
	case DotApproval:
		return HydraOrange
	default:
		return HydraDim
	}
}

// ConfidenceColor returns color for belief confidence levels.
func ConfidenceColor(confidence float64) lipgloss.Color {
	if confidence > 0.85 {
		return HydraGreen
	} else if confidence >= 0.5 {
		return HydraYellow
	}
	return HydraRed
}

// HealthColor returns color for health percentage.
func HealthColor(pct float64) lipgloss.Color {
	if pct >= 90 {
		return HydraGreen
	} else if pct >= 50 {
		return HydraYellow
	}
	return HydraRed
}

// PhaseColor returns color for cognitive phases.
func PhaseColor(phase string) lipgloss.Color {
	switch phase {
	case "Perceive":
		return HydraBlue
	case "Think":
		return HydraYellow
	case "Decide":
		return HydraOrange
	case "Act":
		return HydraGreen
	case "Learn":
		return HydraPurple
	default:
		return HydraDim
	}
}

// Spinner phases for Hydra-themed spinner
var SpinnerChars = []rune{'◌', '◐', '◑', '◒', '◓', '●'}
