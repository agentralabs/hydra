//! Tests for agent swarm.

use super::*;
use super::agent::*;
use super::distributor::*;
use super::aggregator::*;
use chrono::Utc;

// ── Agent Config Serialization ──

#[test]
fn test_agent_config_serialization() {
    let config = AgentConfig {
        name: "test-agent".into(),
        role: AgentRole::Worker,
        host: AgentHost::Local,
        skills: vec!["rust".into(), "testing".into()],
        permissions: AgentPermissions::default(),
        goal: Some("run tests".into()),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: AgentConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "test-agent");
    assert_eq!(parsed.skills.len(), 2);
}

// ── Agent Instance ──

#[test]
fn test_agent_instance_new() {
    let config = AgentConfig {
        name: "worker-1".into(),
        role: AgentRole::Worker,
        host: AgentHost::Local,
        skills: vec!["rust".into()],
        permissions: AgentPermissions::default(),
        goal: None,
    };
    let agent = AgentInstance::new(&config);
    assert_eq!(agent.name, "worker-1");
    assert_eq!(agent.status, AgentStatus::Starting);
    assert!(agent.task.is_none());
    assert!(agent.results.is_empty());
}

#[test]
fn test_agent_can_handle_generic_task() {
    let config = AgentConfig {
        name: "worker".into(),
        role: AgentRole::Worker,
        host: AgentHost::Local,
        skills: vec!["rust".into()],
        permissions: AgentPermissions::default(),
        goal: None,
    };
    let mut agent = AgentInstance::new(&config);
    agent.status = AgentStatus::Idle;

    let task = AgentTask {
        id: "t1".into(),
        description: "do stuff".into(),
        required_skills: vec![],
        priority: 5,
        timeout_secs: 60,
    };
    assert!(agent.can_handle(&task));
}

#[test]
fn test_agent_can_handle_skill_match() {
    let config = AgentConfig {
        name: "rust-worker".into(),
        role: AgentRole::Specialist("rust".into()),
        host: AgentHost::Local,
        skills: vec!["rust".into(), "testing".into()],
        permissions: AgentPermissions::default(),
        goal: None,
    };
    let mut agent = AgentInstance::new(&config);
    agent.status = AgentStatus::Idle;

    let matching = AgentTask {
        id: "t1".into(),
        description: "test".into(),
        required_skills: vec!["rust".into()],
        priority: 5,
        timeout_secs: 60,
    };
    assert!(agent.can_handle(&matching));

    let not_matching = AgentTask {
        id: "t2".into(),
        description: "deploy".into(),
        required_skills: vec!["kubernetes".into()],
        priority: 5,
        timeout_secs: 60,
    };
    assert!(!agent.can_handle(&not_matching));
}

#[test]
fn test_agent_assign_and_complete() {
    let config = AgentConfig {
        name: "w".into(),
        role: AgentRole::Worker,
        host: AgentHost::Local,
        skills: vec![],
        permissions: AgentPermissions::default(),
        goal: None,
    };
    let mut agent = AgentInstance::new(&config);
    agent.status = AgentStatus::Idle;

    let task = AgentTask {
        id: "t1".into(),
        description: "run tests".into(),
        required_skills: vec![],
        priority: 5,
        timeout_secs: 60,
    };
    agent.assign_task(task);
    assert!(matches!(agent.status, AgentStatus::Working(_)));

    let result = TaskResult {
        agent_id: agent.id.clone(),
        task_id: "t1".into(),
        success: true,
        output: "all tests pass".into(),
        error: None,
        duration_ms: 500,
        quality_score: 0.95,
        completed_at: Utc::now(),
    };
    agent.complete_task(result);
    assert_eq!(agent.status, AgentStatus::Completed);
    assert_eq!(agent.results.len(), 1);
}

#[test]
fn test_agent_permissions_default() {
    let perms = AgentPermissions::default();
    assert!(perms.can_write_files);
    assert!(perms.can_execute_commands);
    assert!(!perms.can_access_network);
    assert!(!perms.can_spawn_subagents);
    assert_eq!(perms.max_cost_cents, 100);
}

// ── Task Decomposition ──

#[test]
fn test_task_decomposition() {
    let dist = TaskDistributor::new();
    let tasks = dist.decompose("run all tests", 5);
    assert_eq!(tasks.len(), 5);
    for task in &tasks {
        assert!(task.description.contains("run all tests"));
    }
}

#[test]
fn test_task_decomposition_review() {
    let dist = TaskDistributor::new();
    let tasks = dist.decompose("review this codebase", 3);
    assert_eq!(tasks.len(), 3);
    // Review tasks should have different focus areas
    let descs: Vec<&str> = tasks.iter().map(|t| t.description.as_str()).collect();
    assert!(descs[0] != descs[1]);
}

#[test]
fn test_task_decomposition_zero_agents() {
    let dist = TaskDistributor::new();
    let tasks = dist.decompose("anything", 0);
    assert!(tasks.is_empty());
}

// ── Task Assignment ──

#[test]
fn test_task_assignment() {
    let dist = TaskDistributor::new();

    let mut agents = Vec::new();
    for i in 0..3 {
        let config = AgentConfig {
            name: format!("agent-{}", i),
            role: AgentRole::Worker,
            host: AgentHost::Local,
            skills: vec!["general".into()],
            permissions: AgentPermissions::default(),
            goal: None,
        };
        let mut a = AgentInstance::new(&config);
        a.status = AgentStatus::Idle;
        agents.push(a);
    }

    let tasks = dist.decompose("test everything", 3);
    let assignments = dist.assign(&tasks, &agents);
    assert_eq!(assignments.len(), 3);
}

// ── Task Duplication ──

