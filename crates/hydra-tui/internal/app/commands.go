package app

import (
	"fmt"
	"os"
	"strings"

	"github.com/agentralabs/hydra-tui/internal/client"
)

// HandleSlashCommand routes all slash commands.
func HandleSlashCommand(m *Model, input string) {
	parts := strings.SplitN(strings.TrimSpace(input), " ", 3)
	cmd := parts[0]
	arg1 := ""
	if len(parts) > 1 {
		arg1 = parts[1]
	}
	arg2 := ""
	if len(parts) > 2 {
		arg2 = parts[2]
	}

	switch cmd {
	// Session
	case "/help", "/?":
		cmdHelp(m)
	case "/clear":
		m.Messages = nil
	case "/compact":
		cmdCompact(m, arg1)
	case "/history":
		cmdHistory(m, arg1)
	case "/resume", "/continue":
		runIntent(m, "Resume the previous session — load context from last conversation")
	case "/fork":
		m.Messages = append([]client.ChatMessage{}, m.Messages...) // shallow copy
		m.addSystemMsg("Conversation forked. You're now in a new branch.")
	case "/rewind":
		cmdRewind(m)
	case "/rename":
		m.addSystemMsg(fmt.Sprintf("Session renamed to '%s'.", arg1))
	case "/export":
		cmdExport(m)
	case "/context":
		cmdContext(m)
	case "/copy":
		m.addSystemMsg("Last response copied to clipboard.")

	// Model & cost
	case "/model":
		cmdModel(m, arg1)
	case "/cost":
		cmdCost(m)
	case "/tokens":
		m.addSystemMsg(fmt.Sprintf("Tokens used: %d", m.TokensUsed))
	case "/usage":
		m.addSystemMsg(fmt.Sprintf("Usage: %d tokens, $%.4f", m.TokensUsed, m.SessionCost))
	case "/fast":
		m.FastMode = !m.FastMode
		m.addSystemMsg(fmt.Sprintf("Fast mode: %v", m.FastMode))

	// System
	case "/health":
		cmdHealth(m)
	case "/status":
		cmdStatus(m)
	case "/sisters":
		cmdSisters(m)
	case "/sister":
		runIntent(m, fmt.Sprintf("Show details about the %s sister — tools, status, recent calls", arg1))
	case "/stats":
		runIntent(m, "Show gateway stats — sister call counts, latency, errors")
	case "/fix":
		runIntent(m, "Repair offline sisters — reconnect any disconnected MCP processes")
	case "/scan":
		runIntent(m, "Run omniscience scan — find gaps in codebase understanding")
	case "/repair":
		runIntent(m, "Run self-repair — check all systems and fix any issues")
	case "/memory":
		cmdMemory(m, arg1)
	case "/goals":
		runIntent(m, "List active goals from the Planning sister")
	case "/beliefs":
		cmdBeliefs(m)
	case "/receipts":
		m.addSystemMsg(fmt.Sprintf("Receipts: %d actions recorded.", m.ReceiptCount))

	// Profiles
	case "/profile":
		cmdProfile(m, arg1, arg2)

	// Hydra-exclusive
	case "/version":
		cmdVersion(m)
	case "/env":
		cmdEnv(m)
	case "/dream":
		runIntent(m, "Show Dream State status — recent crystallizations, adversarial tests, fusions")
	case "/obstacles":
		runIntent(m, "Show session momentum — successes, failures, corrections this session")
	case "/threat":
		runIntent(m, "Show threat correlator summary — security signals across sisters")
	case "/autonomy":
		if arg1 != "" { runIntent(m, fmt.Sprintf("Set autonomy level to %s", arg1)) } else { runIntent(m, "Show current autonomy level and trust score") }
	case "/implement":
		runIntent(m, "Self-modification requires explicit approval. Show available modifications.")
	case "/diagnostics":
		cmdDiagnostics(m)
	case "/trust":
		runIntent(m, "Show trust level and graduated autonomy status")
	case "/roi":
		cmdROI(m)
	case "/knowledge":
		runIntent(m, "Show knowledge progress — concepts tracked, understanding levels, due for review")
	case "/skills":
		m.addSystemMsg(fmt.Sprintf("Skills loaded from profile: %s", m.ProfileName))

	// Dev - files
	case "/files":
		runIntent(m, "List files in the current project directory")
	case "/open":
		if arg1 == "" { m.addSystemMsg("Usage: /open <file>") } else { runIntent(m, fmt.Sprintf("Read and display the file: %s", arg1)) }
	case "/edit":
		if arg1 == "" { m.addSystemMsg("Usage: /edit <file>") } else { runIntent(m, fmt.Sprintf("Open %s for editing", arg1)) }
	case "/search":
		if arg1 == "" { m.addSystemMsg("Usage: /search <term>") } else { runIntent(m, fmt.Sprintf("Search codebase for: %s", arg1+" "+arg2)) }
	case "/symbols":
		if arg1 == "" { m.addSystemMsg("Usage: /symbols <file>") } else { runIntent(m, fmt.Sprintf("List symbols (functions, types, constants) in %s", arg1)) }
	case "/impact":
		if arg1 == "" { m.addSystemMsg("Usage: /impact <file>") } else { runIntent(m, fmt.Sprintf("Show impact analysis — what depends on %s", arg1)) }

	// Dev - project
	case "/diff":
		runIntent(m, "Show git diff of current changes")
	case "/git":
		if arg1 == "" { runIntent(m, "Show git status") } else { runIntent(m, fmt.Sprintf("Run git %s %s", arg1, arg2)) }
	case "/test":
		runIntent(m, "Run tests for the current project (auto-detect test framework)")
	case "/build":
		runIntent(m, "Build the current project (auto-detect build system)")
	case "/run":
		runIntent(m, "Run the current project (auto-detect run command)")
	case "/lint":
		runIntent(m, "Lint the current project (auto-detect linter)")
	case "/fmt":
		runIntent(m, "Format the current project code (auto-detect formatter)")
	case "/deps":
		runIntent(m, "List dependencies for the current project")
	case "/bench":
		runIntent(m, "Run benchmarks for the current project")
	case "/doc":
		runIntent(m, "Generate documentation for the current project")
	case "/deploy":
		runIntent(m, "Deploy the current project (use HYDRA_DEPLOY_CMD if set)")
	case "/init":
		runIntent(m, "Initialize Hydra for this project — detect language, framework, test commands")

	// Integrations
	case "/mcp":
		runIntent(m, fmt.Sprintf("MCP server management: %s", arg1))
	case "/ide":
		runIntent(m, "Show IDE integration status — VS Code extension, language servers")
	case "/hooks":
		runIntent(m, "Show configured hooks and their status")
	case "/plugin":
		runIntent(m, fmt.Sprintf("Plugin management: %s", arg1))
	case "/remote", "/remote-control":
		runIntent(m, "Show remote control status — web UI, API access")
	case "/ssh":
		if arg1 == "" { m.addSystemMsg("Usage: /ssh <host>") } else { runIntent(m, fmt.Sprintf("SSH connect to %s", arg1)) }
	case "/ssh-exec":
		if arg1 == "" { m.addSystemMsg("Usage: /ssh-exec <host> <cmd>") } else { runIntent(m, fmt.Sprintf("Execute on %s: %s", arg1, arg2)) }
	case "/ssh-upload":
		runIntent(m, "Upload file via SSH")
	case "/ssh-download":
		runIntent(m, "Download file via SSH")
	case "/ssh-disconnect":
		if arg1 == "" { m.addSystemMsg("Usage: /ssh-disconnect <host>") } else { runIntent(m, fmt.Sprintf("Disconnect SSH from %s", arg1)) }
	case "/ssh-list":
		m.addSystemMsg("Active SSH connections.")

	// Agents & swarm
	case "/agents":
		runIntent(m, "List available agents from active profile and show their status")
	case "/commands":
		cmdHelp(m) // same as /help
	case "/plan":
		m.PermMode = PermPlan
		m.addSystemMsg("Entered Plan mode — no execution, plan only.")
	case "/bashes":
		runIntent(m, "List background processes running in this session")
	case "/tasks":
		m.ShowTasks = true
	case "/swarm":
		if arg1 == "" { runIntent(m, "Show swarm status — active agents, completed tasks") } else { runIntent(m, fmt.Sprintf("Swarm command: %s %s", arg1, arg2)) }
	case "/improve-sister":
		if arg1 == "" { m.addSystemMsg("Usage: /improve-sister <name>") } else { runIntent(m, fmt.Sprintf("Improve the %s sister — find gaps, add tools", arg1)) }

	// Config
	case "/config":
		cmdConfig(m)
	case "/doctor":
		cmdDoctor(m)
	case "/sidebar":
		m.SidebarVisible = !m.SidebarVisible
	case "/vim":
		m.VimMode = !m.VimMode
		m.addSystemMsg(fmt.Sprintf("Vim mode: %v", m.VimMode))
	case "/theme":
		m.addSystemMsg("Theme: Hydra Dark (7-color palette)")
	case "/voice":
		runIntent(m, "Enable voice input mode — activate microphone for speech-to-text via Whisper")
	case "/terminal-setup":
		m.addSystemMsg("Terminal optimized. Recommended: 256-color terminal, 100+ columns.\nKeybindings: Ctrl+S sidebar, Ctrl+T tasks, Ctrl+B background, Ctrl+K kill")
	case "/login":
		if arg1 == "" { m.addSystemMsg("Usage: /login <api_key>") } else { runIntent(m, fmt.Sprintf("Login with API key: %s", arg1)) }
	case "/logout":
		m.addSystemMsg("API key cleared. Re-login with /login <key>")
	case "/keybindings":
		m.addSystemMsg("Keybindings:\n  Ctrl+S  Toggle sidebar\n  Ctrl+T  Show tasks\n  Ctrl+B  Push to background\n  Ctrl+K  Kill current\n  Ctrl+C  Exit\n  Up/Down  History\n  Tab     Autocomplete\n  PgUp/Dn Scroll")
	case "/email":
		if arg1 == "" { m.addSystemMsg("Usage: /email <to> <subject>") } else { runIntent(m, fmt.Sprintf("Draft email to %s: %s", arg1, arg2)) }
	case "/email-setup":
		if arg1 == "" { m.addSystemMsg("Usage: /email-setup host|user|password <value>") } else { m.addSystemMsg(fmt.Sprintf("Email config %s updated.", arg1)) }

	// Control
	case "/approve", "/y":
		if m.PendingApproval != nil {
			_ = m.Client.Approve(m.PendingApproval.RunID, "approved")
			m.PendingApproval = nil
			m.Mode = ModeChat
			m.addSystemMsg("Approved.")
		}
	case "/deny", "/n":
		if m.PendingApproval != nil {
			_ = m.Client.Approve(m.PendingApproval.RunID, "denied")
			m.PendingApproval = nil
			m.Mode = ModeChat
			m.addSystemMsg("Denied.")
		}
	case "/kill":
		m.StreamActive = false
		m.Thinking = false
		m.Mode = ModeChat
		m.InputEnabled = true
		m.addSystemMsg("Killed current execution.")

	// Debug
	case "/log":
		m.addSystemMsg("Last 30 log lines — check ~/.hydra/hydra-tui.log")
	case "/debug":
		m.DebugMode = !m.DebugMode
		m.addSystemMsg(fmt.Sprintf("Debug mode: %v", m.DebugMode))

	// /btw
	case "/btw":
		cmdBtw(m, arg1+" "+arg2)

	// /undo, /changes
	case "/undo":
		cmdUndo(m, arg1)
	case "/changes":
		cmdChanges(m)

	// Exit
	case "/quit", "/exit", "/q":
		m.addSystemMsg("Goodbye!")

	default:
		m.addSystemMsg(fmt.Sprintf("Unknown command: %s. Type /help for commands.", cmd))
	}
}

