package app

import (
	"fmt"
	"os"
	"os/user"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/agentralabs/hydra-tui/internal/theme"
)

// OnboardingStep tracks position in the onboarding flow.
type OnboardingStep int

const (
	StepWelcome OnboardingStep = iota
	StepName
	StepWorkDir
	StepAPIKey
	StepModel
	StepComplete
)

// OnboardingState holds onboarding wizard state.
type OnboardingState struct {
	Step      OnboardingStep
	Profile   PersistedProfile
	Input     string
	CursorPos int
}

// NewOnboarding creates a new onboarding state.
func NewOnboarding() OnboardingState {
	u, _ := user.Current()
	name := "User"
	if u != nil {
		name = u.Username
	}
	cwd, _ := os.Getwd()

	return OnboardingState{
		Step: StepWelcome,
		Profile: PersistedProfile{
			UserName:       name,
			SelectedModel:  "claude-sonnet-4-6",
			MemoryCapture:  "all",
			AutonomyLevel:  "balanced",
			SoundsEnabled:  true,
			SoundVolume:    70,
			Theme:          "dark",
			DefaultMode:    "companion",
		},
		Input: cwd,
	}
}

// NeedsOnboarding checks if onboarding is required.
func NeedsOnboarding() bool {
	p, err := LoadProfile()
	if err != nil {
		return true
	}
	return !p.OnboardingComplete
}

// OnboardingUpdate handles key events during onboarding.
func OnboardingUpdate(o *OnboardingState, msg tea.KeyMsg) (bool, *PersistedProfile) {
	switch msg.String() {
	case "ctrl+c":
		return true, nil // quit without saving
	case "esc":
		if o.Step > StepWelcome {
			return false, nil // skip step
		}
	}

	switch o.Step {
	case StepWelcome:
		if msg.String() == "enter" {
			o.Step = StepName
			o.Input = o.Profile.UserName
			o.CursorPos = len(o.Input)
		}

	case StepName:
		switch msg.String() {
		case "enter":
			if o.Input != "" {
				o.Profile.UserName = o.Input
			}
			cwd, _ := os.Getwd()
			o.Input = cwd
			o.CursorPos = len(o.Input)
			o.Step = StepWorkDir
		case "backspace":
			if o.CursorPos > 0 {
				o.Input = o.Input[:o.CursorPos-1] + o.Input[o.CursorPos:]
				o.CursorPos--
			}
		default:
			if len(msg.String()) == 1 {
				o.Input = o.Input[:o.CursorPos] + msg.String() + o.Input[o.CursorPos:]
				o.CursorPos++
			}
		}

	case StepWorkDir:
		switch msg.String() {
		case "enter":
			if o.Input != "" {
				dir := o.Input
				o.Profile.WorkingDirectory = &dir
			}
			// Pre-fill API key from env
			o.Input = os.Getenv("ANTHROPIC_API_KEY")
			o.CursorPos = len(o.Input)
			o.Step = StepAPIKey
		case "backspace":
			if o.CursorPos > 0 {
				o.Input = o.Input[:o.CursorPos-1] + o.Input[o.CursorPos:]
				o.CursorPos--
			}
		case "esc":
			o.Input = ""
			o.CursorPos = 0
			o.Step = StepAPIKey
		default:
			if len(msg.String()) == 1 {
				o.Input = o.Input[:o.CursorPos] + msg.String() + o.Input[o.CursorPos:]
				o.CursorPos++
			}
		}

	case StepAPIKey:
		switch msg.String() {
		case "enter", "esc":
			if o.Input != "" {
				key := o.Input
				if strings.HasPrefix(key, "sk-ant-") {
					o.Profile.AnthropicAPIKey = &key
				} else if strings.HasPrefix(key, "sk-") {
					o.Profile.OpenAIAPIKey = &key
				} else {
					o.Profile.APIKey = &key
				}
			}
			o.Input = ""
			o.Step = StepModel
		case "backspace":
			if o.CursorPos > 0 {
				o.Input = o.Input[:o.CursorPos-1] + o.Input[o.CursorPos:]
				o.CursorPos--
			}
		default:
			if len(msg.String()) == 1 {
				o.Input = o.Input[:o.CursorPos] + msg.String() + o.Input[o.CursorPos:]
				o.CursorPos++
			}
		}

	case StepModel:
		switch msg.String() {
		case "1":
			o.Profile.SelectedModel = "claude-sonnet-4-6"
			o.Step = StepComplete
		case "2":
			o.Profile.SelectedModel = "claude-opus-4-6"
			o.Step = StepComplete
		case "3":
			o.Profile.SelectedModel = "claude-haiku-4-5"
			o.Step = StepComplete
		case "enter":
			o.Step = StepComplete
		}

	case StepComplete:
		if msg.String() == "enter" {
			o.Profile.OnboardingComplete = true
			_ = SaveProfile(&o.Profile)
			return false, &o.Profile
		}
	}

	return false, nil
}

