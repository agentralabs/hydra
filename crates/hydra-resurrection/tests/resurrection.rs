//! Integration tests for hydra-resurrection.

use hydra_kernel::state::HydraState;
use hydra_resurrection::checkpoint::{Checkpoint, KernelStateSnapshot, TaskDelta};
use hydra_resurrection::index::CheckpointIndex;
use hydra_resurrection::reader::CheckpointReader;
use hydra_resurrection::restart::warm_restart;
use hydra_resurrection::writer::CheckpointWriter;
use hydra_resurrection::CheckpointKind;

#[test]
fn full_write_read_roundtrip() {
    let state = HydraState::initial();
    let snap = KernelStateSnapshot::from_state(&state);
    let cp = Checkpoint::full(1, snap).expect("full");
    cp.verify_integrity().expect("integrity");
    let result = CheckpointReader::reconstruct(&[cp]).expect("reconstruct");
    assert_eq!(result.checkpoints_applied, 1);
    assert!((result.state.lyapunov_value - 1.0).abs() < f64::EPSILON);
}

#[test]
fn writer_sequences_full_then_delta() {
    let mut writer = CheckpointWriter::new();
    let mut index = CheckpointIndex::new();

    let state1 = HydraState::initial();
    let snap1 = KernelStateSnapshot::from_state(&state1);
    let cp1 = writer.write(&snap1, &mut index).expect("write1");
    assert_eq!(cp1.kind, CheckpointKind::Full);

    let mut state2 = state1;
    state2.step_count = 1;
    state2.lyapunov_value = 0.9;
    let snap2 = KernelStateSnapshot::from_state(&state2);
    let cp2 = writer.write(&snap2, &mut index).expect("write2");
    assert_eq!(cp2.kind, CheckpointKind::Delta);

    // Reconstruct from both
    let result = CheckpointReader::reconstruct(&[cp1, cp2]).expect("reconstruct");
    assert_eq!(result.state.step_count, 1);
    assert!((result.state.lyapunov_value - 0.9).abs() < f64::EPSILON);
}

#[test]
fn corrupted_delta_skipped_gracefully() {
    let state = HydraState::initial();
    let snap = KernelStateSnapshot::from_state(&state);
    let full = Checkpoint::full(1, snap.clone()).expect("full");

    let mut state2 = state;
    state2.step_count = 10;
    let snap2 = KernelStateSnapshot::from_state(&state2);
    let delta = TaskDelta::compute(&snap, &snap2);
    let mut bad = Checkpoint::delta(2, delta).expect("delta");
    bad.sha256 = "tampered".to_string();

    let result = CheckpointReader::reconstruct(&[full, bad]).expect("ok");
    assert_eq!(result.corrupted_skipped, 1);
    assert_eq!(result.state.step_count, 0); // delta was skipped
}

#[test]
fn warm_restart_integration() {
    let mut writer = CheckpointWriter::new();
    let mut index = CheckpointIndex::new();
    let mut checkpoints = Vec::new();

    let mut state = HydraState::initial();
    for i in 0..5 {
        state.step_count = i;
        state.lyapunov_value = 1.0 - (i as f64 * 0.1);
        let snap = KernelStateSnapshot::from_state(&state);
        let cp = writer.write(&snap, &mut index).expect("write");
        checkpoints.push(cp);
    }

    let result = warm_restart(&checkpoints).expect("restart");
    assert!(result.met_target);
    assert_eq!(result.state.step_count, 4);
}

#[test]
fn index_monotonicity() {
    let mut index = CheckpointIndex::new();
    let mut prev = 0;
    for i in 0..20 {
        let id = index.register(i % 10 == 0, format!("h{i}"));
        assert!(id > prev, "IDs must be monotonically increasing");
        prev = id;
    }
}