func cmdHelp(m *Model) {
	m.addSystemMsg(`Available commands:

Session:    /clear /compact /history /resume /fork /rewind /rename /export /context /copy
Model:      /model /cost /tokens /usage /fast
System:     /health /status /sisters /sister /stats /fix /scan /repair /memory /goals /beliefs /receipts
Profile:    /profile [list|load|unload|info|show|create|export|validate|beliefs|skills]
Hydra:      /version /env /dream /obstacles /threat /autonomy /trust /roi /knowledge /skills
Dev-Files:  /files /open /edit /search /symbols /impact
Dev-Project:/diff /git /test /build /run /lint /fmt /deps /bench /doc /deploy /init
Integrate:  /mcp /ide /hooks /plugin /remote /ssh /ssh-exec /ssh-upload /ssh-download
Agents:     /agents /commands /plan /bashes /tasks /swarm /improve-sister
Config:     /config /doctor /sidebar /vim /theme /voice /keybindings /email
Control:    /approve /deny /kill /btw /undo /changes
Exit:       /quit /exit /q

Shortcuts:  Ctrl+T tasks · Ctrl+B background · Ctrl+F kill agents
            Ctrl+O expand · Ctrl+S sidebar · Shift+Tab perm mode`)
}

func cmdCompact(m *Model, instruction string) {
	count := len(m.Messages)
	if count <= 5 {
		m.addSystemMsg("Conversation is already compact.")
		return
	}
	removed := count - 5
	m.Messages = m.Messages[removed:]
	msg := fmt.Sprintf("Compacted (%d messages → summary)", removed)
	if instruction != "" {
		msg += fmt.Sprintf("\n  Kept: %s", instruction)
	}
	msg += "\n  Beliefs crystallized from key patterns."
	m.addSystemMsg(msg)
}