#[test]
fn test_task_duplication() {
    let dist = TaskDistributor::new();
    let tasks = dist.duplicate_task("test the API", 5);
    assert_eq!(tasks.len(), 5);
    for (i, task) in tasks.iter().enumerate() {
        assert!(task.description.contains(&format!("agent {}/{}", i + 1, 5)));
    }
}

// ── Result Aggregation ──

#[test]
fn test_result_aggregation() {
    let agg = ResultAggregator::new();
    let results = vec![
        TaskResult {
            agent_id: "a1".into(), task_id: "t1".into(),
            success: true, output: "ok".into(), error: None,
            duration_ms: 100, quality_score: 0.9, completed_at: Utc::now(),
        },
        TaskResult {
            agent_id: "a2".into(), task_id: "t2".into(),
            success: true, output: "ok".into(), error: None,
            duration_ms: 200, quality_score: 0.8, completed_at: Utc::now(),
        },
        TaskResult {
            agent_id: "a3".into(), task_id: "t3".into(),
            success: false, output: "".into(), error: Some("timeout".into()),
            duration_ms: 300, quality_score: 0.0, completed_at: Utc::now(),
        },
    ];
    let report = agg.aggregate(&results);
    assert_eq!(report.total_agents, 3);
    assert_eq!(report.succeeded, 2);
    assert_eq!(report.failed, 1);
    assert_eq!(report.duration_ms, 300);
}

// ── Pick Best Result ──

#[test]
fn test_pick_best_result() {
    let agg = ResultAggregator::new();
    let results = vec![
        TaskResult {
            agent_id: "a1".into(), task_id: "t1".into(),
            success: true, output: "ok".into(), error: None,
            duration_ms: 100, quality_score: 0.7, completed_at: Utc::now(),
        },
        TaskResult {
            agent_id: "a2".into(), task_id: "t2".into(),
            success: true, output: "better".into(), error: None,
            duration_ms: 200, quality_score: 0.95, completed_at: Utc::now(),
        },
        TaskResult {
            agent_id: "a3".into(), task_id: "t3".into(),
            success: false, output: "".into(), error: Some("fail".into()),
            duration_ms: 50, quality_score: 1.0, completed_at: Utc::now(),
        },
    ];
    let best = agg.pick_best(&results).unwrap();
    assert_eq!(best.agent_id, "a2");
    assert_eq!(best.quality_score, 0.95);
}

// ── Health Check ──

#[tokio::test]
async fn test_health_check_empty() {
    let monitor = SwarmMonitor::new();
    let agents = std::collections::HashMap::new();
    let statuses = monitor.health_check(&agents).await;
    assert!(statuses.is_empty());
}

// ── Terminate All ──

#[tokio::test]
async fn test_terminate_all() {
    let monitor = SwarmMonitor::new();
    let spawner = SwarmSpawner::new();
    let mut agents = std::collections::HashMap::new();

    let config = AgentConfig {
        name: "temp".into(),
        role: AgentRole::Worker,
        host: AgentHost::Local,
        skills: vec![],
        permissions: AgentPermissions::default(),
        goal: None,
    };
    let mut a = AgentInstance::new(&config);
    a.status = AgentStatus::Idle;
    let id = a.id.clone();
    agents.insert(id, a);

    monitor.terminate_all(&mut agents, &spawner).await;
    for agent in agents.values() {
        assert_eq!(agent.status, AgentStatus::Terminated);
    }
}

// ── SwarmManager ──

#[test]
fn test_swarm_manager_default() {
    let mgr = SwarmManager::new(100);
    assert_eq!(mgr.agent_count(), 0);
}

#[test]
fn test_swarm_status_empty() {
    let mgr = SwarmManager::new(100);
    let status = mgr.status_summary();
    assert!(status.contains("No agents"));
}

// ── Scale ──

#[tokio::test]
async fn test_scale_up() {
    let mgr = SwarmManager::new(100);
    let result = mgr.scale_to(3).await;
    assert!(result.contains("Scaled up"));
    assert_eq!(mgr.agent_count(), 3);
}

#[tokio::test]
async fn test_scale_down() {
    let mgr = SwarmManager::new(100);
    mgr.scale_to(5).await;
    assert_eq!(mgr.agent_count(), 5);
    let result = mgr.scale_to(2).await;
    assert!(result.contains("Scaled down"));
    // Some agents terminated
    assert!(mgr.agent_count() <= 5);
}

// ── Agent Summary ──

#[test]
fn test_agent_summary() {
    let config = AgentConfig {
        name: "test-agent".into(),
        role: AgentRole::Specialist("rust".into()),
        host: AgentHost::Remote { host: "server1".into(), user: "deploy".into() },
        skills: vec![],
        permissions: AgentPermissions::default(),
        goal: None,
    };
    let agent = AgentInstance::new(&config);
    let summary = agent.summary();
    assert!(summary.contains("test-agent"));
    assert!(summary.contains("server1"));
    assert!(summary.contains("Specialist"));
}

// ── Slash Command Parsing ──

#[test]
fn test_swarm_slash_commands() {
    // Verify command string patterns parse correctly
    let cmds = [
        ("spawn 5", "spawn", "5"),
        ("status", "status", ""),
        ("assign run all tests", "assign", "run all tests"),
        ("results", "results", ""),
        ("kill abc123", "kill", "abc123"),
        ("kill-all", "kill-all", ""),
        ("scale 10", "scale", "10"),
    ];
    for (input, expected_cmd, expected_args) in &cmds {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0];
        let args = parts.get(1).copied().unwrap_or("");
        assert_eq!(cmd, *expected_cmd);
        assert_eq!(args, *expected_args);
    }
}
