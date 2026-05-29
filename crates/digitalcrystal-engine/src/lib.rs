use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const DEFAULT_WRAP_MODE: &str = "principal_pi";
pub const DEFAULT_ENGINE_MODE: &str = "signed_i8_plus_intent_v2";
pub const RWIF_SCHEMA_VERSION: &str = "RWIF_V2";
pub const RWIF_EDGE_SCHEMA_VERSION: &str = "RWIF_EDGE_V2";
pub const RWIF_EVENT_SCHEMA_VERSION: &str = "RWIF_EVENT_V2";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopReason {
    PathFound,
    NoSupportingPath,
    ContradictionDetected,
    TimeoutOrBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedState {
    pub amplitude_signed: i16,
    pub intent_signed: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteStep {
    pub edge_id: String,
    pub direction_forward: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteAudit {
    pub stop_reason: StopReason,
    pub selected_path: Vec<RouteStep>,
    pub contradiction_residual_quantized: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SolverRequest {
    pub variable: String,
    pub a: f64,
    pub b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResidualMetrics {
    pub phase_residual_quantized: i64,
    pub scalar_residual: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SolverResponse {
    pub answer: String,
    pub decision_label: String,
    pub route_audit: RouteAudit,
    pub stop_reason: StopReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contradiction_metrics: Option<ResidualMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solved_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default)]
    pub solver: SolverConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub rwif: RwifConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            solver: SolverConfig::default(),
            runtime: RuntimeConfig::default(),
            rwif: RwifConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigLoadError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|source| ConfigLoadError::Io {
            path: path.display().to_string(),
            source,
        })?;
        toml::from_str(&contents).map_err(|source| ConfigLoadError::Parse {
            path: path.display().to_string(),
            source,
        })
    }
}

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("failed to read config at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config at {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SolverConfig {
    #[serde(default = "default_engine_mode")]
    pub engine_mode: String,
    #[serde(default = "default_wrap_mode")]
    pub wrap_mode: String,
    #[serde(default = "default_integer_wrap_mode")]
    pub integer_wrap_mode: String,
    #[serde(default = "default_integration_rule")]
    pub integration_rule: String,
    #[serde(default = "default_quantization_step")]
    pub quantization_step: i64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            engine_mode: default_engine_mode(),
            wrap_mode: default_wrap_mode(),
            integer_wrap_mode: default_integer_wrap_mode(),
            integration_rule: default_integration_rule(),
            quantization_step: default_quantization_step(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_true")]
    pub emit_route_audit: bool,
    #[serde(default = "default_true")]
    pub deterministic_replay: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            emit_route_audit: true,
            deterministic_replay: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RwifConfig {
    #[serde(default = "default_rwif_schema_version")]
    pub schema_version: String,
    #[serde(default = "default_rwif_edge_schema_version")]
    pub edge_schema_version: String,
    #[serde(default = "default_rwif_event_schema_version")]
    pub event_schema_version: String,
}

impl Default for RwifConfig {
    fn default() -> Self {
        Self {
            schema_version: default_rwif_schema_version(),
            edge_schema_version: default_rwif_edge_schema_version(),
            event_schema_version: default_rwif_event_schema_version(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedRange {
    pub min: i16,
    pub max: i16,
}

impl Default for SignedRange {
    fn default() -> Self {
        Self { min: -127, max: 127 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NumericRange {
    #[serde(default)]
    pub amplitude: SignedRange,
    #[serde(default)]
    pub intent: SignedRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseTrajectoryEvent {
    pub timestamp: String,
    pub phase: f64,
    pub confidence_band: f64,
    pub drift_delta: f64,
    pub event_type: String,
    #[serde(default)]
    pub source: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amplitude_signed: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_signed: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_theta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_omega: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantization_step: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monotonic_index: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeRecord {
    pub node_id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub lobe: String,
    #[serde(default)]
    pub provenance: Value,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeRecord {
    pub edge_id: String,
    pub source_node: String,
    pub relation: String,
    pub target_node: String,
    pub lobe: String,
    pub reinforcing: bool,
    pub base_phase: f64,
    pub confidence_band: f64,
    #[serde(default)]
    pub phase_trajectory: Vec<PhaseTrajectoryEvent>,
    #[serde(default)]
    pub provenance: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numeric_range: Option<NumericRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integer_wrap_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrystalRecord {
    pub crystal_id: String,
    pub crystal_label: String,
    pub domain: String,
    pub lobe: String,
    pub frozen: bool,
    #[serde(default)]
    pub nodes: Vec<NodeRecord>,
    #[serde(default)]
    pub edges: Vec<EdgeRecord>,
    #[serde(default)]
    pub version_history: Vec<Value>,
    pub stability_score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rwif_schema_version: Option<String>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankRecord {
    pub bank_id: String,
    pub bank_label: String,
    pub lobe: String,
    #[serde(default)]
    pub crystals: Vec<CrystalRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rwif_schema_version: Option<String>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConformanceSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConformanceIssue {
    pub code: String,
    pub severity: ConformanceSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<ConformanceIssue>,
}

impl ValidationReport {
    fn from_issues(issues: Vec<ConformanceIssue>) -> Self {
        let valid = !issues
            .iter()
            .any(|issue| issue.severity == ConformanceSeverity::Error);
        Self { valid, issues }
    }
}

pub fn migrate_event_to_v2(mut event: PhaseTrajectoryEvent) -> PhaseTrajectoryEvent {
    event.schema_version.get_or_insert_with(default_rwif_event_schema_version);
    event.state_encoding.get_or_insert_with(default_engine_mode);
    event.quantization_step.get_or_insert(default_quantization_step());
    if event.phase_theta.is_none() {
        event.phase_theta = Some(event.phase);
    }
    event
}

pub fn migrate_edge_to_v2(mut edge: EdgeRecord) -> EdgeRecord {
    edge.phase_trajectory = edge
        .phase_trajectory
        .into_iter()
        .map(migrate_event_to_v2)
        .collect();
    edge.schema_version.get_or_insert_with(default_rwif_edge_schema_version);
    edge.state_encoding
        .get_or_insert_with(|| "phase_scalar_v1".to_string());
    edge.numeric_range.get_or_insert_with(NumericRange::default);
    edge.wrap_mode.get_or_insert_with(default_wrap_mode);
    edge.integer_wrap_mode
        .get_or_insert_with(default_integer_wrap_mode);
    edge.integration_rule
        .get_or_insert_with(|| "legacy_scalar".to_string());
    edge
}

pub fn migrate_crystal_to_v2(mut crystal: CrystalRecord) -> CrystalRecord {
    crystal.rwif_schema_version.get_or_insert_with(default_rwif_schema_version);
    crystal.edges = crystal.edges.into_iter().map(migrate_edge_to_v2).collect();
    crystal
}

pub fn migrate_bank_to_v2(mut bank: BankRecord) -> BankRecord {
    bank.rwif_schema_version.get_or_insert_with(default_rwif_schema_version);
    bank.crystals = bank.crystals.into_iter().map(migrate_crystal_to_v2).collect();
    bank
}

pub fn validate_crystal(crystal: &CrystalRecord) -> ValidationReport {
    let mut issues = Vec::new();

    if crystal.rwif_schema_version.as_deref() != Some(RWIF_SCHEMA_VERSION) {
        issues.push(warning(
            "RWIF_CRYSTAL_SCHEMA_VERSION",
            format!(
                "crystal {} rwif_schema_version should be {}",
                crystal.crystal_id, RWIF_SCHEMA_VERSION
            ),
        ));
    }

    for edge in &crystal.edges {
        if edge.schema_version.as_deref() != Some(RWIF_EDGE_SCHEMA_VERSION) {
            issues.push(warning(
                "RWIF_EDGE_SCHEMA_VERSION",
                format!(
                    "edge {} schema_version should be {}",
                    edge.edge_id, RWIF_EDGE_SCHEMA_VERSION
                ),
            ));
        }
        if edge.integer_wrap_mode.is_none() {
            issues.push(error(
                "RWIF_EDGE_INTEGER_WRAP_MODE_MISSING",
                format!("edge {} is missing integer_wrap_mode", edge.edge_id),
            ));
        }
        for (index, event) in edge.phase_trajectory.iter().enumerate() {
            if event.schema_version.as_deref() != Some(RWIF_EVENT_SCHEMA_VERSION) {
                issues.push(warning(
                    "RWIF_EVENT_SCHEMA_VERSION",
                    format!(
                        "edge {} event {} schema_version should be {}",
                        edge.edge_id, index, RWIF_EVENT_SCHEMA_VERSION
                    ),
                ));
            }
            if event.state_encoding.is_none() {
                issues.push(error(
                    "RWIF_EVENT_STATE_ENCODING_MISSING",
                    format!("edge {} event {} is missing state_encoding", edge.edge_id, index),
                ));
            }
            if event.quantization_step.is_none() {
                issues.push(error(
                    "RWIF_EVENT_QUANTIZATION_STEP_MISSING",
                    format!(
                        "edge {} event {} is missing quantization_step",
                        edge.edge_id, index
                    ),
                ));
            }
        }
    }

    ValidationReport::from_issues(issues)
}

pub fn validate_bank(bank: &BankRecord) -> ValidationReport {
    let mut issues = Vec::new();

    if bank.rwif_schema_version.as_deref() != Some(RWIF_SCHEMA_VERSION) {
        issues.push(warning(
            "RWIF_BANK_SCHEMA_VERSION",
            format!("bank {} rwif_schema_version should be {}", bank.bank_id, RWIF_SCHEMA_VERSION),
        ));
    }

    for crystal in &bank.crystals {
        issues.extend(validate_crystal(crystal).issues);
    }

    ValidationReport::from_issues(issues)
}

pub fn solve_linear_equation(request: &SolverRequest, config: &SolverConfig) -> SolverResponse {
    let quantization = config.quantization_step.max(1) as f64;
    let variable = if request.variable.trim().is_empty() {
        "x"
    } else {
        request.variable.trim()
    };

    if request.a.abs() < f64::EPSILON && request.b.abs() < f64::EPSILON {
        return SolverResponse {
            answer: format!("{} has infinitely many valid values", variable),
            decision_label: "underconstrained_identity".to_string(),
            route_audit: RouteAudit {
                stop_reason: StopReason::NoSupportingPath,
                selected_path: vec![
                    RouteStep {
                        edge_id: "parse_linear_equation".to_string(),
                        direction_forward: true,
                    },
                    RouteStep {
                        edge_id: "classify_identity_equation".to_string(),
                        direction_forward: true,
                    },
                ],
                contradiction_residual_quantized: 0,
            },
            stop_reason: StopReason::NoSupportingPath,
            contradiction_metrics: None,
            solved_value: None,
        };
    }

    if request.a.abs() < f64::EPSILON {
        let residual = (request.b.abs() * quantization).round() as i64;
        return SolverResponse {
            answer: format!("{} has no solution because the equation is inconsistent", variable),
            decision_label: "inconsistent_equation".to_string(),
            route_audit: RouteAudit {
                stop_reason: StopReason::ContradictionDetected,
                selected_path: vec![
                    RouteStep {
                        edge_id: "parse_linear_equation".to_string(),
                        direction_forward: true,
                    },
                    RouteStep {
                        edge_id: "detect_zero_slope_contradiction".to_string(),
                        direction_forward: true,
                    },
                ],
                contradiction_residual_quantized: residual,
            },
            stop_reason: StopReason::ContradictionDetected,
            contradiction_metrics: Some(ResidualMetrics {
                phase_residual_quantized: residual,
                scalar_residual: request.b.abs(),
            }),
            solved_value: None,
        };
    }

    let solution = -request.b / request.a;
    SolverResponse {
        answer: format!("{} = {}", variable, solution),
        decision_label: "solved_linear_equation".to_string(),
        route_audit: RouteAudit {
            stop_reason: StopReason::PathFound,
            selected_path: vec![
                RouteStep {
                    edge_id: "parse_linear_equation".to_string(),
                    direction_forward: true,
                },
                RouteStep {
                    edge_id: "classify_non_degenerate_linear_equation".to_string(),
                    direction_forward: true,
                },
                RouteStep {
                    edge_id: "isolate_variable".to_string(),
                    direction_forward: true,
                },
                RouteStep {
                    edge_id: "emit_solution".to_string(),
                    direction_forward: true,
                },
            ],
            contradiction_residual_quantized: 0,
        },
        stop_reason: StopReason::PathFound,
        contradiction_metrics: None,
        solved_value: Some(solution),
    }
}

pub fn replay_signature_for_crystal(crystal: &CrystalRecord) -> Result<String, serde_json::Error> {
    let value = serde_json::json!({
        "crystal_id": crystal.crystal_id,
        "edges": crystal
            .edges
            .iter()
            .map(|edge| serde_json::json!({
                "edge_id": edge.edge_id,
                "state_encoding": edge.state_encoding,
                "wrap_mode": edge.wrap_mode,
                "integer_wrap_mode": edge.integer_wrap_mode,
                "integration_rule": edge.integration_rule,
                "phase_trajectory": edge.phase_trajectory.iter().map(|event| serde_json::json!({
                    "phase": event.phase,
                    "amplitude_signed": event.amplitude_signed,
                    "intent_signed": event.intent_signed,
                    "phase_theta": event.phase_theta,
                    "phase_omega": event.phase_omega,
                    "state_encoding": event.state_encoding,
                    "quantization_step": event.quantization_step,
                    "monotonic_index": event.monotonic_index,
                })).collect::<Vec<_>>()
            }))
            .collect::<Vec<_>>()
    });
    serde_json::to_string(&value)
}

pub fn wrap_pi(theta: f64) -> f64 {
    let tau = std::f64::consts::PI * 2.0;
    let mut wrapped = (theta + std::f64::consts::PI) % tau;
    if wrapped < 0.0 {
        wrapped += tau;
    }
    wrapped - std::f64::consts::PI
}

fn default_engine_mode() -> String {
    DEFAULT_ENGINE_MODE.to_string()
}

fn default_wrap_mode() -> String {
    DEFAULT_WRAP_MODE.to_string()
}

fn default_integer_wrap_mode() -> String {
    "clamp".to_string()
}

fn default_integration_rule() -> String {
    "deterministic_geometric_v1".to_string()
}

fn default_quantization_step() -> i64 {
    1
}

fn default_bind_address() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_true() -> bool {
    true
}

fn default_rwif_schema_version() -> String {
    RWIF_SCHEMA_VERSION.to_string()
}

fn default_rwif_edge_schema_version() -> String {
    RWIF_EDGE_SCHEMA_VERSION.to_string()
}

fn default_rwif_event_schema_version() -> String {
    RWIF_EVENT_SCHEMA_VERSION.to_string()
}

fn warning(code: &str, message: String) -> ConformanceIssue {
    ConformanceIssue {
        code: code.to_string(),
        severity: ConformanceSeverity::Warning,
        message,
    }
}

fn error(code: &str, message: String) -> ConformanceIssue {
    ConformanceIssue {
        code: code.to_string(),
        severity: ConformanceSeverity::Error,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, DEFAULT_ENGINE_MODE, RWIF_SCHEMA_VERSION, SolverRequest, solve_linear_equation,
        wrap_pi,
    };

    #[test]
    fn wrap_pi_stays_in_principal_interval() {
        let wrapped = wrap_pi(4.0 * std::f64::consts::PI);
        assert!((-std::f64::consts::PI..=std::f64::consts::PI).contains(&wrapped));
    }

    #[test]
    fn typed_config_deserializes_with_defaults() {
        let config: AppConfig = toml::from_str("[rwif]\nschema_version = \"RWIF_V2\"\n")
            .expect("config should deserialize");
        assert_eq!(config.solver.engine_mode, DEFAULT_ENGINE_MODE);
        assert_eq!(config.rwif.schema_version, RWIF_SCHEMA_VERSION);
        assert_eq!(config.runtime.bind_address, "0.0.0.0:8080");
    }

    #[test]
    fn linear_solver_returns_unique_solution_route() {
        let result = solve_linear_equation(
            &SolverRequest {
                variable: "x".to_string(),
                a: 2.0,
                b: -4.0,
            },
            &AppConfig::default().solver,
        );

        assert_eq!(result.decision_label, "solved_linear_equation");
        assert_eq!(result.solved_value, Some(2.0));
        assert_eq!(result.route_audit.selected_path.len(), 4);
    }
}