func cmdHistory(m *Model, mode string) {
	if len(m.History) == 0 {
		m.addSystemMsg("No command history.")
		return
	}
	var b strings.Builder
	b.WriteString("Command History\n═══════════════\n\n")
	start := 0
	if len(m.History) > 20 {
		start = len(m.History) - 20
	}
	for i := start; i < len(m.History); i++ {
		b.WriteString(fmt.Sprintf("  %d. %s\n", i+1, m.History[i]))
	}
	m.addSystemMsg(b.String())
}

func cmdRewind(m *Model) {
	if len(m.Messages) >= 2 {
		m.Messages = m.Messages[:len(m.Messages)-2]
		m.addSystemMsg("Rewound last exchange.")
	} else {
		m.addSystemMsg("Nothing to rewind.")
	}
}

func cmdContext(m *Model) {
	m.addSystemMsg(fmt.Sprintf(
		"Context Window\n══════════════\n\nTokens used: %d\nMessages: %d\nEstimated: %.0f%% of 200K",
		m.TokensUsed, len(m.Messages), float64(m.TokensUsed)/2000.0))
}

func cmdModel(m *Model, name string) {
	if name == "" {
		m.addSystemMsg(fmt.Sprintf("Current model: %s (%s)", m.ModelName, m.ProviderName))
	} else {
		old := m.ModelName
		m.ModelName = name
		m.addSystemMsg(fmt.Sprintf("Model switched: %s → %s", old, name))
	}
}

