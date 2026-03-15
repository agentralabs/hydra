package app

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
)

// PersistedProfile matches ~/.hydra/profile.json
type PersistedProfile struct {
	UserName               string  `json:"user_name"`
	VoiceEnabled           bool    `json:"voice_enabled"`
	OnboardingComplete     bool    `json:"onboarding_complete"`
	SelectedModel          string  `json:"selected_model"`
	APIKey                 *string `json:"api_key,omitempty"`
	AnthropicAPIKey        *string `json:"anthropic_api_key,omitempty"`
	OpenAIAPIKey           *string `json:"openai_api_key,omitempty"`
	GoogleAPIKey           *string `json:"google_api_key,omitempty"`
	Theme                  string  `json:"theme"`
	AutoApproveFlag        bool    `json:"auto_approve"`
	DefaultMode            string  `json:"default_mode"`
	SoundsEnabled          bool    `json:"sounds_enabled"`
	SoundVolume            int     `json:"sound_volume"`
	WorkingDirectory       *string `json:"working_directory,omitempty"`
	AutonomyLevel          string  `json:"autonomy_level"`
	MemoryCapture          string  `json:"memory_capture"`
	SMTPHost               *string `json:"smtp_host,omitempty"`
	SMTPUser               *string `json:"smtp_user,omitempty"`
	SMTPPassword           *string `json:"smtp_password,omitempty"`
	SMTPTo                 *string `json:"smtp_to,omitempty"`
	ActiveOperationalProfile *string `json:"active_operational_profile,omitempty"`
}

// HydraSettings matches .hydra/settings.json (project-level)
type HydraSettings struct {
	Model     *string          `json:"model,omitempty"`
	MaxTokens *int             `json:"maxTokens,omitempty"`
	FastMode  bool             `json:"fastMode"`
	Perms     *PermSettings    `json:"permissions,omitempty"`
}

type PermSettings struct {
	AllowedTools []string `json:"allowedTools,omitempty"`
	Deny         []string `json:"deny,omitempty"`
}

// HydraDir returns ~/.hydra/ path.
func HydraDir() string {
	home, _ := os.UserHomeDir()
	return filepath.Join(home, ".hydra")
}

// LoadProfile loads the user profile from ~/.hydra/profile.json
func LoadProfile() (*PersistedProfile, error) {
	dir := HydraDir()

	// Check for multi-user: ~/.hydra/active
	activePath := filepath.Join(dir, "active")
	if data, err := os.ReadFile(activePath); err == nil {
		username := strings.TrimSpace(string(data))
		if username != "" {
			userProfile := filepath.Join(dir, "users", username, "profile.json")
			if p, err := loadProfileFrom(userProfile); err == nil {
				return p, nil
			}
		}
	}

	// Fallback to legacy path
	return loadProfileFrom(filepath.Join(dir, "profile.json"))
}

func loadProfileFrom(path string) (*PersistedProfile, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var p PersistedProfile
	if err := json.Unmarshal(data, &p); err != nil {
		return nil, err
	}
	return &p, nil
}

// SaveProfile saves the profile to ~/.hydra/profile.json
func SaveProfile(p *PersistedProfile) error {
	dir := HydraDir()
	_ = os.MkdirAll(dir, 0755)

	data, err := json.MarshalIndent(p, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(filepath.Join(dir, "profile.json"), data, 0644)
}

// LoadSettings loads project-level settings from .hydra/settings.json
func LoadSettings() *HydraSettings {
	data, err := os.ReadFile(".hydra/settings.json")
	if err != nil {
		return &HydraSettings{}
	}
	var s HydraSettings
	if err := json.Unmarshal(data, &s); err != nil {
		return &HydraSettings{}
	}
	return &s
}

// ApplyProfile applies profile settings to the app model and env.
func ApplyProfile(m *Model, p *PersistedProfile) {
	if p.UserName != "" {
		m.Username = p.UserName
	}
	if p.SelectedModel != "" {
		m.ModelName = p.SelectedModel
	}
	if p.MemoryCapture != "" {
		m.MemoryMode = p.MemoryCapture
	}
	if p.AutoApproveFlag {
		m.AutoApprove = true
	}
	if p.ActiveOperationalProfile != nil {
		m.ProfileName = *p.ActiveOperationalProfile
	}
	m.FastMode = p.SelectedModel == "claude-haiku-4-5"

	// Set env vars for sisters/server
	if p.AnthropicAPIKey != nil && *p.AnthropicAPIKey != "" {
		os.Setenv("ANTHROPIC_API_KEY", *p.AnthropicAPIKey)
	}
	if p.OpenAIAPIKey != nil && *p.OpenAIAPIKey != "" {
		os.Setenv("OPENAI_API_KEY", *p.OpenAIAPIKey)
	}
	if p.SelectedModel != "" {
		os.Setenv("HYDRA_MODEL", p.SelectedModel)
	}
	if p.WorkingDirectory != nil && *p.WorkingDirectory != "" {
		expanded := expandHome(*p.WorkingDirectory)
		_ = os.Chdir(expanded)
		m.ProjectPath = detectProjectPath()
		m.GitBranch = detectGitBranch()
	}
}

// ApplySettings applies project-level settings.
func ApplySettings(m *Model, s *HydraSettings) {
	if s.Model != nil && *s.Model != "" {
		m.ModelName = *s.Model
	}
	m.FastMode = s.FastMode
}

func expandHome(path string) string {
	if strings.HasPrefix(path, "~/") {
		home, _ := os.UserHomeDir()
		return filepath.Join(home, path[2:])
	}
	return path
}
