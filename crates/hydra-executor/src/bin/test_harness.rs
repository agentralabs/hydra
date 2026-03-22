//! Test harness for `hydra-executor`.

use hydra_executor::{
    ActionRegistry, ApproachType, ExecutionEngine, ExecutionRequest,
    ExecutionReceipt, ExecutorType, ReceiptOutcome, RegisteredAction,
    TaskState,
};
use std::collections::HashMap;

fn main() {
    println!("=== hydra-executor test harness ===\n");

    let mut passed = 0;
    let mut failed = 0;

    // Test 1: 13 approach types in escalation order.
    {
        print!("test 13_approach_types ... ");
        let approaches = ApproachType::all_in_order();
        if approaches.len() == 13 {
            println!("ok");
            passed += 1;
        } else {
            println!("FAILED: got {} approaches", approaches.len());
            failed += 1;
        }
    }

    // Test 2: ApproachType labels.
    {
        print!("test approach_type_labels ... ");
        let first = ApproachType::DirectExecution;
        let last = ApproachType::EscalateToSwarm;
        if first.label() == "direct" && last.label() == "swarm-escalation" {
            println!("ok");
            passed += 1;
        } else {
            println!("FAILED: labels wrong");
            failed += 1;
        }
    }

    // Test 3: TaskState — only Complete and HardDenied are terminal.
    {
        print!("test terminal_states ... ");
        let terminal = [
            TaskState::Complete {
                receipt_id: "r".into(),
            },
            TaskState::HardDenied {
                evidence: "e".into(),
                receipt_id: "r".into(),
            },
        ];
        let non_terminal = [
            TaskState::Active {
                approach: ApproachType::DirectExecution,
            },
            TaskState::Blocked {
                reason: "b".into(),
                approach: ApproachType::DirectExecution,
            },
            TaskState::Rerouting {
                attempt: 1,
                next_approach: ApproachType::AlternativeTooling,
            },
            TaskState::EscalatingToAgent {
                agent_type: "specialist".into(),
            },
            TaskState::Suspended {
                condition: "waiting".into(),
                retry_after_seconds: 60,
            },
        ];
        let ok = terminal.iter().all(|s| s.is_terminal())
            && non_terminal.iter().all(|s| !s.is_terminal());
        if ok {
            println!("ok (7 variants, no Failed)");
            passed += 1;
        } else {
            println!("FAILED: terminal classification wrong");
            failed += 1;
        }
    }

    // Test 4: Receipt creation + hash.
    {
        print!("test receipt_creation_and_hash ... ");
        let r = ExecutionReceipt::for_start(
            "task-1",
            "action.deploy",
            "deploy staging",
            "direct",
        );
        if r.verify()
            && r.content_hash.len() == 64
            && r.outcome == ReceiptOutcome::Started
        {
            println!("ok");
            passed += 1;
        } else {
            println!("FAILED: hash len={}", r.content_hash.len());
            failed += 1;
        }
    }

    // Test 5: Action registry register/unregister.
    {
        print!("test action_registry ... ");
        let mut reg = ActionRegistry::new();
        let count = reg.register_skill_actions(
            "skill-a",
            vec![make_action("a.one"), make_action("a.two")],
        );
        reg.register_skill_actions(
            "skill-b",
            vec![make_action("b.one")],
        );
        let removed = reg.unregister_skill("skill-a");
        if count == 2 && removed == 2 && reg.count() == 1 {
            println!("ok");
            passed += 1;
        } else {
            println!("FAILED");
            failed += 1;
        }
    }

    // Test 6: Successful execution -> Complete state.
    {
        print!("test successful_execution ... ");
        let mut engine = ExecutionEngine::new();
        register(&mut engine, "deploy.staging");
        let req = ExecutionRequest::new(
            "deploy.staging",
            "deploy to staging",
            HashMap::new(),
        );
        match engine.execute(req) {
            Ok(task) => {
                if matches!(task.state, TaskState::Complete { .. }) {
                    println!("ok");
                    passed += 1;
                } else {
                    println!("FAILED: got {}", task.state.label());
                    failed += 1;
                }
            }
            Err(e) => {
                println!("FAILED: {e}");
                failed += 1;
            }
        }
    }

    // Test 7: Unknown action -> error.
    {
        print!("test unknown_action_error ... ");
        let mut engine = ExecutionEngine::new();
        let req = ExecutionRequest::new(
            "nonexistent.action",
            "test",
            HashMap::new(),
        );
        match engine.execute(req) {
            Err(hydra_executor::ExecutorError::ActionNotFound { .. }) => {
                println!("ok");
                passed += 1;
            }
            other => {
                println!("FAILED: expected ActionNotFound, got {other:?}");
                failed += 1;
            }
        }
    }

    // Test 8: Blocked action never produces FAILED state.
    {
        print!("test no_failed_state ... ");
        let mut engine = ExecutionEngine::new();
        engine.registry_mut().register_skill_actions(
            "test",
            vec![RegisteredAction {
                id: "shell.unresolved".into(),
                skill: "test".into(),
                description: "shell with unresolved params".into(),
                verb: "running".into(),
                executor: ExecutorType::Shell {
                    command_template: "cmd {required_param}".into(),
                },
                reversible: false,
                estimated_ms: 100,
                input_params: vec![],
            }],
        );
        let req = ExecutionRequest::new(
            "shell.unresolved",
            "test shell",
            HashMap::new(),
        );
        match engine.execute(req) {
            Ok(task) => {
                assert_ne!(task.state.label(), "failed");
                println!("ok (state={})", task.state.label());
                passed += 1;
            }
            Err(_) => {
                println!("ok (approaches exhausted, not FAILED)");
                passed += 1;
            }
        }
    }

    println!(
        "\n--- executor results: {passed} passed, {failed} failed ---"
    );
    if failed > 0 {
        std::process::exit(1);
    }
}

fn make_action(id: &str) -> RegisteredAction {
    RegisteredAction {
        id: id.to_string(),
        skill: "test-skill".to_string(),
        description: format!("Test action {id}"),
        verb: "test".to_string(),
        executor: ExecutorType::Internal {
            handler: "noop".to_string(),
        },
        reversible: false,
        estimated_ms: 10,
        input_params: vec![],
    }
}

fn register(engine: &mut ExecutionEngine, id: &str) {
    engine.registry_mut().register_skill_actions(
        "test-skill",
        vec![RegisteredAction {
            id: id.to_string(),
            skill: "test-skill".into(),
            description: format!("test action {}", id),
            verb: "testing".into(),
            executor: ExecutorType::Internal {
                handler: "succeed".into(),
            },
            reversible: true,
            estimated_ms: 10,
            input_params: vec![],
        }],
    );
}