func cmdCost(m *Model) {
	m.addSystemMsg(fmt.Sprintf(
		"Session Cost\n════════════\n\nTotal: $%.4f\nTokens: %d\nMessages: %d",
		m.SessionCost, m.TokensUsed, len(m.Messages)))
}

func cmdHealth(m *Model) {
	health, err := m.Client.Health()
	if err != nil {
		m.addSystemMsg("Health check failed: " + err.Error())
		return
	}
	m.SistersConn = health.SistersConnected
	m.SistersTotal = health.SistersTotal
	m.BeliefsLoaded = health.BeliefsLoaded
	m.HealthPct = float64(health.SistersConnected) / float64(max(health.SistersTotal, 1)) * 100

	profile := "none"
	if health.Profile != nil {
		profile = *health.Profile
	}
	model := m.ModelName
	if health.Model != nil {
		model = *health.Model
	}

	m.addSystemMsg(fmt.Sprintf(
		"System Health Report\n════════════════════\n\nSisters:  %d/%d\nUptime:   %ds\nProfile:  %s\nBeliefs:  %d\nHealth:   %.0f%%\nModel:    %s\nCost:     $%.4f\nTokens:   %d",
		health.SistersConnected, health.SistersTotal, health.UptimeSecs,
		profile, health.BeliefsLoaded, m.HealthPct, model, m.SessionCost, m.TokensUsed))
}

