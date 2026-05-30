use std::{fs, path::PathBuf};

use digitalcrystal_engine::{
    life_loop_health, tick_life_loop, LifeActionKind, LifeGoalUpsert, LifeLoopState, LifeLoopTickRequest,
    LifeObservation, LifeObservationKind, SolverConfig,
};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/life_loop_v0/fixtures")
        .join(name)
}

fn load_state_fixture(name: &str) -> LifeLoopState {
    let contents = fs::read_to_string(fixture_path(name)).expect("fixture should read");
    serde_json::from_str(&contents).expect("fixture should parse")
}

fn load_tick_request_fixture(name: &str) -> LifeLoopTickRequest {
    let contents = fs::read_to_string(fixture_path(name)).expect("fixture should read");
    serde_json::from_str(&contents).expect("fixture should parse")
}

#[test]
fn life_loop_adapts_after_repeat_contradictions() {
    let mut state = LifeLoopState::default();
    let solver = SolverConfig::default();

    let base_request = LifeLoopTickRequest {
        goal_updates: vec![LifeGoalUpsert {
            goal_id: Some("goal-linear".to_string()),
            description: "Solve linear equation correctly".to_string(),
            priority: 90,
            confidence: Some(0.8),
        }],
        observation: LifeObservation {
            kind: LifeObservationKind::LinearEquation,
            variable: Some("x".to_string()),
            a: Some(0.0),
            b: Some(4.0),
            note: Some("contradictory sample".to_string()),
            timestamp_ms: Some(1),
        },
        simulate_only: false,
        timestamp_ms: Some(1),
    };

    let first = tick_life_loop(&mut state, &base_request, &solver, 64);
    assert_eq!(first.action.kind, LifeActionKind::SolveLinear);

    let second = tick_life_loop(
        &mut state,
        &LifeLoopTickRequest {
            goal_updates: Vec::new(),
            observation: LifeObservation {
                timestamp_ms: Some(2),
                ..base_request.observation.clone()
            },
            simulate_only: false,
            timestamp_ms: Some(2),
        },
        &solver,
        64,
    );
    assert_eq!(second.action.kind, LifeActionKind::SolveLinear);

    let third = tick_life_loop(
        &mut state,
        &LifeLoopTickRequest {
            goal_updates: Vec::new(),
            observation: LifeObservation {
                timestamp_ms: Some(3),
                ..base_request.observation.clone()
            },
            simulate_only: false,
            timestamp_ms: Some(3),
        },
        &solver,
        64,
    );

    assert_eq!(third.action.kind, LifeActionKind::AskClarification);
    assert!(third.identity.adaptation_events >= 1);
}

#[test]
fn life_loop_health_improves_after_successful_tick() {
    let mut state = LifeLoopState::default();
    let solver = SolverConfig::default();

    let before = life_loop_health(&state);

    tick_life_loop(
        &mut state,
        &LifeLoopTickRequest {
            goal_updates: vec![LifeGoalUpsert {
                goal_id: Some("goal-success".to_string()),
                description: "Solve one equation".to_string(),
                priority: 80,
                confidence: Some(0.7),
            }],
            observation: LifeObservation {
                kind: LifeObservationKind::LinearEquation,
                variable: Some("x".to_string()),
                a: Some(2.0),
                b: Some(-4.0),
                note: None,
                timestamp_ms: Some(10),
            },
            simulate_only: false,
            timestamp_ms: Some(10),
        },
        &solver,
        64,
    );

    let after = life_loop_health(&state);
    assert!(after.knowledge_score >= before.knowledge_score);
    assert!(after.success_ratio >= before.success_ratio);
}

#[test]
fn life_loop_tick_is_deterministic_from_replay_snapshot_fixture() {
    let base_state = load_state_fixture("LIFELOOP-C-001-state-snapshot.json");
    let request = load_tick_request_fixture("LIFELOOP-C-001-tick-request.json");
    let solver = SolverConfig::default();

    let mut left = base_state.clone();
    let mut right = base_state;

    let left_response = tick_life_loop(&mut left, &request, &solver, 64);
    let right_response = tick_life_loop(&mut right, &request, &solver, 64);

    assert_eq!(left_response, right_response);
    assert_eq!(left, right);

    let left_bytes = serde_json::to_string(&left_response).expect("response should serialize");
    let right_bytes = serde_json::to_string(&right_response).expect("response should serialize");
    assert_eq!(left_bytes, right_bytes);
}
