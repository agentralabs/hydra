package client

import (
	"strconv"
	"strings"
	"time"
)

// RpcRequest is a JSON-RPC 2.0 request.
type RpcRequest struct {
	Jsonrpc string      `json:"jsonrpc"`
	Method  string      `json:"method"`
	Params  interface{} `json:"params"`
	ID      uint64      `json:"id"`
}

// RpcResponse is a JSON-RPC 2.0 response.
type RpcResponse struct {
	Result interface{} `json:"result,omitempty"`
	Error  *RpcError   `json:"error,omitempty"`
	ID     *uint64     `json:"id,omitempty"`
}

// RpcError is a JSON-RPC 2.0 error.
type RpcError struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Data    interface{} `json:"data,omitempty"`
}

// HealthInfo from hydra.health RPC.
type HealthInfo struct {
	Sisters       string  `json:"sisters"`
	SistersCount  uint32  `json:"sisters_count"`
	UptimeSecs    uint64  `json:"uptime_seconds"`
	Profile       *string `json:"profile"`
	BeliefsLoaded uint32  `json:"beliefs_loaded"`
	Engine        string  `json:"engine"`
	Status        string  `json:"status"`
	Model         *string `json:"model"`

	// Computed from sisters string
	SistersConnected uint32 `json:"-"`
	SistersTotal     uint32 `json:"-"`
	ToolsCount       uint32 `json:"-"`
}

func (h *HealthInfo) ComputeSisterCounts() {
	h.SistersTotal = h.SistersCount
	if h.SistersTotal == 0 { h.SistersTotal = 17 }
	if h.Sisters != "" && h.Sisters != "not initialized" {
		parts := strings.Split(h.Sisters, ",")
		h.SistersConnected = uint32(len(parts))
		// Count tools from "Name (N tools)" pattern
		h.ToolsCount = 0
		for _, p := range parts {
			p = strings.TrimSpace(p)
			if idx := strings.Index(p, "("); idx > 0 {
				numStr := strings.TrimRight(p[idx+1:], " tools)")
				if n, err := strconv.ParseUint(strings.TrimSpace(numStr), 10, 32); err == nil {
					h.ToolsCount += uint32(n)
				}
			}
		}
	}
}


// SisterStatus for health dashboard.
type SisterStatus struct {
	Name         string  `json:"name"`
	Tools        uint32  `json:"tools"`
	Connected    bool    `json:"connected"`
	LastActivity *string `json:"last_activity,omitempty"`
}

// ProfileInfo from profile.list.
type ProfileInfo struct {
	Name         string  `json:"name"`
	Identity     *string `json:"identity,omitempty"`
	BeliefsCount uint32  `json:"beliefs_count"`
	SkillsCount  uint32  `json:"skills_count"`
	Category     *string `json:"category,omitempty"`
	Active       bool    `json:"active"`
}

// RunResult from hydra.run.
type RunResult struct {
	RunID  string  `json:"run_id"`
	Status string  `json:"status"`
	Output *string `json:"output,omitempty"`
}

// StreamChunk from SSE.
type StreamChunk struct {
	RunID      *string `json:"run_id,omitempty"`
	Type       string  `json:"type"` // text, tool_start, tool_end, thinking, done, error
	Content    *string `json:"content,omitempty"`
	Sister     *string `json:"sister,omitempty"`
	Tool       *string `json:"tool,omitempty"`
	DurationMs *uint64 `json:"duration_ms,omitempty"`
}

// RoiSummary from hydra.roi.
type RoiSummary struct {
	ValueDelivered float64 `json:"value_delivered"`
	LLMCost        float64 `json:"llm_cost"`
	ROIMultiple    float64 `json:"roi_multiple"`
}

// ChatMessage for display.
type ChatMessage struct {
	ID           string
	Role         MessageRole
	Content      string
	Timestamp    time.Time
	ToolResults  []ToolResult
	BeliefsCited []BeliefCitation
}

// MessageRole enum.
type MessageRole int

const (
	RoleUser MessageRole = iota
	RoleAssistant
	RoleSystem
)

// ToolResult for display.
type ToolResult struct {
	Sister      string
	Action      string
	Output      string
	DurationMs  uint64
	Success     bool
	Expanded    bool
	ExpandedAt  time.Time // when Expanded was set to true
	DotCategory int       // maps to theme.DotCategory
}

// BeliefCitation in a response.
type BeliefCitation struct {
	Text        string
	Confidence  float64
	TimesTested uint32
}

// BriefingItem for morning briefing.
type BriefingItem struct {
	Priority BriefingPriority
	Text     string
}

// BriefingPriority levels.
type BriefingPriority int

const (
	PriorityUrgent BriefingPriority = iota
	PriorityImportant
	PriorityInfo
)

// BackgroundTask tracking.
type BackgroundTask struct {
	ID         string
	Name       string
	Sister     string
	Status     TaskStatus
	ElapsedSec uint64
	Cost       float64
}

// TaskStatus enum.
type TaskStatus int

const (
	TaskRunning TaskStatus = iota
	TaskCompleted
	TaskFailed
	TaskIdle
)

// PendingApproval prompt.
type PendingApproval struct {
	RunID       string
	Description string
	Risk        string // LOW, MEDIUM, HIGH, CRITICAL
	FilePath    string
	DiffSummary string
}

// FileChange tracked in a session.
type FileChange struct {
	FilePath     string
	ChangeType   string // created, edited, deleted
	LinesAdded   int
	LinesRemoved int
	Sister       string
	Risk         string
	OldContent   string
	Timestamp    time.Time
}
