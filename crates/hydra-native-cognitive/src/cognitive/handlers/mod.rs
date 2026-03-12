//! Handler modules — extracted from loop_runner.rs for compilation performance.

pub mod actions;
pub mod actions_detect;
pub mod dispatch;
pub mod dispatch_actions;
pub mod dispatch_intents;
pub mod execution;
pub mod execution_deepen;
pub mod llm_helpers;
pub mod memory;
pub mod phase_act;
pub mod phase_act_exec;
pub mod phase_decide;
pub mod phase_learn;
pub mod phase_learn_beliefs;
pub mod phase_perceive;
pub mod phase_think;
pub mod phase_think_call;
pub mod phase_think_prompt;
pub mod platform;
pub mod platform_system;
pub mod sister_ops;
pub mod sisters;
pub mod obstacle_handler;