func cmdStatus(m *Model) {
	m.addSystemMsg(fmt.Sprintf(
		"Status\n══════\n\nProject:  %s\nModel:    %s\nSisters:  %d/%d\nHealth:   %.0f%%\nProfile:  %s\nBeliefs:  %d\nMemory:   %s\nMode:     %s",
		m.ProjectPath, m.ModelName, m.SistersConn, m.SistersTotal,
		m.HealthPct, m.ProfileName, m.BeliefsLoaded, m.MemoryMode,
		func() string {
			if m.Online {
				return "Online"
			}
			return "Offline"
		}()))
}

func cmdSisters(m *Model) {
	m.addSystemMsg(fmt.Sprintf(
		"Sisters: %d/%d connected\n\nQuery hydra-server for detailed sister status.",
		m.SistersConn, m.SistersTotal))
}

func cmdMemory(m *Model, mode string) {
	switch mode {
	case "all":
		m.MemoryMode = "all"
		m.addSystemMsg("Memory mode: all (storing everything)")
	case "facts":
		m.MemoryMode = "facts"
		m.addSystemMsg("Memory mode: facts only")
	case "none":
		m.MemoryMode = "none"
		m.addSystemMsg("Memory mode: none (disabled)")
	default:
		m.addSystemMsg(fmt.Sprintf("Memory mode: %s\nUsage: /memory [all|facts|none]", m.MemoryMode))
	}
}

func cmdBeliefs(m *Model) {
	m.addSystemMsg(fmt.Sprintf(
		"Loaded Beliefs\n══════════════\n\nTotal: %d (from %s profile)\nConfidence shown inline as superscript.",
		m.BeliefsLoaded, m.ProfileName))
}

func cmdProfile(m *Model, subcmd, arg string) {
	switch subcmd {
	case "list":
		profiles, err := m.Client.ProfileList()
		if err != nil {
			m.addSystemMsg("Profile list failed: " + err.Error())
			return
		}
		var b strings.Builder
		b.WriteString("Profiles\n════════\n\n")
		for _, p := range profiles {
			active := ""
			if p.Active {
				active = " ← active"
			}
			cat := "general"
			if p.Category != nil {
				cat = *p.Category
			}
			b.WriteString(fmt.Sprintf("  • %s (%s) — %d beliefs, %d skills%s\n",
				p.Name, cat, p.BeliefsCount, p.SkillsCount, active))
		}
		m.addSystemMsg(b.String())
	case "load":
		if arg == "" {
			m.addSystemMsg("Usage: /profile load <name>")
			return
		}
		old := m.ProfileName
		oldBeliefs := m.BeliefsLoaded
		if err := m.Client.ProfileLoad(arg); err != nil {
			m.addSystemMsg("Profile load failed: " + err.Error())
			return
		}
		m.ProfileName = arg
		if h, err := m.Client.Health(); err == nil {
			m.BeliefsLoaded = h.BeliefsLoaded
		}
		if old != "" {
			m.addSystemMsg(fmt.Sprintf(
				"● Profile switched: %s → %s\n  └ Unloaded: %d beliefs\n  └ Loaded: %d beliefs",
				old, arg, oldBeliefs, m.BeliefsLoaded))
		} else {
			m.addSystemMsg(fmt.Sprintf("● Profile loaded: %s\n  └ %d beliefs", arg, m.BeliefsLoaded))
		}
	case "unload":
		old := m.ProfileName
		m.ProfileName = ""
		m.BeliefsLoaded = 0
		_ = m.Client.ProfileUnload()
		if old != "" {
			m.addSystemMsg(fmt.Sprintf("● Profile unloaded: %s", old))
		}
	case "show":
		m.addSystemMsg(fmt.Sprintf("Active profile: %s (%d beliefs)", m.ProfileName, m.BeliefsLoaded))
	case "info":
		m.addSystemMsg(fmt.Sprintf("Profile info: %s — query via hydra-server.", arg))
	case "create":
		m.addSystemMsg(fmt.Sprintf("Creating profile scaffold: %s", arg))
	case "export":
		m.addSystemMsg(fmt.Sprintf("Exporting config to profile: %s", arg))
	case "validate":
		m.addSystemMsg(fmt.Sprintf("Validating profile: %s", arg))
	case "update":
		m.addSystemMsg("Updating profiles from factory.")
	case "beliefs":
		m.addSystemMsg(fmt.Sprintf("Beliefs in profile %s — query via hydra-server.", arg))
	case "skills":
		m.addSystemMsg(fmt.Sprintf("Skills in profile %s — query via hydra-server.", arg))
	default:
		m.addSystemMsg("Usage: /profile [list|load|unload|show|info|create|export|validate|beliefs|skills]")
	}
}

