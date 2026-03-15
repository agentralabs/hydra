package app

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// ProjectKind identifies the type of project.
type ProjectKind int

const (
	ProjectUnknown ProjectKind = iota
	ProjectRust
	ProjectNode
	ProjectGo
	ProjectPython
)

// ProjectInfo holds detected project information.
type ProjectInfo struct {
	Kind       ProjectKind
	Root       string
	Name       string
	CrateCount int    // Rust workspaces
	GitBranch  string
	GitAhead   int
	GitBehind  int
}

// DetectProject scans the working directory for project markers.
func DetectProject() *ProjectInfo {
	dir, err := os.Getwd()
	if err != nil {
		return nil
	}

	info := &ProjectInfo{Root: dir}

	// Check in order of priority
	if fileExists("Cargo.toml") {
		info.Kind = ProjectRust
		info.Name = extractCargoName()
		info.CrateCount = countCrates()
	} else if fileExists("package.json") {
		info.Kind = ProjectNode
		info.Name = extractPackageName()
	} else if fileExists("go.mod") {
		info.Kind = ProjectGo
		info.Name = extractGoModName()
	} else if fileExists("pyproject.toml") || fileExists("setup.py") || fileExists("requirements.txt") {
		info.Kind = ProjectPython
		info.Name = filepath.Base(dir)
	} else {
		info.Kind = ProjectUnknown
		info.Name = filepath.Base(dir)
	}

	// Git info
	info.GitBranch = detectGitBranch()
	info.GitAhead, info.GitBehind = detectGitAheadBehind()

	return info
}

// TestCommand returns the appropriate test command for the project type.
func (p *ProjectInfo) TestCommand() (string, []string) {
	switch p.Kind {
	case ProjectRust:
		return "cargo", []string{"test", "-j", "1"}
	case ProjectNode:
		return "npm", []string{"test"}
	case ProjectGo:
		return "go", []string{"test", "./..."}
	case ProjectPython:
		return "python", []string{"-m", "pytest"}
	}
	return "echo", []string{"No test command detected"}
}

// BuildCommand returns the build command for the project type.
func (p *ProjectInfo) BuildCommand() (string, []string) {
	switch p.Kind {
	case ProjectRust:
		return "cargo", []string{"build", "-j", "1"}
	case ProjectNode:
		return "npm", []string{"run", "build"}
	case ProjectGo:
		return "go", []string{"build", "./..."}
	case ProjectPython:
		return "python", []string{"-m", "build"}
	}
	return "echo", []string{"No build command detected"}
}

// LintCommand returns the lint command for the project type.
func (p *ProjectInfo) LintCommand() (string, []string) {
	switch p.Kind {
	case ProjectRust:
		return "cargo", []string{"clippy", "-j", "1"}
	case ProjectNode:
		return "npx", []string{"eslint", "."}
	case ProjectGo:
		return "golangci-lint", []string{"run"}
	case ProjectPython:
		return "ruff", []string{"check", "."}
	}
	return "echo", []string{"No lint command detected"}
}

// FmtCommand returns the format command for the project type.
func (p *ProjectInfo) FmtCommand() (string, []string) {
	switch p.Kind {
	case ProjectRust:
		return "cargo", []string{"fmt"}
	case ProjectNode:
		return "npx", []string{"prettier", "--write", "."}
	case ProjectGo:
		return "gofmt", []string{"-w", "."}
	case ProjectPython:
		return "ruff", []string{"format", "."}
	}
	return "echo", []string{"No format command detected"}
}

func fileExists(name string) bool {
	_, err := os.Stat(name)
	return err == nil
}

func extractCargoName() string {
	data, err := os.ReadFile("Cargo.toml")
	if err != nil {
		return "rust-project"
	}
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "name") && strings.Contains(line, "=") {
			parts := strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				return strings.Trim(strings.TrimSpace(parts[1]), "\"")
			}
		}
	}
	return "rust-project"
}

func extractPackageName() string {
	data, err := os.ReadFile("package.json")
	if err != nil {
		return "node-project"
	}
	// Simple extraction — avoid full JSON parse
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "\"name\"") {
			parts := strings.SplitN(line, ":", 2)
			if len(parts) == 2 {
				return strings.Trim(strings.TrimSpace(parts[1]), "\",")
			}
		}
	}
	return "node-project"
}

func extractGoModName() string {
	data, err := os.ReadFile("go.mod")
	if err != nil {
		return "go-project"
	}
	for _, line := range strings.Split(string(data), "\n") {
		if strings.HasPrefix(line, "module ") {
			return strings.TrimPrefix(line, "module ")
		}
	}
	return "go-project"
}

func countCrates() int {
	entries, err := os.ReadDir("crates")
	if err != nil {
		return 0
	}
	count := 0
	for _, e := range entries {
		if e.IsDir() {
			count++
		}
	}
	return count
}

func detectGitAheadBehind() (int, int) {
	out, err := exec.Command("git", "rev-list", "--left-right", "--count", "HEAD...@{upstream}").Output()
	if err != nil {
		return 0, 0
	}
	parts := strings.Fields(strings.TrimSpace(string(out)))
	if len(parts) != 2 {
		return 0, 0
	}
	ahead, behind := 0, 0
	_, _ = fmt.Sscanf(parts[0], "%d", &ahead)
	_, _ = fmt.Sscanf(parts[1], "%d", &behind)
	return ahead, behind
}
