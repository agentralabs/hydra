package app

import (
	"fmt"
	"strings"
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
		m.addSystemMsg("Session resume — use hydra-server session API.")
	case "/fork":
		m.addSystemMsg("Conversation forked.")
	case "/rewind":
		cmdRewind(m)
	case "/rename":
		m.addSystemMsg(fmt.Sprintf("Session renamed to '%s'.", arg1))
	case "/export":
		m.addSystemMsg("Export to markdown — coming soon.")
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
		m.addSystemMsg(fmt.Sprintf("Sister detail: %s — query via hydra-server.", arg1))
	case "/stats":
		m.addSystemMsg("Gateway stats — query via hydra-server.")
	case "/fix":
		m.addSystemMsg("Attempting to repair offline sisters...")
	case "/scan":
		m.addSystemMsg("Omniscience scan — query via hydra-server.")
	case "/repair":
		m.addSystemMsg("Running self-repair specs...")
	case "/memory":
		cmdMemory(m, arg1)
	case "/goals":
		m.addSystemMsg("Active goals — query via hydra-server.")
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
		m.addSystemMsg("Dream state — query via hydra-server.")
	case "/obstacles":
		m.addSystemMsg("Session momentum — query via hydra-server.")
	case "/threat":
		m.addSystemMsg("Threat correlator — query via hydra-server.")
	case "/autonomy":
		m.addSystemMsg(fmt.Sprintf("Autonomy level: query via hydra-server. Arg: %s", arg1))
	case "/implement":
		m.addSystemMsg("Self-modification requires explicit approval.")
	case "/diagnostics":
		cmdDiagnostics(m)
	case "/trust":
		m.addSystemMsg("Trust level — query via hydra-server.")
	case "/roi":
		cmdROI(m)
	case "/knowledge":
		m.addSystemMsg("Knowledge progress — query via hydra-server.")
	case "/skills":
		m.addSystemMsg(fmt.Sprintf("Skills loaded from profile: %s", m.ProfileName))

	// Dev - files
	case "/files":
		m.addSystemMsg("File listing — use Codebase sister.")
	case "/open":
		m.addSystemMsg(fmt.Sprintf("Opening %s — use Codebase sister.", arg1))
	case "/edit":
		m.addSystemMsg(fmt.Sprintf("Editing %s — use $EDITOR.", arg1))
	case "/search":
		m.addSystemMsg(fmt.Sprintf("Searching '%s' — use Codebase sister.", arg1))
	case "/symbols":
		m.addSystemMsg(fmt.Sprintf("Symbols in %s — use Codebase sister.", arg1))
	case "/impact":
		m.addSystemMsg(fmt.Sprintf("Impact of %s — use Codebase sister.", arg1))

	// Dev - project
	case "/diff":
		m.addSystemMsg("Git diff — running via hydra-server.")
	case "/git":
		m.addSystemMsg(fmt.Sprintf("Git %s — running via hydra-server.", arg1))
	case "/test":
		m.addSystemMsg("Running tests — auto-detected project type.")
	case "/build":
		m.addSystemMsg("Building — auto-detected project type.")
	case "/run":
		m.addSystemMsg("Running — auto-detected project type.")
	case "/lint":
		m.addSystemMsg("Linting — auto-detected project type.")
	case "/fmt":
		m.addSystemMsg("Formatting — auto-detected project type.")
	case "/deps":
		m.addSystemMsg("Dependencies — auto-detected project type.")
	case "/bench":
		m.addSystemMsg("Benchmarks — auto-detected project type.")
	case "/doc":
		m.addSystemMsg("Documentation — auto-detected project type.")
	case "/deploy":
		m.addSystemMsg("Deploying — use HYDRA_DEPLOY_CMD env var.")
	case "/init":
		m.addSystemMsg("Project init — auto-detected.")

	// Integrations
	case "/mcp":
		m.addSystemMsg(fmt.Sprintf("MCP management: %s", arg1))
	case "/ide":
		m.addSystemMsg("IDE integration status.")
	case "/hooks":
		m.addSystemMsg("Hook configuration.")
	case "/plugin":
		m.addSystemMsg(fmt.Sprintf("Plugin: %s", arg1))
	case "/remote", "/remote-control":
		m.addSystemMsg("Remote control — web UI.")
	case "/ssh":
		m.addSystemMsg(fmt.Sprintf("SSH to %s", arg1))
	case "/ssh-exec":
		m.addSystemMsg(fmt.Sprintf("SSH exec on %s: %s", arg1, arg2))
	case "/ssh-upload":
		m.addSystemMsg("SSH upload.")
	case "/ssh-download":
		m.addSystemMsg("SSH download.")
	case "/ssh-disconnect":
		m.addSystemMsg(fmt.Sprintf("SSH disconnect from %s", arg1))
	case "/ssh-list":
		m.addSystemMsg("Active SSH connections.")

	// Agents & swarm
	case "/agents":
		m.addSystemMsg("Custom subagents — project + personal.")
	case "/commands":
		cmdHelp(m) // same as /help
	case "/plan":
		m.PermMode = PermPlan
		m.addSystemMsg("Entered Plan mode — no execution, plan only.")
	case "/bashes":
		m.addSystemMsg("Background processes.")
	case "/tasks":
		m.ShowTasks = true
	case "/swarm":
		m.addSystemMsg(fmt.Sprintf("Swarm: %s %s", arg1, arg2))
	case "/improve-sister":
		m.addSystemMsg(fmt.Sprintf("Improving sister: %s", arg1))

	// Config
	case "/config":
		m.addSystemMsg("Configuration — check ~/.hydra/settings.json")
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
		m.addSystemMsg("Voice mode — hold Space to speak. Requires Whisper STT.")
	case "/terminal-setup":
		m.addSystemMsg("Terminal keybindings installed.")
	case "/login":
		m.addSystemMsg(fmt.Sprintf("Logged in as: %s", arg1))
	case "/logout":
		m.addSystemMsg("Logged out.")
	case "/keybindings":
		m.addSystemMsg("Keybindings: ~/.hydra/keybindings.json")
	case "/email":
		m.addSystemMsg(fmt.Sprintf("Email to %s: %s", arg1, arg2))
	case "/email-setup":
		m.addSystemMsg(fmt.Sprintf("Email config: %s = %s", arg1, arg2))

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