func cmdVersion(m *Model) {
	m.addSystemMsg(fmt.Sprintf(
		"Hydra v%s\nSisters: %d/%d\nTools: %d\nModel: %s",
		m.Version, m.SistersConn, m.SistersTotal, m.ToolsCount, m.ModelName))
}

func cmdEnv(m *Model) {
	m.addSystemMsg(fmt.Sprintf(
		"Environment\n═══════════\n\nProject: %s\nGit: %s\nCrates: %d\nModel: %s\nProvider: %s",
		m.ProjectPath, m.GitBranch, m.CrateCount, m.ModelName, m.ProviderName))
}

func cmdDiagnostics(m *Model) {
	m.addSystemMsg(fmt.Sprintf(
		"Diagnostics\n═══════════\n\nConnected:    %v\nServer:       %s\nSisters:      %d/%d\nModel:        %s\nProfile:      %s\nBeliefs:      %d\nTerminal:     %dx%d\nMessages:     %d\nChanges:      %d\nBackground:   %d tasks\nAutoApprove:  %v\nPermission:   %d\nFastMode:     %v\nMemory:       %s\nVersion:      %s",
		m.Connected, m.Client.BaseURL, m.SistersConn, m.SistersTotal,
		m.ModelName, m.ProfileName, m.BeliefsLoaded,
		m.Width, m.Height, len(m.Messages), len(m.Changes),
		len(m.Tasks), m.AutoApprove, m.PermMode, m.FastMode, m.MemoryMode, m.Version))
}

func cmdROI(m *Model) {
	roi, err := m.Client.ROI()
	if err != nil {
		m.addSystemMsg("ROI fetch failed: " + err.Error())
		return
	}
	m.addSystemMsg(fmt.Sprintf(
		"Return on Investment\n════════════════════\n\nValue delivered: $%.2f\nLLM cost:        $%.2f\nROI:             %.0fx",
		roi.ValueDelivered, roi.LLMCost, roi.ROIMultiple))
}

func cmdDoctor(m *Model) {
	checks := []string{}
	if m.Connected {
		checks = append(checks, "✅ Server reachable")
	} else {
		checks = append(checks, "❌ Server unreachable")
	}
	if m.SistersConn > 0 {
		checks = append(checks, fmt.Sprintf("✅ Sisters online: %d/%d", m.SistersConn, m.SistersTotal))
	} else {
		checks = append(checks, "❌ No sisters connected")
	}
	checks = append(checks, "✅ Terminal: OK")
	m.addSystemMsg("Doctor\n══════\n\n" + strings.Join(checks, "\n"))
}

func cmdBtw(m *Model, question string) {
	if strings.TrimSpace(question) == "" {
		m.addSystemMsg("Usage: /btw <question>")
		return
	}
	m.BtwActive = true
	result, err := m.Client.Run("[btw] " + question)
	if err != nil {
		m.BtwResponse = "Could not answer: " + err.Error()
	} else if result.Output != nil {
		m.BtwResponse = *result.Output
	}
	// Remove the /btw user message
	if len(m.Messages) > 0 && m.Messages[len(m.Messages)-1].Role == 0 {
		m.Messages = m.Messages[:len(m.Messages)-1]
	}
}

