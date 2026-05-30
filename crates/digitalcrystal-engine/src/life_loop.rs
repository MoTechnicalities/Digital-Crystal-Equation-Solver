use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{solve_linear_equation, SolverConfig, SolverRequest, SolverResponse, StopReason};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifeGoalStatus {
    Active,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifeGoal {
    pub goal_id: String,
    pub description: String,
    pub priority: u8,
    pub confidence: f64,
    pub status: LifeGoalStatus,
    pub success_count: u32,
    pub failure_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifeObservationKind {
    LinearEquation,
    ContradictionSignal,
    KnowledgePing,
    Freeform,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifeObservation {
    pub kind: LifeObservationKind,
    #[serde(default)]
    pub variable: Option<String>,
    #[serde(default)]
    pub a: Option<f64>,
    #[serde(default)]
    pub b: Option<f64>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub timestamp_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifeActionKind {
    SolveLinear,
    AskClarification,
    Hold,
    SimulateLinear,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifeAction {
    pub kind: LifeActionKind,
    pub rationale: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifeOutcome {
    Success,
    Failure,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifeEpisodeRecord {
    pub cycle_index: u64,
    pub goal_id: Option<String>,
    pub action: LifeAction,
    pub outcome: LifeOutcome,
    pub summary: String,
    pub stop_reason: Option<StopReason>,
    pub solved_value: Option<f64>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeHealthReport {
    pub knowledge_score: f64,
    pub auditability_score: f64,
    pub success_ratio: f64,
    pub contradiction_load: f64,
    pub active_goal_count: usize,
    pub adaptation_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifeIdentityState {
    pub cycle_index: u64,
    pub active_goal_count: usize,
    pub completed_goal_count: usize,
    pub adaptation_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifeLoopState {
    pub cycle_index: u64,
    #[serde(default)]
    pub goals: Vec<LifeGoal>,
    #[serde(default)]
    pub episodes: Vec<LifeEpisodeRecord>,
    #[serde(default)]
    pub action_failures: BTreeMap<String, u32>,
    #[serde(default)]
    pub adaptation_events: u64,
    #[serde(default)]
    pub last_health: Option<KnowledgeHealthReport>,
}

impl Default for LifeLoopState {
    fn default() -> Self {
        Self {
            cycle_index: 0,
            goals: Vec::new(),
            episodes: Vec::new(),
            action_failures: BTreeMap::new(),
            adaptation_events: 0,
            last_health: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifeGoalUpsert {
    #[serde(default)]
    pub goal_id: Option<String>,
    pub description: String,
    #[serde(default = "default_goal_priority")]
    pub priority: u8,
    #[serde(default)]
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifeLoopTickRequest {
    #[serde(default)]
    pub goal_updates: Vec<LifeGoalUpsert>,
    pub observation: LifeObservation,
    #[serde(default)]
    pub simulate_only: bool,
    #[serde(default)]
    pub timestamp_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifeLoopTickResponse {
    pub cycle_index: u64,
    pub selected_goal_id: Option<String>,
    pub action: LifeAction,
    pub outcome: LifeOutcome,
    #[serde(default)]
    pub solver_response: Option<SolverResponse>,
    pub identity: LifeIdentityState,
    pub health: KnowledgeHealthReport,
}

#[derive(Debug, Error)]
pub enum LifeLoopStoreError {
    #[error("unable to read life loop state: {0}")]
    Io(#[from] std::io::Error),
    #[error("unable to deserialize life loop state: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn load_life_loop_state(path: &Path) -> Result<LifeLoopState, LifeLoopStoreError> {
    if !path.exists() {
        return Ok(LifeLoopState::default());
    }

    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save_life_loop_state(path: &Path, state: &LifeLoopState) -> Result<(), LifeLoopStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(state)?;
    fs::write(path, payload)?;
    Ok(())
}

pub fn life_loop_health(state: &LifeLoopState) -> KnowledgeHealthReport {
    let episodes_considered: Vec<&LifeEpisodeRecord> = state.episodes.iter().rev().take(20).collect();
    if episodes_considered.is_empty() {
        return KnowledgeHealthReport {
            knowledge_score: 0.5,
            auditability_score: 1.0,
            success_ratio: 0.5,
            contradiction_load: 0.0,
            active_goal_count: state
                .goals
                .iter()
                .filter(|goal| goal.status == LifeGoalStatus::Active)
                .count(),
            adaptation_events: state.adaptation_events,
        };
    }

    let success_count = episodes_considered
        .iter()
        .filter(|episode| episode.outcome == LifeOutcome::Success)
        .count();
    let failure_count = episodes_considered
        .iter()
        .filter(|episode| episode.outcome == LifeOutcome::Failure)
        .count();
    let contradiction_count = episodes_considered
        .iter()
        .filter(|episode| episode.stop_reason == Some(StopReason::ContradictionDetected))
        .count();
    let audited_count = episodes_considered
        .iter()
        .filter(|episode| {
            matches!(
                episode.action.kind,
                LifeActionKind::SolveLinear | LifeActionKind::SimulateLinear
            )
            .then_some(episode.stop_reason.is_some())
            .unwrap_or(true)
        })
        .count();

    let total = episodes_considered.len() as f64;
    let success_ratio = success_count as f64 / total;
    let contradiction_load = contradiction_count as f64 / total;
    let auditability_score = audited_count as f64 / total;
    let stability = (1.0 - contradiction_load).clamp(0.0, 1.0);
    let knowledge_score = (0.45 * success_ratio + 0.35 * auditability_score + 0.20 * stability)
        .clamp(0.0, 1.0);

    let _ = failure_count;

    KnowledgeHealthReport {
        knowledge_score,
        auditability_score,
        success_ratio,
        contradiction_load,
        active_goal_count: state
            .goals
            .iter()
            .filter(|goal| goal.status == LifeGoalStatus::Active)
            .count(),
        adaptation_events: state.adaptation_events,
    }
}

pub fn tick_life_loop(
    state: &mut LifeLoopState,
    request: &LifeLoopTickRequest,
    solver_config: &SolverConfig,
    history_limit: usize,
) -> LifeLoopTickResponse {
    let now = request
        .timestamp_ms
        .or(request.observation.timestamp_ms)
        .unwrap_or_else(now_ms);

    for update in &request.goal_updates {
        upsert_goal(state, update, now);
    }

    let selected_goal_index = state
        .goals
        .iter()
        .enumerate()
        .filter(|(_, goal)| goal.status == LifeGoalStatus::Active)
        .max_by(|(_, left), (_, right)| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.confidence.total_cmp(&right.confidence))
                .then_with(|| right.goal_id.cmp(&left.goal_id))
        })
        .map(|(index, _)| index);

    let selected_goal_id = selected_goal_index.map(|index| state.goals[index].goal_id.clone());
    let action = choose_action(state, selected_goal_id.as_deref(), request);

    let (outcome, summary, stop_reason, solved_value, solver_response) = match action.kind {
        LifeActionKind::SolveLinear | LifeActionKind::SimulateLinear => {
            if let (Some(a), Some(b)) = (request.observation.a, request.observation.b) {
                let variable = request
                    .observation
                    .variable
                    .clone()
                    .unwrap_or_else(|| "x".to_string());
                let response = solve_linear_equation(
                    &SolverRequest {
                        variable,
                        a,
                        b,
                    },
                    solver_config,
                );

                let stop_reason = Some(response.stop_reason.clone());
                let solved_value = response.solved_value;
                let summary = response.decision_label.clone();

                let outcome = match response.stop_reason {
                    StopReason::PathFound => {
                        if !request.simulate_only {
                            LifeOutcome::Success
                        } else {
                            LifeOutcome::Neutral
                        }
                    }
                    StopReason::ContradictionDetected => LifeOutcome::Failure,
                    StopReason::NoSupportingPath => LifeOutcome::Neutral,
                    StopReason::TimeoutOrBudget => LifeOutcome::Neutral,
                };

                (outcome, summary, stop_reason, solved_value, Some(response))
            } else {
                (
                    LifeOutcome::Failure,
                    "missing linear coefficients for requested solve action".to_string(),
                    Some(StopReason::NoSupportingPath),
                    None,
                    None,
                )
            }
        }
        LifeActionKind::AskClarification => {
            (
                LifeOutcome::Neutral,
                "requesting clarification to avoid repeated contradiction".to_string(),
                None,
                None,
                None,
            )
        }
        LifeActionKind::Hold => {
            (
                LifeOutcome::Neutral,
                "holding loop because no active goal is available".to_string(),
                None,
                None,
                None,
            )
        }
    };

    if let Some(goal_index) = selected_goal_index {
        let goal = &mut state.goals[goal_index];
        goal.updated_at_ms = now;
        match outcome {
            LifeOutcome::Success => {
                goal.success_count += 1;
                goal.confidence = (goal.confidence + 0.05).clamp(0.0, 1.0);
                if matches!(action.kind, LifeActionKind::SolveLinear) {
                    goal.status = LifeGoalStatus::Completed;
                }
            }
            LifeOutcome::Failure => {
                goal.failure_count += 1;
                goal.confidence = (goal.confidence - 0.08).clamp(0.0, 1.0);
            }
            LifeOutcome::Neutral => {
                goal.confidence = (goal.confidence + 0.01).clamp(0.0, 1.0);
            }
        }
    }

    if outcome == LifeOutcome::Failure {
        let count = state.action_failures.entry(action.signature.clone()).or_default();
        *count += 1;
    }

    state.episodes.push(LifeEpisodeRecord {
        cycle_index: state.cycle_index,
        goal_id: selected_goal_id.clone(),
        action: action.clone(),
        outcome: outcome.clone(),
        summary,
        stop_reason,
        solved_value,
        timestamp_ms: now,
    });

    let limit = history_limit.max(16);
    if state.episodes.len() > limit {
        let trim = state.episodes.len() - limit;
        state.episodes.drain(0..trim);
    }

    state.cycle_index += 1;
    let health = life_loop_health(state);
    state.last_health = Some(health.clone());

    LifeLoopTickResponse {
        cycle_index: state.cycle_index,
        selected_goal_id,
        action,
        outcome,
        solver_response,
        identity: life_loop_identity(state),
        health,
    }
}

pub fn life_loop_identity(state: &LifeLoopState) -> LifeIdentityState {
    LifeIdentityState {
        cycle_index: state.cycle_index,
        active_goal_count: state
            .goals
            .iter()
            .filter(|goal| goal.status == LifeGoalStatus::Active)
            .count(),
        completed_goal_count: state
            .goals
            .iter()
            .filter(|goal| goal.status == LifeGoalStatus::Completed)
            .count(),
        adaptation_events: state.adaptation_events,
    }
}

fn choose_action(
    state: &mut LifeLoopState,
    goal_id: Option<&str>,
    request: &LifeLoopTickRequest,
) -> LifeAction {
    let signature = action_signature(goal_id, &request.observation);
    let failure_streak = state.action_failures.get(&signature).copied().unwrap_or(0);

    if goal_id.is_none() {
        return LifeAction {
            kind: LifeActionKind::Hold,
            rationale: "no active goal available for this cycle".to_string(),
            signature,
        };
    }

    if failure_streak >= 2 {
        state.adaptation_events += 1;
        return LifeAction {
            kind: LifeActionKind::AskClarification,
            rationale: "repeat-failure guard triggered; ask for clarification before retry"
                .to_string(),
            signature,
        };
    }

    if request.observation.a.is_some() && request.observation.b.is_some() {
        return LifeAction {
            kind: if request.simulate_only {
                LifeActionKind::SimulateLinear
            } else {
                LifeActionKind::SolveLinear
            },
            rationale: "observation carries linear coefficients; execute deterministic solver"
                .to_string(),
            signature,
        };
    }

    LifeAction {
        kind: LifeActionKind::AskClarification,
        rationale: "insufficient structured coefficients; gather clarifying context".to_string(),
        signature,
    }
}

fn action_signature(goal_id: Option<&str>, observation: &LifeObservation) -> String {
    format!(
        "{}|{:?}|{}|{}|{}",
        goal_id.unwrap_or("none"),
        observation.kind,
        observation.variable.as_deref().unwrap_or("x"),
        observation.a.unwrap_or(0.0),
        observation.b.unwrap_or(0.0)
    )
}

fn upsert_goal(state: &mut LifeLoopState, update: &LifeGoalUpsert, now: u64) {
    if let Some(goal_id) = &update.goal_id {
        if let Some(existing) = state.goals.iter_mut().find(|goal| &goal.goal_id == goal_id) {
            existing.description = update.description.clone();
            existing.priority = update.priority;
            if let Some(confidence) = update.confidence {
                existing.confidence = confidence.clamp(0.0, 1.0);
            }
            existing.updated_at_ms = now;
            if existing.status == LifeGoalStatus::Paused {
                existing.status = LifeGoalStatus::Active;
            }
            return;
        }
    }

    let new_id = update
        .goal_id
        .clone()
        .unwrap_or_else(|| format!("goal-{}", state.cycle_index + state.goals.len() as u64 + 1));
    state.goals.push(LifeGoal {
        goal_id: new_id,
        description: update.description.clone(),
        priority: update.priority,
        confidence: update.confidence.unwrap_or(0.5).clamp(0.0, 1.0),
        status: LifeGoalStatus::Active,
        success_count: 0,
        failure_count: 0,
        created_at_ms: now,
        updated_at_ms: now,
    });
}

fn now_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as u64,
        Err(_) => 0,
    }
}

fn default_goal_priority() -> u8 {
    50
}