// OnboardingView renders the onboarding wizard.
func OnboardingView(o OnboardingState, width, height int) string {
	var b strings.Builder

	padTop := height/2 - 8
	for i := 0; i < padTop; i++ {
		b.WriteString("\n")
	}

	padLeft := strings.Repeat(" ", width/4)
	title := lipgloss.NewStyle().Foreground(theme.HydraBlue).Bold(true)
	dim := lipgloss.NewStyle().Foreground(theme.HydraDim)
	cyan := lipgloss.NewStyle().Foreground(theme.HydraCyan)

	b.WriteString(padLeft + title.Render("◉ Hydra — Setup") + "\n\n")

	switch o.Step {
	case StepWelcome:
		b.WriteString(padLeft + "Welcome to Hydra!\n\n")
		b.WriteString(padLeft + dim.Render("Press Enter to begin setup...") + "\n")

	case StepName:
		b.WriteString(padLeft + "What's your name?\n\n")
		b.WriteString(padLeft + cyan.Render("> ") + o.Input + "█\n\n")
		b.WriteString(padLeft + dim.Render("Press Enter to confirm") + "\n")

	case StepWorkDir:
		b.WriteString(padLeft + "Working directory?\n\n")
		b.WriteString(padLeft + cyan.Render("> ") + o.Input + "█\n\n")
		b.WriteString(padLeft + dim.Render("Press Enter to confirm, Esc to skip") + "\n")

	case StepAPIKey:
		b.WriteString(padLeft + "API Key (Anthropic or OpenAI)?\n\n")
		masked := strings.Repeat("•", len(o.Input))
		if len(o.Input) > 8 {
			masked = o.Input[:4] + strings.Repeat("•", len(o.Input)-8) + o.Input[len(o.Input)-4:]
		}
		b.WriteString(padLeft + cyan.Render("> ") + masked + "█\n\n")
		b.WriteString(padLeft + dim.Render("Press Enter to confirm, Esc to skip") + "\n")

	case StepModel:
		b.WriteString(padLeft + "Select model:\n\n")
		b.WriteString(padLeft + cyan.Render("1") + " claude-sonnet-4-6 (recommended)\n")
		b.WriteString(padLeft + cyan.Render("2") + " claude-opus-4-6\n")
		b.WriteString(padLeft + cyan.Render("3") + " claude-haiku-4-5 (fast)\n\n")
		b.WriteString(padLeft + dim.Render("Press 1, 2, or 3") + "\n")

	case StepComplete:
		b.WriteString(padLeft + lipgloss.NewStyle().Foreground(theme.HydraGreen).Render("✅ Setup complete!") + "\n\n")
		b.WriteString(padLeft + fmt.Sprintf("Name:  %s\n", o.Profile.UserName))
		b.WriteString(padLeft + fmt.Sprintf("Model: %s\n", o.Profile.SelectedModel))
		if o.Profile.WorkingDirectory != nil {
			b.WriteString(padLeft + fmt.Sprintf("Dir:   %s\n", *o.Profile.WorkingDirectory))
		}
		b.WriteString("\n" + padLeft + dim.Render("Press Enter to start Hydra...") + "\n")
	}

	return b.String()
}