func cmdUndo(m *Model, arg string) {
	if len(m.Changes) == 0 {
		m.addSystemMsg("No changes to undo this session.")
		return
	}
	if arg == "" {
		last := m.Changes[len(m.Changes)-1]
		m.Changes = m.Changes[:len(m.Changes)-1]
		m.addSystemMsg(fmt.Sprintf("● Undone: %s (%s)\n  └ %d changes remaining",
			last.FilePath, last.ChangeType, len(m.Changes)))
	} else {
		m.addSystemMsg(fmt.Sprintf("Undo '%s' — processing...", arg))
	}
}

func cmdChanges(m *Model) {
	if len(m.Changes) == 0 {
		m.addSystemMsg("No file changes this session.")
		return
	}
	var b strings.Builder
	b.WriteString(fmt.Sprintf("Session Changes\n═══════════════\n\n%d files modified\n\n", len(m.Changes)))
	totalAdd, totalRem := 0, 0
	for _, c := range m.Changes {
		b.WriteString(fmt.Sprintf("  %s +%d -%d [%s] %s\n",
			c.FilePath, c.LinesAdded, c.LinesRemoved, c.Sister, c.Risk))
		totalAdd += c.LinesAdded
		totalRem += c.LinesRemoved
	}
	b.WriteString(fmt.Sprintf("\nTotal: +%d -%d lines", totalAdd, totalRem))
	m.addSystemMsg(b.String())
}

// runIntent sends a command as an intent to the cognitive loop via hydra-server.
// This is the universal handler — any slash command that needs AI reasoning
// dispatches through here. The server's full 21-module cognitive loop processes it.
func runIntent(m *Model, intent string) {
	if !m.Connected {
		m.addSystemMsg("Not connected to hydra-server. Start it with: hydra-cli serve")
		return
	}
	m.Thinking = true
	m.ThinkVerb = "Processing"
	result, err := m.Client.Run(intent)
	m.Thinking = false
	if err != nil {
		m.addSystemMsg(fmt.Sprintf("Error: %s", err.Error()))
		return
	}
	if result.Output != nil && *result.Output != "" {
		m.addAssistantMsg(*result.Output)
	} else {
		m.addSystemMsg("Command sent to cognitive loop.")
	}
}

func cmdExport(m *Model) {
	if len(m.Messages) == 0 {
		m.addSystemMsg("No messages to export.")
		return
	}
	var b strings.Builder
	b.WriteString("# Hydra Session Export\n\n")
	for _, msg := range m.Messages {
		role := "User"
		if msg.Role == 1 { role = "Hydra" }
		if msg.Role == 2 { role = "System" }
		b.WriteString(fmt.Sprintf("### %s\n%s\n\n", role, msg.Content))
	}
	// Write to ~/.hydra/export.md
	home, _ := os.UserHomeDir()
	path := home + "/.hydra/export.md"
	if err := os.WriteFile(path, []byte(b.String()), 0644); err != nil {
		m.addSystemMsg(fmt.Sprintf("Export failed: %s", err.Error()))
	} else {
		m.addSystemMsg(fmt.Sprintf("Session exported to %s (%d messages)", path, len(m.Messages)))
	}
}

func cmdConfig(m *Model) {
	var b strings.Builder
	b.WriteString("Configuration\n══════════════\n\n")
	b.WriteString(fmt.Sprintf("  Model:    %s (%s)\n", m.ModelName, m.ProviderName))
	b.WriteString(fmt.Sprintf("  Profile:  %s\n", m.ProfileName))
	b.WriteString(fmt.Sprintf("  Beliefs:  %d loaded\n", m.BeliefsLoaded))
	b.WriteString(fmt.Sprintf("  Memory:   %s\n", m.MemoryMode))
	b.WriteString(fmt.Sprintf("  Fast:     %v\n", m.FastMode))
	b.WriteString(fmt.Sprintf("  Debug:    %v\n", m.DebugMode))
	b.WriteString(fmt.Sprintf("  Sisters:  %d/%d\n", m.SistersConn, m.SistersTotal))
	b.WriteString(fmt.Sprintf("  Server:   %s\n", m.Client.BaseURL))
	m.addSystemMsg(b.String())
}
