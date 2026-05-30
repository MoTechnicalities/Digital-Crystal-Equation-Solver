use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const DEFAULT_WRAP_MODE: &str = "principal_pi";
pub const DEFAULT_ENGINE_MODE: &str = "signed_i8_plus_intent_v2";
pub const RWIF_SCHEMA_VERSION: &str = "RWIF_V2";
pub const RWIF_EDGE_SCHEMA_VERSION: &str = "RWIF_EDGE_V2";
pub const RWIF_EVENT_SCHEMA_VERSION: &str = "RWIF_EVENT_V2";
const HAFNIAN_EXACT_MAX_DIMENSION: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopReason {
    PathFound,
    NoSupportingPath,
    ContradictionDetected,
    TimeoutOrBudget,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MathMode {
    Algebraic,
    Geometric,
    SymbolicIdentity,
}

impl MathMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Algebraic => "algebraic",
            Self::Geometric => "geometric",
            Self::SymbolicIdentity => "symbolic_identity",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AngleUnit {
    Radians,
    Degrees,
}

impl AngleUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Radians => "radians",
            Self::Degrees => "degrees",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MathOptions {
    pub mode: MathMode,
    pub angle_unit: AngleUnit,
}

impl Default for MathOptions {
    fn default() -> Self {
        Self {
            mode: MathMode::Algebraic,
            angle_unit: AngleUnit::Radians,
        }
    }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ComplexValue {
    pub re: f64,
    pub im: f64,
}

impl ComplexValue {
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }

    pub fn is_real(self) -> bool {
        self.im.abs() < 1e-12
    }

    pub fn is_zero(self) -> bool {
        self.re.abs() < 1e-12 && self.im.abs() < 1e-12
    }
}

impl Add for ComplexValue {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl Sub for ComplexValue {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl Mul for ComplexValue {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

impl Div for ComplexValue {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let denominator = rhs.re * rhs.re + rhs.im * rhs.im;
        Self::new(
            (self.re * rhs.re + self.im * rhs.im) / denominator,
            (self.im * rhs.re - self.re * rhs.im) / denominator,
        )
    }
}

impl Neg for ComplexValue {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.re, -self.im)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatrixValue {
    pub rows: Vec<Vec<MathValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolicStatementValue {
    pub statement: String,
    pub trusted: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MathValue {
    Real(f64),
    Complex(ComplexValue),
    Matrix(MatrixValue),
    Statement(SymbolicStatementValue),
}

impl MathValue {
    fn from_complex(value: ComplexValue) -> Self {
        if value.is_real() {
            Self::Real(value.re)
        } else {
            Self::Complex(value)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MathGeometry {
    pub base_phase: f64,
    pub op: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MathDerivationStep {
    pub step: usize,
    pub rule: String,
    pub expression: String,
    pub latex: String,
    pub result: MathValue,
    pub result_text: String,
    #[serde(default)]
    pub crystal_traces: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numeric_trust: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<MathGeometry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MathPhaseStep {
    pub monotonic_index: usize,
    pub op: String,
    pub inputs: Vec<String>,
    pub output: String,
    pub phase_theta: f64,
    pub cumulative_theta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseSignature {
    pub final_theta: f64,
    pub cumulative_theta: f64,
    pub resonance: f64,
    pub torsion_residual: f64,
    pub torsion_norm: f64,
    pub crystal_state: String,
    pub crystal_class: String,
    #[serde(default)]
    pub trajectory: Vec<MathPhaseStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MathResponse {
    pub object: String,
    pub engine: String,
    pub mode: String,
    pub angle_unit: String,
    pub expression: String,
    pub normalized_expression: String,
    pub latex_expression: String,
    pub result: MathValue,
    pub result_latex: String,
    pub deterministic: bool,
    #[serde(default)]
    pub derivation_trace: Vec<MathDerivationStep>,
    pub bridge_audit: Value,
    pub phase_signature: PhaseSignature,
    pub path_signature: String,
    pub endpoint_signature: String,
    pub rwif_export: CrystalRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformModule {
    pub module_id: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub primary_route: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformCatalog {
    pub platform_id: String,
    pub title: String,
    pub tagline: String,
    #[serde(default)]
    pub shared_capabilities: Vec<String>,
    #[serde(default)]
    pub modules: Vec<PlatformModule>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathError {
    EmptyExpression,
    InvalidMode(String),
    InvalidAngleUnit(String),
    Parse(String),
    Domain(String),
}

impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyExpression => write!(f, "expression must not be empty"),
            Self::InvalidMode(mode) => write!(f, "invalid mode '{}'; expected algebraic or geometric", mode),
            Self::InvalidAngleUnit(unit) => write!(f, "invalid angle_unit '{}'; expected radians or degrees", unit),
            Self::Parse(message) | Self::Domain(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for MathError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Number(f64),
    Identifier(String),
    UnaryNeg(Box<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Function {
        name: String,
        args: Vec<Expr>,
    },
    Matrix(Vec<Vec<Expr>>),
    ConstantI,
    ConstantPi,
    ConstantE,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    End,
}

#[derive(Debug, Clone, PartialEq)]
struct ComplexMatrix {
    rows: Vec<Vec<ComplexValue>>,
}

impl ComplexMatrix {
    fn new(rows: Vec<Vec<ComplexValue>>) -> Result<Self, MathError> {
        if rows.is_empty() {
            return Err(MathError::Domain("matrix literal must contain at least one row".to_string()));
        }
        let width = rows[0].len();
        if width == 0 {
            return Err(MathError::Domain("matrix literal must contain at least one column".to_string()));
        }
        if rows.iter().any(|row| row.len() != width) {
            return Err(MathError::Domain("matrix rows must all have the same length".to_string()));
        }
        Ok(Self { rows })
    }

    fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn column_count(&self) -> usize {
        self.rows.first().map(|row| row.len()).unwrap_or(0)
    }

    fn is_square(&self) -> bool {
        self.row_count() == self.column_count()
    }

    fn into_math_value(self) -> MathValue {
        MathValue::Matrix(MatrixValue {
            rows: self
                .rows
                .into_iter()
                .map(|row| row.into_iter().map(MathValue::from_complex).collect())
                .collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
enum EvalData {
    Scalar(ComplexValue),
    Matrix(ComplexMatrix),
}

impl EvalData {
    fn into_math_value(self) -> MathValue {
        match self {
            Self::Scalar(value) => MathValue::from_complex(value),
            Self::Matrix(matrix) => matrix.into_math_value(),
        }
    }

    fn as_scalar(&self, context: &str) -> Result<ComplexValue, MathError> {
        match self {
            Self::Scalar(value) => Ok(*value),
            Self::Matrix(_) => Err(MathError::Domain(format!("{} requires a scalar value", context))),
        }
    }

    fn as_matrix(&self, context: &str) -> Result<&ComplexMatrix, MathError> {
        match self {
            Self::Matrix(matrix) => Ok(matrix),
            Self::Scalar(_) => Err(MathError::Domain(format!("{} requires a matrix value", context))),
        }
    }
}

#[derive(Debug, Clone)]
struct EvalValue {
    value: EvalData,
    text: String,
}

#[derive(Debug, Default)]
struct EvalContext {
    steps: Vec<MathDerivationStep>,
    trajectory: Vec<MathPhaseStep>,
    bindings: HashMap<String, ComplexValue>,
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

pub fn platform_catalog() -> PlatformCatalog {
    PlatformCatalog {
        platform_id: "digitalcrystal".to_string(),
        title: "DigitalCrystal".to_string(),
        tagline: "Deterministic scientific computation with replayable traces and RWIF export."
            .to_string(),
        shared_capabilities: vec![
            "Deterministic evaluation and replay".to_string(),
            "Phase-aware derivation traces".to_string(),
            "RWIF v2 export surface".to_string(),
            "Typed Rust API boundary".to_string(),
        ],
        modules: vec![
            PlatformModule {
                module_id: "special-functions".to_string(),
                title: "Special Functions Lab".to_string(),
                summary: "Explore advanced functions with deterministic traces, LaTeX output, and phase signatures.".to_string(),
                status: "planned".to_string(),
                primary_route: "/labs/special-functions".to_string(),
                capabilities: vec![
                    "Gamma, zeta, Lambert W, Bessel-class functions".to_string(),
                    "Complex arithmetic traces".to_string(),
                    "RWIF export of evaluated crystals".to_string(),
                ],
            },
            PlatformModule {
                module_id: "quantum".to_string(),
                title: "Quantum and Boson Tools".to_string(),
                summary: "Use hafnian-backed deterministic math for circuit-inspired and Gaussian boson sampling workflows.".to_string(),
                status: "planned".to_string(),
                primary_route: "/labs/quantum".to_string(),
                capabilities: vec![
                    "Complex matrix evaluation".to_string(),
                    "Hafnian-based workloads".to_string(),
                    "Deterministic audit trails".to_string(),
                ],
            },
            PlatformModule {
                module_id: "controls".to_string(),
                title: "Control System Analyzer".to_string(),
                summary: "Inspect transfer-function style expressions, stability cues, and reproducible phase behavior.".to_string(),
                status: "planned".to_string(),
                primary_route: "/labs/controls".to_string(),
                capabilities: vec![
                    "Complex-domain expressions".to_string(),
                    "Phase-derived stability views".to_string(),
                    "Deterministic report generation".to_string(),
                ],
            },
            PlatformModule {
                module_id: "finance".to_string(),
                title: "Deterministic Finance".to_string(),
                summary: "Price and inspect model expressions with strict reproducibility and exportable derivation artifacts.".to_string(),
                status: "planned".to_string(),
                primary_route: "/labs/finance".to_string(),
                capabilities: vec![
                    "Closed-form expression evaluation".to_string(),
                    "Complex characteristic-function support".to_string(),
                    "Audit-friendly result traces".to_string(),
                ],
            },
            PlatformModule {
                module_id: "rwif-explorer".to_string(),
                title: "RWIF Explorer".to_string(),
                summary: "Inspect exported crystals, migration behavior, and replay signatures across the platform.".to_string(),
                status: "foundation".to_string(),
                primary_route: "/labs/rwif".to_string(),
                capabilities: vec![
                    "RWIF validation and migration".to_string(),
                    "Replay signature checks".to_string(),
                    "Schema-aware artifact inspection".to_string(),
                ],
            },
        ],
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

pub fn parse_math_mode(mode: Option<&str>) -> Result<MathMode, MathError> {
    match mode.map(|value| value.trim().to_ascii_lowercase()) {
        None => Ok(MathMode::Algebraic),
        Some(value) if value == "algebraic" => Ok(MathMode::Algebraic),
        Some(value) if value == "geometric" => Ok(MathMode::Geometric),
        Some(value) if matches!(value.as_str(), "symbolic_identity" | "symbolic" | "identity") => {
            Ok(MathMode::SymbolicIdentity)
        }
        Some(other) => Err(MathError::InvalidMode(other)),
    }
}

pub fn parse_angle_unit(unit: Option<&str>) -> Result<AngleUnit, MathError> {
    match unit.map(|value| value.trim().to_ascii_lowercase()) {
        None => Ok(AngleUnit::Radians),
        Some(value) if matches!(value.as_str(), "rad" | "radian" | "radians") => Ok(AngleUnit::Radians),
        Some(value) if matches!(value.as_str(), "deg" | "degree" | "degrees") => Ok(AngleUnit::Degrees),
        Some(other) => Err(MathError::InvalidAngleUnit(other)),
    }
}

pub fn classify_math_error(error: &MathError) -> (&'static str, &'static str) {
    match error {
        MathError::EmptyExpression
        | MathError::InvalidMode(_)
        | MathError::InvalidAngleUnit(_)
        | MathError::Parse(_) => ("parse_error", "MATH_PARSE_ERROR"),
        MathError::Domain(_) => ("domain_error", "MATH_DOMAIN_ERROR"),
    }
}

pub fn build_math_error_bridge_audit(expression: &str, error: &MathError) -> Value {
    let (status, error_code) = classify_math_error(error);
    serde_json::json!({
        "envelope_id": format!("math_error_{}_{}", unix_time_secs(), status),
        "source_text": expression,
        "source_kind": "MathExpression",
        "intent": {
            "intent_id": "math_eval",
            "primary_goal": "EvaluateNumeric",
            "requested_output_mode": "TextAndStructured"
        },
        "semantic_jobs": [
            {
                "job_id": "semantic_route_error",
                "requested_operation": "PreserveFailureContext",
                "status": "failed",
                "input_summary": expression,
                "trace": [error.to_string()]
            }
        ],
        "math_jobs": [],
        "routing_trace": [
            {
                "stage": "error",
                "decision": status,
                "rationale": error.to_string()
            }
        ],
        "job_influence_audit": [
            {
                "job_id": "semantic_route_error",
                "job_kind": "semantic",
                "used_in_final_answer": true,
                "explanation": format!("error path preserved for {}", error_code)
            }
        ],
        "diagnostics": [
            {
                "code": error_code,
                "message": error.to_string(),
                "severity": "error"
            }
        ],
        "final_outcome": {
            "status": "failed",
            "responder_text": error.to_string(),
            "machine_summary": {
                "final_value": Value::Null,
                "confidence": 0.0,
                "contradiction_count": 0
            }
        }
    })
}

pub fn evaluate_math_expression(expression: &str, options: MathOptions) -> Result<MathResponse, MathError> {
    let normalized_expression_owned = normalize_math_input_expression(expression);
    let normalized_expression = normalized_expression_owned.trim();
    if normalized_expression.is_empty() {
        return Err(MathError::EmptyExpression);
    }
    if options.mode == MathMode::SymbolicIdentity {
        return evaluate_symbolic_identity_statement(expression, normalized_expression, options);
    }
    if contains_symbolic_identity_notation(normalized_expression) {
        return Err(MathError::Parse(
            "symbolic identity notation is non-numeric in this slice; switch mode to symbolic_identity to store/display it, or evaluate hafnian(...) with a concrete numeric matrix literal"
                .to_string(),
        ));
    }

    let tokens = tokenize_math(normalized_expression)?;
    let mut parser = Parser::new(tokens);
    let ast = parser.parse_expression()?;
    parser.expect_end()?;

    let mut context = EvalContext::default();
    let result = evaluate_expr(&ast, &options, &mut context)?;
    let final_theta = final_theta_for_result(&result.value);
    let cumulative_theta = context
        .trajectory
        .last()
        .map(|step| step.cumulative_theta)
        .unwrap_or(0.0);
    let torsion_residual = torsion_residual(&context.trajectory);
    let torsion_norm = if context.trajectory.is_empty() {
        0.0
    } else {
        torsion_residual / std::f64::consts::PI
    };
    let (crystal_state, crystal_class) = classify_phase_signature(final_theta, torsion_norm);
    let trajectory = context.trajectory.clone();
    let rwif_export = build_math_rwif_export(normalized_expression, &result.text, &trajectory, final_theta, &crystal_state);
    let phase_signature = PhaseSignature {
        final_theta,
        cumulative_theta,
        resonance: resonance(final_theta),
        torsion_residual,
        torsion_norm,
        crystal_state,
        crystal_class,
        trajectory: trajectory.clone(),
    };
    let bridge_audit = build_success_bridge_audit(
        normalized_expression,
        &context.steps,
        &trajectory,
        &result,
    );
    let path_signature = compute_path_signature(&context.steps);
    let result_value = result.value.clone().into_math_value();
    let result_latex = format!("{} = {}", normalized_expression, result.text);
    let endpoint_signature = compute_endpoint_signature(&result_value, &result_latex, &phase_signature);

    Ok(MathResponse {
        object: "csif.math.result".to_string(),
        engine: "digitalcrystal_math_v2".to_string(),
        mode: options.mode.as_str().to_string(),
        angle_unit: options.angle_unit.as_str().to_string(),
        expression: expression.trim().to_string(),
        normalized_expression: normalized_expression.to_string(),
        latex_expression: normalized_expression.to_string(),
        result: result_value,
        result_latex,
        deterministic: true,
        derivation_trace: context.steps,
        bridge_audit,
        phase_signature,
        path_signature,
        endpoint_signature,
        rwif_export,
    })
}

fn to_sha256_hex(payload: &[u8]) -> String {
    let digest = Sha256::digest(payload);
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn compute_path_signature(steps: &[MathDerivationStep]) -> String {
    let canonical = serde_json::json!(
        steps
            .iter()
            .map(|step| {
                serde_json::json!({
                    "step": step.step,
                    "rule": step.rule,
                    "expression": step.expression,
                    "op": step.geometry.as_ref().map(|geometry| geometry.op.clone()),
                    "base_phase": step.geometry.as_ref().map(|geometry| geometry.base_phase),
                })
            })
            .collect::<Vec<_>>()
    );
    to_sha256_hex(
        &serde_json::to_vec(&canonical).expect("path signature payload should serialize"),
    )
}

fn compute_endpoint_signature(
    result: &MathValue,
    result_latex: &str,
    phase_signature: &PhaseSignature,
) -> String {
    let canonical = serde_json::json!({
        "result": result,
        "result_latex": result_latex,
        "final_theta": phase_signature.final_theta,
        "cumulative_theta": phase_signature.cumulative_theta,
        "crystal_state": phase_signature.crystal_state,
        "crystal_class": phase_signature.crystal_class,
    });
    to_sha256_hex(
        &serde_json::to_vec(&canonical).expect("endpoint signature payload should serialize"),
    )
}

fn normalize_math_input_expression(expression: &str) -> String {
    let mut normalized = expression
        .trim()
        .replace(['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'], "")
        .replace('\u{00A0}', " ")
        .replace('−', "-")
        .replace('×', "*")
        .replace('✕', "*")
        .replace('⋅', "*")
        .replace('∗', "*")
        .replace('÷', "/")
        .replace('Γ', "gamma")
        .replace('γ', "gamma")
        .replace('Ζ', "zeta")
        .replace('ζ', "zeta")
        .replace('σ', "sigma")
        .replace('Σ', "sum");

    normalized = rewrite_latex_integrals(&normalized);
    normalized = replace_latex_fraction_command_until_stable(&normalized, "\\cfrac");
    normalized = replace_latex_fraction_command_until_stable(&normalized, "\\frac");
    normalized = replace_latex_unary_command_until_stable(&normalized, "\\sqrt", "sqrt");

    for (from, to) in [
        ("\\Gamma", "gamma"),
        ("\\gamma", "gamma"),
        ("\\Zeta", "zeta"),
        ("\\zeta", "zeta"),
        ("\\Beta", "beta"),
        ("\\beta", "beta"),
        ("\\Pi", "pi"),
        ("\\pi", "pi"),
        ("\\mathrm{Ai}", "Ai"),
        ("\\operatorname{Ai}", "Ai"),
        ("\\mathrm{Bi}", "Bi"),
        ("\\operatorname{Bi}", "Bi"),
        ("\\mathrm{Si}", "Si"),
        ("\\operatorname{Si}", "Si"),
        ("\\mathrm{Ci}", "Ci"),
        ("\\operatorname{Ci}", "Ci"),
        ("\\mathrm{Ei}", "Ei"),
        ("\\operatorname{Ei}", "Ei"),
        ("\\mathrm{Li}", "li"),
        ("\\operatorname{Li}", "li"),
        ("\\mathrm{W}", "W"),
        ("\\operatorname{W}", "W"),
        ("\\mathrm{Haf}", "hafnian"),
        ("\\operatorname{Haf}", "hafnian"),
        ("\\Haf", "hafnian"),
        ("\\sum", "sum"),
        ("\\prod", "prod"),
        ("\\substack", "substack"),
        ("\\sigma", "sigma"),
        ("\\in", "in"),
        ("\\cdots", "..."),
        ("\\cdot", "*"),
        ("\\times", "*"),
        ("\\div", "/"),
        ("\\infty", "inf"),
        ("\\infin", "inf"),
        ("\\ddots", "2"),
        ("\\,", " "),
    ] {
        normalized = normalized.replace(from, to);
    }

    normalized = normalized
        .replace('{', "(")
        .replace('}', ")");

    normalized = rewrite_single_equation_as_difference(&normalized);

    normalized
}

fn extract_braced_group(input: &str, open_index: usize) -> Option<(String, usize)> {
    if input.as_bytes().get(open_index).copied()? != b'{' {
        return None;
    }
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut close_index = None;
    let mut idx = open_index;
    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    close_index = Some(idx);
                    break;
                }
            }
            _ => {}
        }
        idx += 1;
    }
    let close = close_index?;
    Some((input[open_index + 1..close].to_string(), close + 1))
}

fn replace_latex_unary_command(input: &str, command: &str, target: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0usize;
    while let Some(local_start) = input[cursor..].find(command) {
        let start = cursor + local_start;
        output.push_str(&input[cursor..start]);
        let mut arg_start = start + command.len();
        while arg_start < input.len() && input.as_bytes()[arg_start].is_ascii_whitespace() {
            arg_start += 1;
        }
        if arg_start >= input.len() || input.as_bytes()[arg_start] != b'{' {
            output.push_str(command);
            cursor = start + command.len();
            continue;
        }
        if let Some((arg, next_index)) = extract_braced_group(input, arg_start) {
            output.push_str(target);
            output.push('(');
            output.push_str(&arg);
            output.push(')');
            cursor = next_index;
        } else {
            output.push_str(command);
            cursor = start + command.len();
        }
    }
    output.push_str(&input[cursor..]);
    output
}

fn replace_latex_unary_command_until_stable(input: &str, command: &str, target: &str) -> String {
    let mut current = input.to_string();
    loop {
        let next = replace_latex_unary_command(&current, command, target);
        if next == current {
            return next;
        }
        current = next;
    }
}

fn replace_latex_fraction_command(input: &str, command: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0usize;
    while let Some(local_start) = input[cursor..].find(command) {
        let start = cursor + local_start;
        output.push_str(&input[cursor..start]);

        let mut first_start = start + command.len();
        while first_start < input.len() && input.as_bytes()[first_start].is_ascii_whitespace() {
            first_start += 1;
        }
        if first_start >= input.len() || input.as_bytes()[first_start] != b'{' {
            output.push_str(command);
            cursor = start + command.len();
            continue;
        }
        let Some((numerator, after_numerator)) = extract_braced_group(input, first_start) else {
            output.push_str(command);
            cursor = start + command.len();
            continue;
        };

        let mut second_start = after_numerator;
        while second_start < input.len() && input.as_bytes()[second_start].is_ascii_whitespace() {
            second_start += 1;
        }
        if second_start >= input.len() || input.as_bytes()[second_start] != b'{' {
            output.push_str(command);
            cursor = start + command.len();
            continue;
        }
        let Some((denominator, next_index)) = extract_braced_group(input, second_start) else {
            output.push_str(command);
            cursor = start + command.len();
            continue;
        };

        output.push_str("((");
        output.push_str(&numerator);
        output.push_str(")/(");
        output.push_str(&denominator);
        output.push_str("))");
        cursor = next_index;
    }

    output.push_str(&input[cursor..]);
    output
}

fn replace_latex_fraction_command_until_stable(input: &str, command: &str) -> String {
    let mut current = input.to_string();
    loop {
        let next = replace_latex_fraction_command(&current, command);
        if next == current {
            return next;
        }
        current = next;
    }
}

fn map_latex_infinite_bound(bound: &str) -> String {
    let compact = bound.replace([' ', '\\'], "").to_ascii_lowercase();
    match compact.as_str() {
        "-inf" | "-infty" | "-infinity" => "-12".to_string(),
        "inf" | "infty" | "infinity" => "12".to_string(),
        _ => bound.trim().to_string(),
    }
}

fn rewrite_latex_integrals(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0usize;

    while let Some(local_start) = input[cursor..].find("\\int") {
        let start = cursor + local_start;
        output.push_str(&input[cursor..start]);
        let mut index = start + "\\int".len();

        while index < input.len() && input.as_bytes()[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= input.len() || input.as_bytes()[index] != b'_' {
            output.push_str("\\int");
            cursor = start + "\\int".len();
            continue;
        }
        index += 1;
        while index < input.len() && input.as_bytes()[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= input.len() || input.as_bytes()[index] != b'{' {
            output.push_str("\\int");
            cursor = start + "\\int".len();
            continue;
        }
        let Some((lower, after_lower)) = extract_braced_group(input, index) else {
            output.push_str("\\int");
            cursor = start + "\\int".len();
            continue;
        };
        index = after_lower;

        while index < input.len() && input.as_bytes()[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= input.len() || input.as_bytes()[index] != b'^' {
            output.push_str("\\int");
            cursor = start + "\\int".len();
            continue;
        }
        index += 1;
        while index < input.len() && input.as_bytes()[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= input.len() || input.as_bytes()[index] != b'{' {
            output.push_str("\\int");
            cursor = start + "\\int".len();
            continue;
        }
        let Some((upper, after_upper)) = extract_braced_group(input, index) else {
            output.push_str("\\int");
            cursor = start + "\\int".len();
            continue;
        };

        let remainder = &input[after_upper..];
        let mut differential_start = None;
        let mut differential_end = None;
        let mut variable = None;
        for marker in ["\\, d", "\\,d", " d"] {
            if let Some(local_diff) = remainder.find(marker) {
                let var_start = local_diff + marker.len();
                if var_start < remainder.len() {
                    let c = remainder.as_bytes()[var_start] as char;
                    if c.is_ascii_alphabetic() {
                        differential_start = Some(local_diff);
                        differential_end = Some(var_start + 1);
                        variable = Some(c);
                        break;
                    }
                }
            }
        }

        let (diff_start, diff_end, var) = match (differential_start, differential_end, variable) {
            (Some(start_idx), Some(end_idx), Some(var_name)) => (start_idx, end_idx, var_name),
            _ => {
                output.push_str("\\int");
                cursor = start + "\\int".len();
                continue;
            }
        };

        let integrand = remainder[..diff_start].trim();
        let tail = &remainder[diff_end..];
        output.push_str(&format!(
            "integral({}, {}, {}, {}){}",
            map_latex_infinite_bound(&lower),
            map_latex_infinite_bound(&upper),
            integrand,
            var,
            tail
        ));
        cursor = input.len();
    }

    output.push_str(&input[cursor..]);
    output
}

fn rewrite_single_equation_as_difference(input: &str) -> String {
    if input.contains("==") || input.contains("<=") || input.contains(">=") {
        return input.to_string();
    }
    let mut eq_positions = input.match_indices('=').map(|(idx, _)| idx);
    let Some(position) = eq_positions.next() else {
        return input.to_string();
    };
    if eq_positions.next().is_some() {
        return input.to_string();
    }
    let left = input[..position].trim();
    let right = input[position + 1..].trim();
    if left.is_empty() || right.is_empty() {
        return input.to_string();
    }
    format!("({})-({})", left, right)
}

fn contains_symbolic_identity_notation(expression: &str) -> bool {
    let compact = expression.replace(' ', "").to_ascii_lowercase();
    compact.contains("sum_")
        || compact.contains("prod_")
        || compact.contains("substack")
    || compact.contains("\\begin(pmatrix)")
    || compact.contains("\\end(pmatrix)")
    || compact.contains("\\vdots")
    || compact.contains("\\ddots")
    || compact.contains("\\text(")
}

fn evaluate_symbolic_identity_statement(
    expression: &str,
    normalized_expression: &str,
    options: MathOptions,
) -> Result<MathResponse, MathError> {
    let statement_text = expression.trim();
    let statement = SymbolicStatementValue {
        statement: statement_text.to_string(),
        trusted: true,
        executable: false,
    };
    let result = MathValue::Statement(statement.clone());
    let derivation_trace = vec![MathDerivationStep {
        step: 1,
        rule: "symbolic_identity_statement".to_string(),
        expression: normalized_expression.to_string(),
        latex: normalized_expression.to_string(),
        result: result.clone(),
        result_text: "trusted symbolic statement (non-numeric)".to_string(),
        crystal_traces: vec![],
        numeric_trust: Some(serde_json::json!({
            "kind": "symbolic_identity",
            "trusted": true,
            "executable": false,
            "note": "Stored and displayed as a trusted non-numeric statement"
        })),
        geometry: None,
    }];
    let phase_signature = PhaseSignature {
        final_theta: 0.0,
        cumulative_theta: 0.0,
        resonance: 0.0,
        torsion_residual: 0.0,
        torsion_norm: 0.0,
        crystal_state: "SYMBOLIC".to_string(),
        crystal_class: "statement".to_string(),
        trajectory: vec![],
    };
    let rwif_export = build_math_rwif_export(
        normalized_expression,
        "symbolic_identity_statement",
        &[],
        0.0,
        "SYMBOLIC",
    );
    let path_signature = compute_path_signature(&derivation_trace);
    let endpoint_signature = compute_endpoint_signature(
        &result,
        &format!("{}\\;\\text{{(trusted symbolic statement)}}", normalized_expression),
        &phase_signature,
    );
    let bridge_audit = serde_json::json!({
        "envelope_id": format!("math_symbolic_{}_ok", unix_time_secs()),
        "source_text": expression,
        "source_kind": "MathIdentityStatement",
        "intent": {
            "intent_id": "math_identity_store",
            "primary_goal": "StoreTrustedStatement",
            "requested_output_mode": "TextAndStructured"
        },
        "routing_trace": [
            {
                "stage": "symbolic",
                "decision": "trusted_statement",
                "rationale": "mode symbolic_identity stores the formula as a non-numeric identity"
            }
        ],
        "final_outcome": {
            "status": "symbolic_statement",
            "responder_text": "trusted symbolic statement stored",
            "machine_summary": {
                "final_value": statement_text,
                "confidence": 1.0,
                "contradiction_count": 0
            }
        },
        "diagnostics": []
    });

    Ok(MathResponse {
        object: "csif.math.result".to_string(),
        engine: "digitalcrystal_math_v2".to_string(),
        mode: options.mode.as_str().to_string(),
        angle_unit: options.angle_unit.as_str().to_string(),
        expression: statement_text.to_string(),
        normalized_expression: normalized_expression.to_string(),
        latex_expression: normalized_expression.to_string(),
        result,
        result_latex: format!("{}\\;\\text{{(trusted symbolic statement)}}", normalized_expression),
        deterministic: true,
        derivation_trace,
        bridge_audit,
        phase_signature,
        path_signature,
        endpoint_signature,
        rwif_export,
    })
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

fn tokenize_math(expression: &str) -> Result<Vec<Token>, MathError> {
    let chars: Vec<char> = expression.chars().collect();
    let mut index = 0;
    let mut tokens = Vec::new();

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            let value = expression[start..index]
                .parse::<f64>()
                .map_err(|_| MathError::Parse(format!("invalid number '{}'", &expression[start..index])))?;
            tokens.push(Token::Number(value));
            continue;
        }
        if ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_alphanumeric() || chars[index] == '_') {
                index += 1;
            }
            tokens.push(Token::Ident(expression[start..index].to_string()));
            continue;
        }
        let token = match ch {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '^' => Token::Caret,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => Token::Comma,
            _ => {
                return Err(MathError::Parse(format!("unsupported token '{}'", ch)));
            }
        };
        tokens.push(token);
        index += 1;
    }

    Ok(normalize_implicit_multiplication(tokens))
}

fn normalize_implicit_multiplication(mut tokens: Vec<Token>) -> Vec<Token> {
    let mut normalized = Vec::new();
    for token in tokens.drain(..) {
        if let Some(previous) = normalized.last() {
            if needs_implicit_multiply(previous, &token) {
                normalized.push(Token::Star);
            }
        }
        normalized.push(token);
    }
    normalized.push(Token::End);
    normalized
}

fn needs_implicit_multiply(previous: &Token, next: &Token) -> bool {
    let previous_allows = matches!(previous, Token::Number(_) | Token::Ident(_) | Token::RParen | Token::RBracket);
    let next_allows = matches!(next, Token::Number(_) | Token::Ident(_) | Token::LParen | Token::LBracket);
    if !previous_allows || !next_allows {
        return false;
    }
    !matches!((previous, next), (Token::Ident(_), Token::LParen) | (_, Token::End))
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_expression(&mut self) -> Result<Expr, MathError> {
        self.parse_additive()
    }

    fn expect_end(&self) -> Result<(), MathError> {
        if matches!(self.current(), Token::End) {
            Ok(())
        } else {
            Err(MathError::Parse("unexpected trailing input".to_string()))
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, MathError> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let op = match self.current() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Subtract,
                _ => break,
            };
            self.index += 1;
            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, MathError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = match self.current() {
                Token::Star => BinaryOp::Multiply,
                Token::Slash => BinaryOp::Divide,
                _ => break,
            };
            self.index += 1;
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<Expr, MathError> {
        let left = self.parse_primary()?;
        if matches!(self.current(), Token::Caret) {
            self.index += 1;
            let right = self.parse_power()?;
            Ok(Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Power,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, MathError> {
        if matches!(self.current(), Token::Minus) {
            self.index += 1;
            Ok(Expr::UnaryNeg(Box::new(self.parse_unary()?)))
        } else {
            self.parse_power()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, MathError> {
        match self.current().clone() {
            Token::Number(value) => {
                self.index += 1;
                Ok(Expr::Number(value))
            }
            Token::LBracket => self.parse_matrix_literal(),
            Token::Ident(name) => {
                self.index += 1;
                if matches!(self.current(), Token::LParen) {
                    self.index += 1;
                    let args = self.parse_function_arguments(&name)?;
                    self.index += 1;
                    Ok(Expr::Function { name, args })
                } else {
                    match name.to_ascii_lowercase().as_str() {
                        "i" => Ok(Expr::ConstantI),
                        "pi" => Ok(Expr::ConstantPi),
                        "e" => Ok(Expr::ConstantE),
                        _ => Ok(Expr::Identifier(name)),
                    }
                }
            }
            Token::LParen => {
                self.index += 1;
                let expr = self.parse_expression()?;
                if !matches!(self.current(), Token::RParen) {
                    return Err(MathError::Parse("expected ')'".to_string()));
                }
                self.index += 1;
                Ok(expr)
            }
            _ => Err(MathError::Parse("expected a number, identifier, or parenthesized expression".to_string())),
        }
    }

    fn parse_function_arguments(&mut self, name: &str) -> Result<Vec<Expr>, MathError> {
        let mut args = Vec::new();
        if matches!(self.current(), Token::RParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);
            match self.current() {
                Token::Comma => self.index += 1,
                Token::RParen => break,
                _ => {
                    return Err(MathError::Parse(format!(
                        "expected ',' or ')' after function '{}' argument",
                        name
                    )));
                }
            }
        }

        Ok(args)
    }

    fn parse_matrix_literal(&mut self) -> Result<Expr, MathError> {
        self.index += 1;
        let mut rows = Vec::new();
        loop {
            match self.current() {
                Token::LBracket => {
                    self.index += 1;
                    let mut row = Vec::new();
                    loop {
                        row.push(self.parse_expression()?);
                        match self.current() {
                            Token::Comma => self.index += 1,
                            Token::RBracket => {
                                self.index += 1;
                                break;
                            }
                            _ => {
                                return Err(MathError::Parse(
                                    "expected ',' or ']' in matrix row".to_string(),
                                ));
                            }
                        }
                    }
                    rows.push(row);
                }
                Token::RBracket => {
                    self.index += 1;
                    break;
                }
                Token::Comma => self.index += 1,
                _ => {
                    return Err(MathError::Parse(
                        "expected '[' for matrix row or ']' to end matrix literal".to_string(),
                    ));
                }
            }
        }
        Ok(Expr::Matrix(rows))
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }
}

fn evaluate_expr(expr: &Expr, options: &MathOptions, context: &mut EvalContext) -> Result<EvalValue, MathError> {
    match expr {
        Expr::Number(value) => Ok(EvalValue {
            value: EvalData::Scalar(ComplexValue::new(*value, 0.0)),
            text: format_number(*value),
        }),
        Expr::Identifier(name) => {
            let key = name.to_ascii_lowercase();
            if let Some(value) = context.bindings.get(&key) {
                Ok(EvalValue {
                    value: EvalData::Scalar(*value),
                    text: complex_to_text(*value),
                })
            } else {
                Err(MathError::Parse(format!("unknown identifier '{}'", key)))
            }
        }
        Expr::ConstantI => Ok(EvalValue {
            value: EvalData::Scalar(ComplexValue::new(0.0, 1.0)),
            text: "i".to_string(),
        }),
        Expr::ConstantPi => Ok(EvalValue {
            value: EvalData::Scalar(ComplexValue::new(std::f64::consts::PI, 0.0)),
            text: "pi".to_string(),
        }),
        Expr::ConstantE => Ok(EvalValue {
            value: EvalData::Scalar(ComplexValue::new(std::f64::consts::E, 0.0)),
            text: "e".to_string(),
        }),
        Expr::Matrix(rows) => {
            let mut evaluated_rows = Vec::with_capacity(rows.len());
            let mut text_rows = Vec::with_capacity(rows.len());
            for row in rows {
                let mut evaluated_row = Vec::with_capacity(row.len());
                let mut text_row = Vec::with_capacity(row.len());
                for cell in row {
                    let value = evaluate_expr(cell, options, context)?;
                    evaluated_row.push(value.value.as_scalar("matrix literal")?);
                    text_row.push(value.text);
                }
                evaluated_rows.push(evaluated_row);
                text_rows.push(text_row);
            }
            let matrix = ComplexMatrix::new(evaluated_rows)?;
            let text = format!(
                "[{}]",
                text_rows
                    .iter()
                    .map(|row| format!("[{}]", row.join(", ")))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            Ok(EvalValue {
                value: EvalData::Matrix(matrix),
                text,
            })
        }
        Expr::UnaryNeg(inner) => {
            let value = evaluate_expr(inner, options, context)?;
            let result = match value.value {
                EvalData::Scalar(scalar) => EvalData::Scalar(-scalar),
                EvalData::Matrix(matrix) => EvalData::Matrix(ComplexMatrix::new(
                    matrix
                        .rows
                        .into_iter()
                        .map(|row| row.into_iter().map(|cell| -cell).collect())
                        .collect(),
                )?),
            };
            let text = eval_data_to_text(&result);
            push_math_step(
                context,
                "unary_negation",
                format!("-({})", value.text),
                "-",
                result.clone(),
                vec![value.text],
                text.clone(),
            );
            Ok(EvalValue { value: result, text })
        }
        Expr::Binary { left, op, right } => {
            let left_value = evaluate_expr(left, options, context)?;
            let right_value = evaluate_expr(right, options, context)?;
            let left_scalar = left_value.value.as_scalar("binary operator")?;
            let right_scalar = right_value.value.as_scalar("binary operator")?;
            let (result, rule, symbol) = if options.mode == MathMode::Geometric && left_scalar.is_real() && right_scalar.is_real() {
                match op {
                    BinaryOp::Add => {
                        let exact = geometric_real_binary_result(left_value.text.as_str(), right_value.text.as_str(), op)?;
                        (exact, "addition", "+")
                    }
                    BinaryOp::Subtract => {
                        let exact = geometric_real_binary_result(left_value.text.as_str(), right_value.text.as_str(), op)?;
                        (exact, "subtraction", "-")
                    }
                    BinaryOp::Multiply => {
                        let exact = geometric_real_binary_result(left_value.text.as_str(), right_value.text.as_str(), op)?;
                        (exact, "multiplication", "*")
                    }
                    BinaryOp::Divide => {
                        if right_scalar.is_zero() {
                            return Err(MathError::Domain("division by zero".to_string()));
                        }
                        let exact = geometric_real_binary_result(left_value.text.as_str(), right_value.text.as_str(), op)?;
                        (exact, "division", "/")
                    }
                    BinaryOp::Power => {
                        if left_scalar.is_real() && right_scalar.is_real() {
                            let base = left_scalar.re;
                            let exponent = right_scalar.re;
                            if base == 0.0 && exponent <= 0.0 {
                                return Err(MathError::Domain("zero cannot be raised to this power".to_string()));
                            }
                            (
                                ComplexValue::new(base.powf(exponent), 0.0),
                                "exponentiation",
                                "^",
                            )
                        } else {
                            (c_pow(left_scalar, right_scalar)?, "exponentiation", "^")
                        }
                    }
                }
            } else {
                match op {
                    BinaryOp::Add => (left_scalar + right_scalar, "addition", "+"),
                    BinaryOp::Subtract => (left_scalar - right_scalar, "subtraction", "-"),
                    BinaryOp::Multiply => (left_scalar * right_scalar, "multiplication", "*"),
                    BinaryOp::Divide => {
                        if right_scalar.is_zero() {
                            return Err(MathError::Domain("division by zero".to_string()));
                        }
                        (left_scalar / right_scalar, "division", "/")
                    }
                    BinaryOp::Power => {
                        if left_scalar.is_real() && right_scalar.is_real() {
                            let base = left_scalar.re;
                            let exponent = right_scalar.re;
                            if base == 0.0 && exponent <= 0.0 {
                                return Err(MathError::Domain("zero cannot be raised to this power".to_string()));
                            }
                            (
                                ComplexValue::new(base.powf(exponent), 0.0),
                                "exponentiation",
                                "^",
                            )
                        } else {
                            (c_pow(left_scalar, right_scalar)?, "exponentiation", "^")
                        }
                    }
                }
            };
            let text = complex_to_text(result);
            push_math_step(
                context,
                rule,
                format!("{} {} {}", left_value.text, symbol, right_value.text),
                symbol,
                EvalData::Scalar(result),
                vec![left_value.text, right_value.text],
                text.clone(),
            );
            Ok(EvalValue { value: EvalData::Scalar(result), text })
        }
        Expr::Function { name, args } => {
            let normalized = canonical_function_name(name);

            if normalized == "integral" {
                let result = evaluate_integral_function(args, options, context)?;
                let text = eval_data_to_text(&result);
                push_math_step(
                    context,
                    "definite_integral",
                    format!("integral({})", args.len()),
                    "integral",
                    result.clone(),
                    vec![],
                    text.clone(),
                );
                return Ok(EvalValue { value: result, text });
            }

            let evaluated_args = args
                .iter()
                .map(|arg| evaluate_expr(arg, options, context))
                .collect::<Result<Vec<_>, _>>()?;
            let arg_texts = evaluated_args
                .iter()
                .map(|value| value.text.clone())
                .collect::<Vec<_>>();
            let mut trust_override: Option<Value> = None;
            let (result, rule, op) = match normalized.as_str() {
                "abs" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(ComplexValue::new(value.abs(), 0.0)), "absolute_value", "abs")
                }
                "arg" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(ComplexValue::new(value.arg(), 0.0)), "argument", "arg")
                }
                "conj" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(value.conj()), "complex_conjugate", "conj")
                }
                "exp" => {
                    let value = input_to_radians(unary_scalar_arg(&normalized, &evaluated_args)?, options.angle_unit, &normalized);
                    (EvalData::Scalar(c_exp(value)), "exponential", "exp")
                }
                "ln" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_log(value)?), "natural_logarithm", "ln")
                }
                "log" => match evaluated_args.as_slice() {
                    [value] => (
                        EvalData::Scalar(c_log(value.value.as_scalar("log")?)? / ComplexValue::new(10.0_f64.ln(), 0.0)),
                        "common_logarithm",
                        "log",
                    ),
                    [value, base] => {
                        let base_log = c_log(base.value.as_scalar("log base")?)?;
                        if base_log.abs() < 1e-12 {
                            return Err(MathError::Domain("log base must not evaluate to 1".to_string()));
                        }
                        (
                            EvalData::Scalar(c_log(value.value.as_scalar("log")?)? / base_log),
                            "logarithm_base_n",
                            "log",
                        )
                    }
                    _ => {
                        return Err(MathError::Parse(format!(
                            "function '{}' expects 1 or 2 arguments",
                            normalized
                        )));
                    }
                },
                "gamma" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_gamma(value)?), "gamma_function", "gamma")
                }
                "beta" => {
                    let [a, b] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_beta(a, b)?), "beta_function", "beta")
                }
                "lambertw" | "w" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_lambertw(value)?), "lambert_w", "lambertw")
                }
                "zeta" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_zeta(value)?), "riemann_zeta", "zeta")
                }
                "erf" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_erf(value)?), "error_function", "erf")
                }
                "si" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_si(value)?), "sine_integral", "si")
                }
                "ci" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_ci(value)?), "cosine_integral", "ci")
                }
                "fresnelc" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_fresnel_c(value)?), "fresnel_c", "fresnelc")
                }
                "fresnels" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_fresnel_s(value)?), "fresnel_s", "fresnels")
                }
                "ei" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_ei(value)?), "exponential_integral", "ei")
                }
                "li" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_li(value)?), "logarithmic_integral", "li")
                }
                "ai" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_ai(value)?), "airy_ai", "ai")
                }
                "bi" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bi(value)?), "airy_bi", "bi")
                }
                "theta4" => {
                    let [z, q] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_theta4(z, q)?), "jacobi_theta4", "theta4")
                }
                "polylog" => {
                    let [s, z] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_polylog(s, z)?), "polylogarithm", "polylog")
                }
                "gammainc" => {
                    let [a, z] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_gammainc(a, z)?), "incomplete_gamma", "gammainc")
                }
                "besselj" => {
                    let [order, z] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_j(bessel_order(order, &normalized)?, z)?), "bessel_j", "besselj")
                }
                "bessely" => {
                    let [order, z] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_y(bessel_order(order, &normalized)?, z)?), "bessel_y", "bessely")
                }
                "besseli" => {
                    let [order, z] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_i(bessel_order(order, &normalized)?, z)?), "bessel_i", "besseli")
                }
                "besselk" => {
                    let [order, z] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_k(bessel_order(order, &normalized)?, z)?), "bessel_k", "besselk")
                }
                "j_sph" => {
                    let [order, z] = binary_scalar_args(&normalized, &evaluated_args)?;
                    (
                        EvalData::Scalar(c_spherical_bessel_j(bessel_order(order, &normalized)?, z)?),
                        "spherical_bessel_j",
                        "j_sph",
                    )
                }
                "j0" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_j(0, value)?), "bessel_j0", "j0")
                }
                "j1" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_j(1, value)?), "bessel_j1", "j1")
                }
                "j2" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_j(2, value)?), "bessel_j2", "j2")
                }
                "j3" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_j(3, value)?), "bessel_j3", "j3")
                }
                "y0" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_y(0, value)?), "bessel_y0", "y0")
                }
                "y1" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_y(1, value)?), "bessel_y1", "y1")
                }
                "i0" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_i(0, value)?), "bessel_i0", "i0")
                }
                "i1" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_i(1, value)?), "bessel_i1", "i1")
                }
                "k0" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_k(0, value)?), "bessel_k0", "k0")
                }
                "k1" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_bessel_k(1, value)?), "bessel_k1", "k1")
                }
                "det" => {
                    let matrix = unary_matrix_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(matrix_determinant(matrix)?), "determinant_matrix", "det")
                }
                "inverse" => {
                    let matrix = unary_matrix_arg(&normalized, &evaluated_args)?;
                    (EvalData::Matrix(matrix_inverse(matrix)?), "inverse_matrix", "inverse")
                }
                "hafnian" => {
                    let matrix = unary_matrix_arg(&normalized, &evaluated_args)?;
                    let hafnian_value = matrix_hafnian(matrix)?;
                    trust_override = Some(serde_json::json!({
                        "approximate": false,
                        "method": "exact recursive hafnian (dimension-capped)",
                        "complexity": "exponential worst-case in matrix dimension",
                        "expression": format!("{}({})", normalized, arg_texts.join(", ")),
                        "output": complex_to_text(hafnian_value),
                        "hafnian_flux_probe": hafnian_flux_probe(matrix, hafnian_value),
                    }));
                    (EvalData::Scalar(hafnian_value), "hafnian_matrix", "hafnian")
                }
                "tf" | "transfer" | "tf_eval" => {
                    let (num, den, s) = tf_args(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(transfer_function_eval(num, den, s)?), "transfer_function_eval", "tf")
                }
                "sqrt" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_sqrt(value)), "square_root", "sqrt")
                }
                "sinc" => {
                    let value = unary_scalar_arg(&normalized, &evaluated_args)?;
                    (EvalData::Scalar(c_sinc(value)), "sinc_function", "sinc")
                }
                "sin" => {
                    let value = input_to_radians(unary_scalar_arg(&normalized, &evaluated_args)?, options.angle_unit, &normalized);
                    (EvalData::Scalar(c_sin(value)), "sine", "sin")
                }
                "cos" => {
                    let value = input_to_radians(unary_scalar_arg(&normalized, &evaluated_args)?, options.angle_unit, &normalized);
                    (EvalData::Scalar(c_cos(value)), "cosine", "cos")
                }
                _ => {
                    return Err(MathError::Parse(format!("function '{}' is not implemented in this slice", normalized)));
                }
            };
            let text = eval_data_to_text(&result);
            push_math_step_with_trust(
                context,
                rule,
                format!("{}({})", normalized, arg_texts.join(", ")),
                op,
                result.clone(),
                arg_texts,
                text.clone(),
                trust_override,
            );
            Ok(EvalValue { value: result, text })
        }
    }
}

fn unary_scalar_arg(name: &str, args: &[EvalValue]) -> Result<ComplexValue, MathError> {
    match args {
        [value] => value.value.as_scalar(name),
        _ => Err(MathError::Parse(format!("function '{}' expects 1 argument", name))),
    }
}

fn unary_matrix_arg<'a>(name: &str, args: &'a [EvalValue]) -> Result<&'a ComplexMatrix, MathError> {
    match args {
        [value] => value.value.as_matrix(name),
        _ => Err(MathError::Parse(format!("function '{}' expects 1 argument", name))),
    }
}

fn binary_scalar_args(name: &str, args: &[EvalValue]) -> Result<[ComplexValue; 2], MathError> {
    match args {
        [left, right] => Ok([left.value.as_scalar(name)?, right.value.as_scalar(name)?]),
        _ => Err(MathError::Parse(format!("function '{}' expects 2 arguments", name))),
    }
}

fn tf_args<'a>(name: &str, args: &'a [EvalValue]) -> Result<(&'a ComplexMatrix, &'a ComplexMatrix, ComplexValue), MathError> {
    match args {
        [num, den, s] => Ok((
            num.value.as_matrix(name)?,
            den.value.as_matrix(name)?,
            s.value.as_scalar(name)?,
        )),
        _ => Err(MathError::Parse(format!("function '{}' expects 3 arguments", name))),
    }
}

fn evaluate_integral_function(
    args: &[Expr],
    options: &MathOptions,
    context: &mut EvalContext,
) -> Result<EvalData, MathError> {
    if args.len() != 4 {
        return Err(MathError::Parse("function 'integral' expects 4 arguments".to_string()));
    }

    let lower = evaluate_expr(&args[0], options, context)?.value.as_scalar("integral lower bound")?;
    let upper = evaluate_expr(&args[1], options, context)?.value.as_scalar("integral upper bound")?;
    if !lower.is_real() || !upper.is_real() {
        return Err(MathError::Domain("integral bounds must be real values".to_string()));
    }

    let variable_name = match &args[3] {
        Expr::Identifier(name) => name.to_ascii_lowercase(),
        Expr::ConstantI => "i".to_string(),
        Expr::ConstantPi => "pi".to_string(),
        Expr::ConstantE => "e".to_string(),
        _ => {
            return Err(MathError::Parse(
                "integral variable argument must be an identifier (for example: x)".to_string(),
            ));
        }
    };

    let mut step_count = ((upper.re - lower.re).abs() * 800.0).ceil() as usize + 800;
    if step_count % 2 == 1 {
        step_count += 1;
    }
    let h = (upper.re - lower.re) / step_count as f64;

    let mut acc = ComplexValue::new(0.0, 0.0);
    for i in 0..=step_count {
        let x = lower.re + i as f64 * h;
        let weight = if i == 0 || i == step_count {
            1.0
        } else if i % 2 == 0 {
            2.0
        } else {
            4.0
        };

        let mut local_context = EvalContext::default();
        local_context
            .bindings
            .insert(variable_name.clone(), ComplexValue::new(x, 0.0));
        let integrand = evaluate_expr(&args[2], options, &mut local_context)?
            .value
            .as_scalar("integral integrand")?;
        acc = acc + integrand * ComplexValue::new(weight, 0.0);
    }

    Ok(EvalData::Scalar(acc * ComplexValue::new(h / 3.0, 0.0)))
}

fn canonical_function_name(name: &str) -> String {
    let lowered = name.to_ascii_lowercase();
    match lowered.as_str() {
        "b" => "beta".to_string(),
        _ => lowered,
    }
}

fn bessel_order(value: ComplexValue, function_name: &str) -> Result<usize, MathError> {
    if !value.is_real() || value.re < 0.0 {
        return Err(MathError::Domain(format!(
            "{} requires a non-negative real order",
            function_name
        )));
    }

    let rounded = value.re.round();
    if (rounded - value.re).abs() > 1e-12 {
        return Err(MathError::Domain(format!(
            "{} requires an integer order",
            function_name
        )));
    }

    Ok(rounded as usize)
}

fn input_to_radians(value: ComplexValue, angle_unit: AngleUnit, function_name: &str) -> ComplexValue {
    match (angle_unit, function_name) {
        (AngleUnit::Degrees, "sin") | (AngleUnit::Degrees, "cos") => {
            let factor = std::f64::consts::PI / 180.0;
            ComplexValue::new(value.re * factor, value.im * factor)
        }
        _ => value,
    }
}

fn push_math_step(
    context: &mut EvalContext,
    rule: &str,
    expression: String,
    op: &str,
    result: EvalData,
    inputs: Vec<String>,
    output: String,
) {
    push_math_step_with_trust(context, rule, expression, op, result, inputs, output, None);
}

fn push_math_step_with_trust(
    context: &mut EvalContext,
    rule: &str,
    expression: String,
    op: &str,
    result: EvalData,
    inputs: Vec<String>,
    output: String,
    trust_override: Option<Value>,
) {
    let step_index = context.steps.len() + 1;
    let phase_theta = phase_for_operation(op);
    let cumulative_theta = wrap_pi(
        context
            .trajectory
            .last()
            .map(|step| step.cumulative_theta)
            .unwrap_or(0.0)
            + phase_theta,
    );
    context.steps.push(MathDerivationStep {
        step: step_index,
        rule: rule.to_string(),
        expression: expression.clone(),
        latex: expression.clone(),
        result: result.clone().into_math_value(),
        result_text: output.clone(),
        crystal_traces: Vec::new(),
        numeric_trust: trust_override.or_else(|| numeric_trust_metadata(rule, op, &expression, &result, &inputs)),
        geometry: Some(MathGeometry {
            base_phase: phase_theta,
            op: op.to_string(),
        }),
    });
    context.trajectory.push(MathPhaseStep {
        monotonic_index: step_index,
        op: rule.to_string(),
        inputs,
        output,
        phase_theta,
        cumulative_theta,
    });
}

fn hafnian_flux_probe(matrix: &ComplexMatrix, hafnian_value: ComplexValue) -> Value {
    let n = matrix.row_count();
    let mut pair_count = 0usize;
    let mut unit_sum = ComplexValue::new(0.0, 0.0);
    let mut phase_samples = Vec::new();
    let mut magnitude_samples = Vec::new();
    let mut symmetry_gap_acc = 0.0;
    let mut max_diag_abs: f64 = 0.0;

    for i in 0..n {
        max_diag_abs = max_diag_abs.max(matrix.rows[i][i].abs());
        for j in (i + 1)..n {
            let a = matrix.rows[i][j];
            let b = matrix.rows[j][i];
            symmetry_gap_acc += (a - b).abs();

            let magnitude = a.abs();
            if magnitude > 1e-12 {
                let unit = a / ComplexValue::new(magnitude, 0.0);
                unit_sum = unit_sum + unit;
                phase_samples.push(a.arg());
                magnitude_samples.push(magnitude);
                pair_count += 1;
            }
        }
    }

    let coherence = if pair_count == 0 {
        0.0
    } else {
        unit_sum.abs() / pair_count as f64
    };
    let mean_phase = if pair_count == 0 {
        0.0
    } else {
        unit_sum.arg()
    };
    let symmetry_gap = if pair_count == 0 {
        0.0
    } else {
        symmetry_gap_acc / pair_count as f64
    };

    let (mean_magnitude, magnitude_cv) = if magnitude_samples.is_empty() {
        (0.0, 0.0)
    } else {
        let mean = magnitude_samples.iter().sum::<f64>() / magnitude_samples.len() as f64;
        if mean <= 1e-12 {
            (mean, 0.0)
        } else {
            let variance = magnitude_samples
                .iter()
                .map(|value| {
                    let delta = value - mean;
                    delta * delta
                })
                .sum::<f64>()
                / magnitude_samples.len() as f64;
            (mean, variance.sqrt() / mean)
        }
    };

    let observed_theta = hafnian_value.arg();
    let predicted_theta = wrap_pi((n as f64 / 2.0) * mean_phase);
    let residual = wrap_pi(observed_theta - predicted_theta);

    serde_json::json!({
        "experimental": true,
        "dimension": n,
        "off_diagonal_pairs": pair_count,
        "coherence_magnitude": coherence,
        "mean_edge_phase": mean_phase,
        "mean_edge_magnitude": mean_magnitude,
        "magnitude_coefficient_of_variation": magnitude_cv,
        "symmetry_gap_mean_abs": symmetry_gap,
        "diagonal_max_abs": max_diag_abs,
        "observed_hafnian_theta": observed_theta,
        "predicted_uniform_theta": predicted_theta,
        "uniform_phase_residual": residual,
        "hypothesis": "If edge phases are coherent and symmetric, hafnian phase tends to follow n/2 * mean edge phase.",
    })
}

fn numeric_trust_metadata(
    rule: &str,
    _op: &str,
    expression: &str,
    result: &EvalData,
    inputs: &[String],
) -> Option<Value> {
    let output_text = eval_data_to_text(result);
    let metadata = match rule {
        "gamma_function" => serde_json::json!({
            "approximate": true,
            "method": "Lanczos approximation with reflection",
            "truncation": "fixed 9-coefficient table",
            "estimated_error": "typically < 1e-12 for well-conditioned inputs",
            "expression": expression,
            "output": output_text,
        }),
        "riemann_zeta" => serde_json::json!({
            "approximate": true,
            "method": "Dirichlet eta series with analytic continuation",
            "truncation": "up to 20,000 terms",
            "estimated_error": "term threshold 1e-14",
            "expression": expression,
            "output": output_text,
        }),
        "polylogarithm" => serde_json::json!({
            "approximate": true,
            "method": "power series plus continuation branch",
            "truncation": "up to 20,000 terms in the small-|z| branch",
            "estimated_error": "term threshold 1e-14",
            "expression": expression,
            "output": output_text,
        }),
        "incomplete_gamma" => serde_json::json!({
            "approximate": true,
            "method": "series / recurrence approximation",
            "truncation": "bounded series iteration",
            "estimated_error": "term threshold 1e-14",
            "expression": expression,
            "output": output_text,
        }),
        "error_function" => serde_json::json!({
            "approximate": true,
            "method": "power series",
            "truncation": "up to 120 terms",
            "estimated_error": "term threshold 1e-14",
            "expression": expression,
            "output": output_text,
        }),
        "sine_integral" | "cosine_integral" | "fresnel_c" | "fresnel_s" => serde_json::json!({
            "approximate": true,
            "method": "Simpson-rule quadrature",
            "truncation": "fixed step grid sized from input magnitude",
            "estimated_error": "integration truncation depends on step count",
            "expression": expression,
            "output": output_text,
        }),
        "exponential_integral" | "logarithmic_integral" => serde_json::json!({
            "approximate": true,
            "method": "series-based exponential/logarithmic integral approximation",
            "truncation": "bounded by series convergence",
            "estimated_error": "term threshold 1e-15",
            "expression": expression,
            "output": output_text,
        }),
        "airy_ai" | "airy_bi" => serde_json::json!({
            "approximate": true,
            "method": "Bessel-based continuation identity",
            "truncation": "finite special-function evaluation",
            "estimated_error": "inherits underlying Bessel approximation error",
            "expression": expression,
            "output": output_text,
        }),
        "jacobi_theta4" => serde_json::json!({
            "approximate": true,
            "method": "truncated Jacobi theta series",
            "truncation": "up to 300 terms",
            "estimated_error": "term threshold 1e-14",
            "expression": expression,
            "output": output_text,
        }),
        "definite_integral" => serde_json::json!({
            "approximate": true,
            "method": "Simpson-rule quadrature",
            "truncation": "fixed step grid sized from interval width",
            "estimated_error": "depends on integrand smoothness and step count",
            "inputs": inputs,
            "expression": expression,
            "output": output_text,
        }),
        "bessel_j" | "bessel_i" | "bessel_y" | "bessel_k" | "spherical_bessel_j" => serde_json::json!({
            "approximate": true,
            "method": "special-function series / continuation",
            "truncation": "bounded term iteration",
            "estimated_error": "term threshold 1e-14 to 1e-15",
            "expression": expression,
            "output": output_text,
        }),
        _ => return None,
    };

    Some(metadata)
}

fn phase_for_operation(op: &str) -> f64 {
    match op {
        "+" => 0.0,
        "-" => std::f64::consts::PI / 4.0,
        "*" => std::f64::consts::PI / 6.0,
        "/" => std::f64::consts::PI / 2.0,
        "^" => std::f64::consts::PI / 3.0,
        "sqrt" => -std::f64::consts::PI / 6.0,
        "exp" => std::f64::consts::PI / 4.0,
        "sin" | "cos" => std::f64::consts::PI / 8.0,
        "conj" => -std::f64::consts::PI / 8.0,
        "arg" => std::f64::consts::PI / 5.0,
        "ln" | "log" => -std::f64::consts::PI / 4.0,
        "gamma" => std::f64::consts::PI / 12.0,
        "lambertw" => std::f64::consts::PI / 13.0,
        "zeta" => std::f64::consts::PI / 14.0,
        "polylog" => std::f64::consts::PI / 15.0,
        "gammainc" => std::f64::consts::PI / 16.0,
        "besselj" | "j0" | "j1" | "j2" | "j3" => std::f64::consts::PI / 10.0,
        "bessely" | "y0" | "y1" => -std::f64::consts::PI / 10.0,
        "besseli" | "i0" | "i1" => std::f64::consts::PI / 9.0,
        "besselk" | "k0" | "k1" => -std::f64::consts::PI / 9.0,
        "j_sph" => std::f64::consts::PI / 11.0,
        "det" => std::f64::consts::PI / 7.0,
        "inverse" => -std::f64::consts::PI / 7.0,
        "tf" => std::f64::consts::PI / 18.0,
        "abs" => 0.0,
        _ => 0.0,
    }
}

fn final_theta_for_result(value: &EvalData) -> f64 {
    match value {
        EvalData::Scalar(scalar) => {
            if scalar.is_zero() {
                0.0
            } else {
                wrap_pi(scalar.arg())
            }
        }
        EvalData::Matrix(matrix) => {
            let combined = matrix
                .rows
                .iter()
                .flatten()
                .copied()
                .fold(ComplexValue::new(0.0, 0.0), |acc, value| acc + value);
            if combined.is_zero() {
                0.0
            } else {
                wrap_pi(combined.arg())
            }
        }
    }
}

fn torsion_residual(trajectory: &[MathPhaseStep]) -> f64 {
    trajectory
        .windows(2)
        .map(|pair| wrap_pi(pair[1].phase_theta - pair[0].phase_theta).abs())
        .sum()
}

fn resonance(theta: f64) -> f64 {
    1.0 - (wrap_pi(theta).abs() / std::f64::consts::PI)
}

fn classify_phase_signature(final_theta: f64, torsion_norm: f64) -> (String, String) {
    if final_theta.abs() < 0.05 && torsion_norm < 1.0 {
        ("YES".to_string(), "yes".to_string())
    } else if final_theta.abs() > std::f64::consts::PI * 0.85 {
        ("NO".to_string(), "no".to_string())
    } else {
        ("NEEDS_INPUT".to_string(), "need".to_string())
    }
}

fn build_math_rwif_export(
    expression: &str,
    result_text: &str,
    trajectory: &[MathPhaseStep],
    final_theta: f64,
    crystal_state: &str,
) -> CrystalRecord {
    let timestamp = unix_time_secs();
    let phase_trajectory = trajectory
        .iter()
        .map(|step| PhaseTrajectoryEvent {
            timestamp: timestamp.to_string(),
            phase: step.phase_theta,
            confidence_band: resonance(step.phase_theta),
            drift_delta: 0.0,
            event_type: step.op.clone(),
            source: serde_json::json!({"type": "digitalcrystal_math_v2"}),
            amplitude_signed: None,
            intent_signed: None,
            phase_theta: Some(step.phase_theta),
            phase_omega: None,
            state_encoding: Some(DEFAULT_ENGINE_MODE.to_string()),
            quantization_step: Some(1),
            monotonic_index: Some(step.monotonic_index as i64),
            schema_version: Some(RWIF_EVENT_SCHEMA_VERSION.to_string()),
            extra: BTreeMap::from([
                ("inputs".to_string(), serde_json::json!(step.inputs)),
                ("output".to_string(), serde_json::json!(step.output)),
                ("cumulative_theta".to_string(), serde_json::json!(step.cumulative_theta)),
            ]),
        })
        .collect::<Vec<_>>();

    CrystalRecord {
        crystal_id: format!("calc_{}", timestamp),
        crystal_label: "DigitalCrystal Math Export".to_string(),
        domain: "csif_scientific_math".to_string(),
        lobe: "symbolic".to_string(),
        frozen: false,
        nodes: vec![
            NodeRecord {
                node_id: "node_expression".to_string(),
                label: expression.to_string(),
                aliases: Vec::new(),
                lobe: "symbolic".to_string(),
                provenance: serde_json::json!({"role": "expression", "source": "digitalcrystal"}),
                extra: BTreeMap::new(),
            },
            NodeRecord {
                node_id: "node_result".to_string(),
                label: result_text.to_string(),
                aliases: Vec::new(),
                lobe: "symbolic".to_string(),
                provenance: serde_json::json!({"role": "result", "source": "digitalcrystal"}),
                extra: BTreeMap::new(),
            },
        ],
        edges: vec![EdgeRecord {
            edge_id: "edge_eval_path".to_string(),
            source_node: "node_expression".to_string(),
            relation: "evaluates_to".to_string(),
            target_node: "node_result".to_string(),
            lobe: "symbolic".to_string(),
            reinforcing: crystal_state != "NO",
            base_phase: trajectory.first().map(|step| step.phase_theta).unwrap_or(final_theta),
            confidence_band: resonance(final_theta),
            phase_trajectory,
            provenance: serde_json::json!({"source": "digitalcrystal", "export_mode": "strict_rwif_v2"}),
            state_encoding: Some(DEFAULT_ENGINE_MODE.to_string()),
            numeric_range: Some(NumericRange::default()),
            wrap_mode: Some(DEFAULT_WRAP_MODE.to_string()),
            integer_wrap_mode: Some(default_integer_wrap_mode()),
            integration_rule: Some("scientific_parser_v1".to_string()),
            schema_version: Some(RWIF_EDGE_SCHEMA_VERSION.to_string()),
            extra: BTreeMap::new(),
        }],
        version_history: vec![serde_json::json!({
            "timestamp": timestamp,
            "note": "Exported from DigitalCrystal Rust math engine"
        })],
        stability_score: resonance(final_theta),
        rwif_schema_version: Some(RWIF_SCHEMA_VERSION.to_string()),
        extra: BTreeMap::new(),
    }
}

fn build_success_bridge_audit(
    expression: &str,
    steps: &[MathDerivationStep],
    trajectory: &[MathPhaseStep],
    result: &EvalValue,
) -> Value {
    let step_diagnostics = steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "code": format!("MATH_STEP_{}", step.step),
                "message": format!("{} => {}", step.expression, step.result_text),
                "severity": "info",
                "rule": step.rule,
            })
        })
        .collect::<Vec<_>>();
    let derivation_steps = steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "step": step.step,
                "rule": step.rule,
                "expression": step.expression,
                "result": step.result,
                "result_text": step.result_text,
                "geometry": step.geometry,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "envelope_id": format!("math_success_{}_{}", unix_time_secs(), steps.len()),
        "source_text": expression,
        "source_kind": "MathExpression",
        "intent": {
            "intent_id": "math_eval",
            "primary_goal": "EvaluateNumeric",
            "requested_output_mode": "TextAndStructured"
        },
        "semantic_jobs": [
            {
                "job_id": "semantic_route_success",
                "requested_operation": "PreserveSuccessContext",
                "status": "succeeded",
                "input_summary": expression,
                "trace": ["expression parsed and routed deterministically"]
            }
        ],
        "math_jobs": [
            {
                "job_id": "math_user",
                "requested_operation": "Evaluate",
                "status": "succeeded",
                "normalized_expression": expression,
                "step_count": steps.len(),
                "result": result.value.clone().into_math_value(),
                "derivation_steps": derivation_steps,
                "diagnostics": step_diagnostics,
            }
        ],
        "routing_trace": [
            {
                "stage": "parse",
                "decision": "parsed_expression",
                "rationale": format!("{} derivation step(s)", steps.len())
            },
            {
                "stage": "evaluate",
                "decision": "evaluated_expression",
                "rationale": result.text
            },
            {
                "stage": "synthesize",
                "decision": "bridge_audit_emitted",
                "rationale": format!("{} trajectory event(s) attached", trajectory.len())
            }
        ],
        "job_influence_audit": [
            {
                "job_id": "math_user",
                "job_kind": "math",
                "used_in_final_answer": true,
                "explanation": "deterministic evaluator produced the structured result"
            }
        ],
        "diagnostics": step_diagnostics,
        "final_outcome": {
            "status": "succeeded",
            "responder_text": result.text,
            "machine_summary": {
                "final_value": result.value.clone().into_math_value(),
                "confidence": 1.0,
                "contradiction_count": 0
            }
        }
    })
}

fn unix_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn geometric_real_binary_result(left_text: &str, right_text: &str, op: &BinaryOp) -> Result<ComplexValue, MathError> {
    let left = decimal_rational_from_text(left_text)
        .ok_or_else(|| MathError::Domain("geometric mode requires decimal-compatible real operands".to_string()))?;
    let right = decimal_rational_from_text(right_text)
        .ok_or_else(|| MathError::Domain("geometric mode requires decimal-compatible real operands".to_string()))?;

    let (ln, ld) = left;
    let (rn, rd) = right;
    let (numerator, denominator) = match op {
        BinaryOp::Add => (ln * rd + rn * ld, ld * rd),
        BinaryOp::Subtract => (ln * rd - rn * ld, ld * rd),
        BinaryOp::Multiply => (ln * rn, ld * rd),
        BinaryOp::Divide => {
            if rn == 0 {
                return Err(MathError::Domain("division by zero".to_string()));
            }
            (ln * rd, ld * rn)
        }
        BinaryOp::Power => return Err(MathError::Domain("geometric exact power handling is not implemented for this operand pair".to_string())),
    };

    if denominator == 0 {
        return Err(MathError::Domain("geometric decimal scaling produced a zero denominator".to_string()));
    }

    let reduced = reduce_rational(numerator, denominator);
    Ok(ComplexValue::new(reduced.0 as f64 / reduced.1 as f64, 0.0))
}

fn decimal_rational_from_text(text: &str) -> Option<(i128, i128)> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.contains('e') || trimmed.contains('E') || trimmed.contains('i') {
        return None;
    }

    let mut sign = 1_i128;
    let mut body = trimmed;
    if let Some(rest) = body.strip_prefix('-') {
        sign = -1;
        body = rest;
    } else if let Some(rest) = body.strip_prefix('+') {
        body = rest;
    }

    let (whole, fraction) = match body.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (body, ""),
    };

    if whole.is_empty() && fraction.is_empty() {
        return None;
    }

    let whole_value = if whole.is_empty() { 0_i128 } else { whole.parse::<i128>().ok()? };
    let fraction_value = if fraction.is_empty() { 0_i128 } else { fraction.parse::<i128>().ok()? };
    let scale = 10_i128.pow(fraction.len() as u32);
    let numerator = whole_value.checked_mul(scale)?.checked_add(fraction_value)? * sign;
    Some(reduce_rational(numerator, scale))
}

fn reduce_rational(numerator: i128, denominator: i128) -> (i128, i128) {
    let mut numerator = numerator;
    let mut denominator = denominator;
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    let divisor = gcd_i128(numerator.abs(), denominator.abs()).max(1);
    (numerator / divisor, denominator / divisor)
}

fn gcd_i128(mut left: i128, mut right: i128) -> i128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.abs()
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < 1e-12 {
        format!("{}", value.round() as i64)
    } else {
        let mut text = format!("{:.12}", value);
        while text.contains('.') && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        text
    }
}

fn format_imaginary_component(value: f64) -> String {
    if (value.abs() - 1.0).abs() < 1e-12 {
        if value.is_sign_negative() {
            "-i".to_string()
        } else {
            "i".to_string()
        }
    } else {
        format!("{}i", format_number(value))
    }
}

fn complex_to_text(value: ComplexValue) -> String {
    if value.is_real() {
        return format_number(value.re);
    }
    if value.re.abs() < 1e-12 {
        return format_imaginary_component(value.im);
    }
    let imag_abs = format_imaginary_component(value.im.abs());
    if value.im.is_sign_negative() {
        format!("{}-{}", format_number(value.re), imag_abs.trim_start_matches('-'))
    } else {
        format!("{}+{}", format_number(value.re), imag_abs)
    }
}

fn matrix_to_text(matrix: &ComplexMatrix) -> String {
    format!(
        "[{}]",
        matrix
            .rows
            .iter()
            .map(|row| format!("[{}]", row.iter().copied().map(complex_to_text).collect::<Vec<_>>().join(", ")))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn eval_data_to_text(value: &EvalData) -> String {
    match value {
        EvalData::Scalar(scalar) => complex_to_text(*scalar),
        EvalData::Matrix(matrix) => matrix_to_text(matrix),
    }
}

fn c_exp(value: ComplexValue) -> ComplexValue {
    let scale = value.re.exp();
    ComplexValue::new(scale * value.im.cos(), scale * value.im.sin())
}

fn c_log(value: ComplexValue) -> Result<ComplexValue, MathError> {
    if value.is_zero() {
        return Err(MathError::Domain("log undefined at zero".to_string()));
    }
    Ok(ComplexValue::new(value.abs().ln(), value.arg()))
}

fn c_beta(a: ComplexValue, b: ComplexValue) -> Result<ComplexValue, MathError> {
    Ok(c_gamma(a)? * c_gamma(b)? / c_gamma(a + b)?)
}

fn c_erf(z: ComplexValue) -> Result<ComplexValue, MathError> {
    let mut term = z;
    let mut sum = term;
    let zz = z * z;
    for n in 0..120usize {
        let numerator = -zz * ComplexValue::new((2 * n + 1) as f64, 0.0);
        let denominator = ComplexValue::new(((n + 1) * (2 * n + 3)) as f64, 0.0);
        term = term * (numerator / denominator);
        sum = sum + term;
        if term.abs() < 1e-14 {
            break;
        }
    }
    Ok(sum * ComplexValue::new(2.0 / std::f64::consts::PI.sqrt(), 0.0))
}

fn real_only_arg(name: &str, value: ComplexValue) -> Result<f64, MathError> {
    if !value.is_real() {
        return Err(MathError::Domain(format!("{} currently requires a real argument", name)));
    }
    Ok(value.re)
}

fn simpson_integrate_real<F>(a: f64, b: f64, steps: usize, mut f: F) -> f64
where
    F: FnMut(f64) -> f64,
{
    let mut n = steps.max(2);
    if n % 2 == 1 {
        n += 1;
    }
    let h = (b - a) / n as f64;
    let mut acc = f(a) + f(b);
    for i in 1..n {
        let x = a + i as f64 * h;
        let weight = if i % 2 == 0 { 2.0 } else { 4.0 };
        acc += weight * f(x);
    }
    acc * h / 3.0
}

fn c_si(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let x = real_only_arg("si", value)?;
    let steps = (x.abs() * 600.0).ceil() as usize + 600;
    let integral = simpson_integrate_real(0.0, x, steps, |t| {
        if t.abs() < 1e-12 {
            1.0
        } else {
            t.sin() / t
        }
    });
    Ok(ComplexValue::new(integral, 0.0))
}

fn c_ci(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let x = real_only_arg("ci", value)?;
    if x <= 0.0 {
        return Err(MathError::Domain("ci currently requires x > 0".to_string()));
    }
    let steps = (x * 600.0).ceil() as usize + 600;
    let integral = simpson_integrate_real(0.0, x, steps, |t| {
        if t.abs() < 1e-12 {
            0.0
        } else {
            (t.cos() - 1.0) / t
        }
    });
    let euler_gamma = 0.577_215_664_901_532_9_f64;
    Ok(ComplexValue::new(euler_gamma + x.ln() + integral, 0.0))
}

fn c_fresnel_c(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let x = real_only_arg("fresnelc", value)?;
    let steps = (x.abs() * 900.0).ceil() as usize + 900;
    let integral = simpson_integrate_real(0.0, x, steps, |t| {
        (std::f64::consts::PI * t * t / 2.0).cos()
    });
    Ok(ComplexValue::new(integral, 0.0))
}

fn c_fresnel_s(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let x = real_only_arg("fresnels", value)?;
    let steps = (x.abs() * 900.0).ceil() as usize + 900;
    let integral = simpson_integrate_real(0.0, x, steps, |t| {
        (std::f64::consts::PI * t * t / 2.0).sin()
    });
    Ok(ComplexValue::new(integral, 0.0))
}

fn c_ei(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let x = real_only_arg("ei", value)?;
    if x.abs() < 1e-12 {
        return Err(MathError::Domain("ei is singular at 0".to_string()));
    }
    let euler_gamma = 0.577_215_664_901_532_9_f64;
    let mut factorial = 1.0;
    let mut x_pow = x;
    let mut sum = 0.0;
    for k in 1..=120usize {
        factorial *= k as f64;
        let term = x_pow / (k as f64 * factorial);
        sum += term;
        if term.abs() < 1e-15 {
            break;
        }
        x_pow *= x;
    }
    Ok(ComplexValue::new(euler_gamma + x.abs().ln() + sum, 0.0))
}

fn c_li(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let x = real_only_arg("li", value)?;
    if x <= 0.0 {
        return Err(MathError::Domain("li currently requires x > 0".to_string()));
    }
    if (x - 1.0).abs() < 1e-12 {
        return Err(MathError::Domain("li is singular at x = 1".to_string()));
    }
    c_ei(ComplexValue::new(x.ln(), 0.0))
}

fn c_sinc(value: ComplexValue) -> ComplexValue {
    if value.is_zero() {
        ComplexValue::new(1.0, 0.0)
    } else {
        c_sin(value) / value
    }
}

fn c_bessel_k_real_order(order: f64, z: ComplexValue) -> Result<ComplexValue, MathError> {
    let near_integer_eps = 1e-6;
    let nu_shift = if (order - order.round()).abs() < near_integer_eps {
        order + near_integer_eps
    } else {
        order
    };
    let i_minus = c_bessel_i_real_order(-nu_shift, z)?;
    let i_plus = c_bessel_i_real_order(nu_shift, z)?;
    let numerator = i_minus - i_plus;
    let denominator = ComplexValue::new((std::f64::consts::PI * nu_shift).sin(), 0.0);
    if denominator.abs() < 1e-14 {
        return Err(MathError::Domain("besselk continuation denominator underflow".to_string()));
    }
    Ok(ComplexValue::new(std::f64::consts::PI / 2.0, 0.0) * (numerator / denominator))
}

fn c_ai(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let x = real_only_arg("ai", value)?;
    if x < 0.0 {
        return Err(MathError::Domain("ai currently supports x >= 0".to_string()));
    }
    let t = 2.0 * x.powf(1.5) / 3.0;
    let prefactor = (x / 3.0).sqrt() / std::f64::consts::PI;
    Ok(c_bessel_k_real_order(1.0 / 3.0, ComplexValue::new(t, 0.0))? * ComplexValue::new(prefactor, 0.0))
}

fn c_bi(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let x = real_only_arg("bi", value)?;
    if x < 0.0 {
        return Err(MathError::Domain("bi currently supports x >= 0".to_string()));
    }
    let t = 2.0 * x.powf(1.5) / 3.0;
    let prefactor = (x / 3.0).sqrt();
    let i_neg = c_bessel_i_real_order(-1.0 / 3.0, ComplexValue::new(t, 0.0))?;
    let i_pos = c_bessel_i_real_order(1.0 / 3.0, ComplexValue::new(t, 0.0))?;
    Ok((i_neg + i_pos) * ComplexValue::new(prefactor, 0.0))
}

fn c_theta4(z: ComplexValue, q: ComplexValue) -> Result<ComplexValue, MathError> {
    if q.abs() >= 1.0 {
        return Err(MathError::Domain("theta4 currently requires |q| < 1".to_string()));
    }

    let mut sum = ComplexValue::new(1.0, 0.0);
    for n in 1..=300usize {
        let n_f = n as f64;
        let q_pow = c_pow(q, ComplexValue::new((n * n) as f64, 0.0))?;
        let cosine = c_cos(ComplexValue::new(2.0 * n_f, 0.0) * z);
        let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
        let term = ComplexValue::new(2.0 * sign, 0.0) * q_pow * cosine;
        sum = sum + term;
        if term.abs() < 1e-14 {
            break;
        }
    }
    Ok(sum)
}

fn c_pow(base: ComplexValue, exponent: ComplexValue) -> Result<ComplexValue, MathError> {
    if base.is_zero() {
        if exponent.im.abs() < 1e-12 && exponent.re > 0.0 {
            return Ok(ComplexValue::new(0.0, 0.0));
        }
        return Err(MathError::Domain("zero cannot be raised to this power".to_string()));
    }
    Ok(c_exp(exponent * c_log(base)?))
}

fn c_sqrt(value: ComplexValue) -> ComplexValue {
    let radius = value.abs();
    let real = ((radius + value.re) / 2.0).sqrt();
    let imaginary = ((radius - value.re) / 2.0).sqrt().copysign(value.im);
    ComplexValue::new(real, imaginary)
}

fn c_sin(value: ComplexValue) -> ComplexValue {
    ComplexValue::new(
        value.re.sin() * value.im.cosh(),
        value.re.cos() * value.im.sinh(),
    )
}

fn c_cos(value: ComplexValue) -> ComplexValue {
    ComplexValue::new(
        value.re.cos() * value.im.cosh(),
        -value.re.sin() * value.im.sinh(),
    )
}

fn c_gamma(value: ComplexValue) -> Result<ComplexValue, MathError> {
    if value.is_real() {
        let rounded = value.re.round();
        if value.re <= 0.0 && (value.re - rounded).abs() < 1e-12 {
            return Err(MathError::Domain("gamma undefined at non-positive integers".to_string()));
        }
    }

    const G: f64 = 7.0;
    const COEFFS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        0.000_009_984_369_578_019_572,
        0.000_000_150_563_273_514_931_16,
    ];

    if value.re < 0.5 {
        let pi = ComplexValue::new(std::f64::consts::PI, 0.0);
        let sin_pi_z = c_sin(pi * value);
        if sin_pi_z.is_zero() {
            return Err(MathError::Domain("gamma undefined at reflection pole".to_string()));
        }
        let reflected = c_gamma(ComplexValue::new(1.0, 0.0) - value)?;
        return Ok(pi / (sin_pi_z * reflected));
    }

    let z = value - ComplexValue::new(1.0, 0.0);
    let mut x = ComplexValue::new(COEFFS[0], 0.0);
    for (index, coeff) in COEFFS.iter().enumerate().skip(1) {
        let denom = z + ComplexValue::new(index as f64, 0.0);
        if denom.is_zero() {
            return Err(MathError::Domain("gamma denominator underflow".to_string()));
        }
        x = x + (ComplexValue::new(*coeff, 0.0) / denom);
    }

    let t = z + ComplexValue::new(G + 0.5, 0.0);
    let sqrt_two_pi = ComplexValue::new((2.0 * std::f64::consts::PI).sqrt(), 0.0);
    Ok(sqrt_two_pi * c_pow(t, z + ComplexValue::new(0.5, 0.0))? * c_exp(-t) * x)
}

fn c_lambertw(value: ComplexValue) -> Result<ComplexValue, MathError> {
    if value.is_zero() {
        return Ok(ComplexValue::new(0.0, 0.0));
    }

    let mut w = c_log(value + ComplexValue::new(1.0, 0.0))?;
    for _ in 0..60 {
        let ew = c_exp(w);
        let wew = w * ew;
        let f = wew - value;
        let wp1 = w + ComplexValue::new(1.0, 0.0);
        if wp1.abs() < 1e-15 {
            return Err(MathError::Domain("lambertw iteration singular near branch point".to_string()));
        }
        let denom_left = ew * wp1;
        let denom_right = ((w + ComplexValue::new(2.0, 0.0)) * f)
            / (ComplexValue::new(2.0, 0.0) * wp1);
        let denom = denom_left - denom_right;
        if denom.abs() < 1e-18 {
            return Err(MathError::Domain("lambertw iteration denominator underflow".to_string()));
        }
        let delta = f / denom;
        let next = w - delta;
        if delta.abs() < 1e-13 {
            return Ok(next);
        }
        w = next;
    }
    Ok(w)
}

fn c_zeta_positive_half_plane(value: ComplexValue) -> Result<ComplexValue, MathError> {
    let mut eta_sum = ComplexValue::new(0.0, 0.0);
    for n in 1..=20_000usize {
        let ln_n = (n as f64).ln();
        let term = c_exp(ComplexValue::new(-value.re * ln_n, -value.im * ln_n));
        let signed = if n % 2 == 1 { term } else { -term };
        eta_sum = eta_sum + signed;
        if signed.abs() < 1e-14 {
            break;
        }
    }

    let denom = ComplexValue::new(1.0, 0.0)
        - c_exp((ComplexValue::new(1.0, 0.0) - value) * ComplexValue::new(2.0_f64.ln(), 0.0));
    if denom.abs() < 1e-12 {
        return Err(MathError::Domain("zeta undefined at singular denominator".to_string()));
    }
    Ok(eta_sum / denom)
}

fn c_zeta(value: ComplexValue) -> Result<ComplexValue, MathError> {
    if (value.re - 1.0).abs() < 1e-12 && value.im.abs() < 1e-12 {
        return Err(MathError::Domain("zeta has a pole at s = 1".to_string()));
    }

    if value.re > 0.0 {
        return c_zeta_positive_half_plane(value);
    }

    let one = ComplexValue::new(1.0, 0.0);
    let two = ComplexValue::new(2.0, 0.0);
    let pi = ComplexValue::new(std::f64::consts::PI, 0.0);
    let one_minus_s = one - value;
    let zeta_one_minus_s = c_zeta_positive_half_plane(one_minus_s)?;
    let two_pow_s = c_exp(value * ComplexValue::new(2.0_f64.ln(), 0.0));
    let pi_pow_s_minus_one = c_pow(pi, value - one)?;
    let sin_term = c_sin((pi * value) / two);
    let gamma_term = c_gamma(one_minus_s)?;
    Ok(two_pow_s * pi_pow_s_minus_one * sin_term * gamma_term * zeta_one_minus_s)
}

fn c_bessel_j(order: usize, z: ComplexValue) -> Result<ComplexValue, MathError> {
    if z.is_zero() {
        return Ok(if order == 0 {
            ComplexValue::new(1.0, 0.0)
        } else {
            ComplexValue::new(0.0, 0.0)
        });
    }

    let factorial = (1..=order).fold(1.0_f64, |acc, value| acc * value as f64);
    let z_over_2 = z / ComplexValue::new(2.0, 0.0);
    let z_over_2_sq = z_over_2 * z_over_2;
    let mut term = c_pow(z_over_2, ComplexValue::new(order as f64, 0.0))?
        / ComplexValue::new(factorial, 0.0);
    let mut sum = term;

    for m in 0..80 {
        let denominator = ((m + 1) * (m + order + 1)) as f64;
        let factor = -z_over_2_sq / ComplexValue::new(denominator, 0.0);
        term = term * factor;
        sum = sum + term;
        if term.abs() < 1e-15 {
            break;
        }
    }

    Ok(sum)
}

fn c_spherical_bessel_j(order: usize, z: ComplexValue) -> Result<ComplexValue, MathError> {
    if order == 0 {
        if z.is_zero() {
            return Ok(ComplexValue::new(1.0, 0.0));
        }
        return Ok(c_sin(z) / z);
    }

    if order == 1 {
        if z.is_zero() {
            return Ok(ComplexValue::new(0.0, 0.0));
        }
        let z_squared = z * z;
        return Ok((c_sin(z) / z_squared) - (c_cos(z) / z));
    }

    if z.is_zero() {
        return Ok(ComplexValue::new(0.0, 0.0));
    }

    let mut jm1 = c_sin(z) / z;
    let mut current = (c_sin(z) / (z * z)) - (c_cos(z) / z);
    for n in 1..order {
        let coeff = ComplexValue::new((2 * n + 1) as f64, 0.0);
        let next = (coeff * current) / z - jm1;
        jm1 = current;
        current = next;
    }
    Ok(current)
}

fn c_bessel_i(order: usize, z: ComplexValue) -> Result<ComplexValue, MathError> {
    let z_over_2 = z / ComplexValue::new(2.0, 0.0);
    let z_over_2_sq = z_over_2 * z_over_2;
    let factorial = (1..=order).fold(1.0_f64, |acc, value| acc * value as f64);
    let mut term = c_pow(z_over_2, ComplexValue::new(order as f64, 0.0))?
        / ComplexValue::new(factorial, 0.0);
    let mut sum = term;

    for m in 0..80 {
        let denominator = ((m + 1) * (m + order + 1)) as f64;
        term = term * (z_over_2_sq / ComplexValue::new(denominator, 0.0));
        sum = sum + term;
        if term.abs() < 1e-15 {
            break;
        }
    }

    Ok(sum)
}

fn factorial(value: usize) -> f64 {
    (1..=value).fold(1.0, |acc, item| acc * item as f64)
}

fn c_bessel_j_real_order(order: f64, z: ComplexValue) -> Result<ComplexValue, MathError> {
    let z_over_2 = z / ComplexValue::new(2.0, 0.0);
    let mut sum = ComplexValue::new(0.0, 0.0);
    for m in 0..80usize {
        let exponent = ComplexValue::new(2.0 * m as f64 + order, 0.0);
        let numerator = c_pow(z_over_2, exponent)?;
        let denominator = factorial(m) * c_gamma(ComplexValue::new(m as f64 + order + 1.0, 0.0))?.re;
        if denominator.abs() < 1e-15 {
            return Err(MathError::Domain("besselj denominator underflow".to_string()));
        }
        let sign = if m % 2 == 0 { 1.0 } else { -1.0 };
        let term = numerator * ComplexValue::new(sign / denominator, 0.0);
        sum = sum + term;
        if term.abs() < 1e-14 {
            break;
        }
    }
    Ok(sum)
}

fn c_bessel_i_real_order(order: f64, z: ComplexValue) -> Result<ComplexValue, MathError> {
    let z_over_2 = z / ComplexValue::new(2.0, 0.0);
    let mut sum = ComplexValue::new(0.0, 0.0);
    for m in 0..80usize {
        let exponent = ComplexValue::new(2.0 * m as f64 + order, 0.0);
        let numerator = c_pow(z_over_2, exponent)?;
        let denominator = factorial(m) * c_gamma(ComplexValue::new(m as f64 + order + 1.0, 0.0))?.re;
        if denominator.abs() < 1e-15 {
            return Err(MathError::Domain("besseli denominator underflow".to_string()));
        }
        let term = numerator * ComplexValue::new(1.0 / denominator, 0.0);
        sum = sum + term;
        if term.abs() < 1e-14 {
            break;
        }
    }
    Ok(sum)
}

fn c_bessel_k(order: usize, z: ComplexValue) -> Result<ComplexValue, MathError> {
    let nu = order as f64;
    let near_integer_eps = 1e-6;
    let nu_shift = nu + near_integer_eps;
    let i_minus = c_bessel_i_real_order(-nu_shift, z)?;
    let i_plus = c_bessel_i_real_order(nu_shift, z)?;
    let numerator = i_minus - i_plus;
    let denominator = ComplexValue::new((std::f64::consts::PI * nu_shift).sin(), 0.0);
    if denominator.abs() < 1e-14 {
        return Err(MathError::Domain("besselk continuation denominator underflow".to_string()));
    }
    Ok(ComplexValue::new(std::f64::consts::PI / 2.0, 0.0) * (numerator / denominator))
}

fn c_bessel_y(order: usize, z: ComplexValue) -> Result<ComplexValue, MathError> {
    let nu = order as f64;
    let near_integer_eps = 1e-6;
    let nu_shift = nu + near_integer_eps;
    let j_minus = c_bessel_j_real_order(-nu_shift, z)?;
    let j_plus = c_bessel_j_real_order(nu_shift, z)?;
    let numerator = j_plus * ComplexValue::new((std::f64::consts::PI * nu_shift).cos(), 0.0) - j_minus;
    let denominator = ComplexValue::new((std::f64::consts::PI * nu_shift).sin(), 0.0);
    if denominator.abs() < 1e-14 {
        return Err(MathError::Domain("bessely continuation denominator underflow".to_string()));
    }
    Ok(numerator / denominator)
}

fn matrix_minor(matrix: &ComplexMatrix, excluded_row: usize, excluded_col: usize) -> Result<ComplexMatrix, MathError> {
    ComplexMatrix::new(
        matrix
            .rows
            .iter()
            .enumerate()
            .filter(|(row_index, _)| *row_index != excluded_row)
            .map(|(_, row)| {
                row.iter()
                    .enumerate()
                    .filter(|(column_index, _)| *column_index != excluded_col)
                    .map(|(_, value)| *value)
                    .collect::<Vec<_>>()
            })
            .collect(),
    )
}

fn matrix_determinant(matrix: &ComplexMatrix) -> Result<ComplexValue, MathError> {
    if !matrix.is_square() {
        return Err(MathError::Domain("det requires a square matrix".to_string()));
    }
    match matrix.row_count() {
        0 => Err(MathError::Domain("det requires a non-empty matrix".to_string())),
        1 => Ok(matrix.rows[0][0]),
        2 => Ok(matrix.rows[0][0] * matrix.rows[1][1] - matrix.rows[0][1] * matrix.rows[1][0]),
        _ => {
            let mut determinant = ComplexValue::new(0.0, 0.0);
            for (column_index, value) in matrix.rows[0].iter().enumerate() {
                let sign = if column_index % 2 == 0 { 1.0 } else { -1.0 };
                determinant = determinant
                    + ComplexValue::new(sign, 0.0) * *value * matrix_determinant(&matrix_minor(matrix, 0, column_index)?)?;
            }
            Ok(determinant)
        }
    }
}

fn matrix_inverse(matrix: &ComplexMatrix) -> Result<ComplexMatrix, MathError> {
    if !matrix.is_square() {
        return Err(MathError::Domain("inverse requires a square matrix".to_string()));
    }
    let n = matrix.row_count();
    let mut augmented = matrix
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let mut values = row.clone();
            values.extend((0..n).map(|column_index| {
                if row_index == column_index {
                    ComplexValue::new(1.0, 0.0)
                } else {
                    ComplexValue::new(0.0, 0.0)
                }
            }));
            values
        })
        .collect::<Vec<_>>();

    for pivot_index in 0..n {
        let pivot_row = (pivot_index..n)
            .find(|row_index| !augmented[*row_index][pivot_index].is_zero())
            .ok_or_else(|| MathError::Domain("matrix is singular and cannot be inverted".to_string()))?;
        if pivot_row != pivot_index {
            augmented.swap(pivot_row, pivot_index);
        }

        let pivot = augmented[pivot_index][pivot_index];
        for column_index in 0..2 * n {
            augmented[pivot_index][column_index] = augmented[pivot_index][column_index] / pivot;
        }

        for row_index in 0..n {
            if row_index == pivot_index {
                continue;
            }
            let factor = augmented[row_index][pivot_index];
            if factor.is_zero() {
                continue;
            }
            for column_index in 0..2 * n {
                augmented[row_index][column_index] =
                    augmented[row_index][column_index] - factor * augmented[pivot_index][column_index];
            }
        }
    }

    ComplexMatrix::new(
        augmented
            .into_iter()
            .map(|row| row.into_iter().skip(n).collect::<Vec<_>>())
            .collect(),
    )
}

fn matrix_hafnian(matrix: &ComplexMatrix) -> Result<ComplexValue, MathError> {
    if !matrix.is_square() {
        return Err(MathError::Domain("hafnian requires a square matrix".to_string()));
    }

    let dimension = matrix.row_count();
    if dimension % 2 != 0 {
        return Err(MathError::Domain("hafnian requires an even-dimension matrix".to_string()));
    }
    if dimension > HAFNIAN_EXACT_MAX_DIMENSION {
        return Err(MathError::Domain(format!(
            "exact hafnian is currently capped at {}x{} in this slice; use symbolic_identity mode for statement capture or provide a smaller concrete matrix",
            HAFNIAN_EXACT_MAX_DIMENSION,
            HAFNIAN_EXACT_MAX_DIMENSION,
        )));
    }

    let indices: Vec<usize> = (0..dimension).collect();
    Ok(matrix_hafnian_recursive(matrix, &indices))
}

fn matrix_hafnian_recursive(matrix: &ComplexMatrix, indices: &[usize]) -> ComplexValue {
    if indices.is_empty() {
        return ComplexValue::new(1.0, 0.0);
    }

    let first = indices[0];
    let mut total = ComplexValue::new(0.0, 0.0);
    for pair_position in 1..indices.len() {
        let pair_index = indices[pair_position];
        let mut remaining = Vec::with_capacity(indices.len().saturating_sub(2));
        for (position, index) in indices.iter().enumerate().skip(1) {
            if position != pair_position {
                remaining.push(*index);
            }
        }

        total = total + matrix.rows[first][pair_index] * matrix_hafnian_recursive(matrix, &remaining);
    }

    total
}

fn matrix_coefficients(matrix: &ComplexMatrix, name: &str) -> Result<Vec<ComplexValue>, MathError> {
    if matrix.row_count() == 1 {
        return Ok(matrix.rows[0].clone());
    }
    if matrix.column_count() == 1 {
        return Ok(matrix.rows.iter().map(|row| row[0]).collect());
    }
    Err(MathError::Domain(format!("{} expects a row or column coefficient matrix", name)))
}

fn polynomial_eval(coefficients: &[ComplexValue], s: ComplexValue) -> ComplexValue {
    coefficients
        .iter()
        .copied()
        .fold(ComplexValue::new(0.0, 0.0), |acc, coefficient| acc * s + coefficient)
}

fn transfer_function_eval(
    numerator: &ComplexMatrix,
    denominator: &ComplexMatrix,
    s: ComplexValue,
) -> Result<ComplexValue, MathError> {
    let numerator_coeffs = matrix_coefficients(numerator, "tf")?;
    let denominator_coeffs = matrix_coefficients(denominator, "tf")?;
    let denominator_value = polynomial_eval(&denominator_coeffs, s);
    if denominator_value.is_zero() {
        return Err(MathError::Domain("tf denominator evaluates to zero".to_string()));
    }
    Ok(polynomial_eval(&numerator_coeffs, s) / denominator_value)
}

fn c_polylog(s: ComplexValue, z: ComplexValue) -> Result<ComplexValue, MathError> {
    if s.re <= 0.0 {
        return Err(MathError::Domain("polylog currently supports Re(s) > 0".to_string()));
    }
    if z.is_zero() {
        return Ok(ComplexValue::new(0.0, 0.0));
    }

    if z.abs() < 0.9 {
        let mut sum = ComplexValue::new(0.0, 0.0);
        let mut z_power = z;
        for n in 1..=20_000usize {
            let ln_n = (n as f64).ln();
            let n_pow_neg_s = c_exp(ComplexValue::new(-s.re * ln_n, -s.im * ln_n));
            let current = z_power * n_pow_neg_s;
            sum = sum + current;
            if current.abs() < 1e-14 {
                break;
            }
            z_power = z_power * z;
        }
        return Ok(sum);
    }

    let gamma_s = c_gamma(s)?;
    if gamma_s.abs() < 1e-15 {
        return Err(MathError::Domain("polylog gamma(s) underflow".to_string()));
    }

    let epsilon = 1e-8;
    let upper = 40.0;
    let steps = 1200usize;
    let h = (upper - epsilon) / steps as f64;
    let one = ComplexValue::new(1.0, 0.0);
    let s_minus_one = s - one;
    let mut accumulator = ComplexValue::new(0.0, 0.0);

    for index in 0..=steps {
        let t = epsilon + index as f64 * h;
        let t_power = c_exp(s_minus_one * ComplexValue::new(t.ln(), 0.0));
        let denominator = ComplexValue::new(t.exp(), 0.0) - z;
        if denominator.abs() < 1e-12 {
            return Err(MathError::Domain(
                "polylog integral encountered pole on contour".to_string(),
            ));
        }
        let integrand = t_power * (z / denominator);
        let weight = if index == 0 || index == steps {
            1.0
        } else if index % 2 == 0 {
            2.0
        } else {
            4.0
        };
        accumulator = accumulator + integrand * ComplexValue::new(weight, 0.0);
    }

    Ok((accumulator * ComplexValue::new(h / 3.0, 0.0)) / gamma_s)
}

fn c_gammainc(a: ComplexValue, z: ComplexValue) -> Result<ComplexValue, MathError> {
    if a.re <= 0.0 {
        return Err(MathError::Domain("gammainc currently supports Re(a) > 0".to_string()));
    }
    if a.is_zero() {
        return Err(MathError::Domain("gammainc undefined for a = 0".to_string()));
    }

    let mut series = ComplexValue::new(1.0, 0.0) / a;
    let mut term = series;
    for n in 0..500usize {
        let denominator = a + ComplexValue::new((n + 1) as f64, 0.0);
        term = (term * z) / denominator;
        series = series + term;
        if term.abs() < 1e-14 {
            break;
        }
    }

    Ok(c_pow(z, a)? * c_exp(-z) * series)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        AngleUnit, AppConfig, DEFAULT_ENGINE_MODE, MathError, MathMode, MathOptions, MathValue,
        RWIF_SCHEMA_VERSION, SolverRequest, evaluate_math_expression, parse_angle_unit, parse_math_mode,
        platform_catalog, solve_linear_equation, wrap_pi,
    };
    use serde_json::Value;

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

    #[test]
    fn platform_catalog_exposes_foundation_modules() {
        let catalog = platform_catalog();

        assert_eq!(catalog.platform_id, "digitalcrystal");
        assert!(catalog.modules.iter().any(|module| module.module_id == "special-functions"));
        assert!(catalog.modules.iter().any(|module| module.module_id == "rwif-explorer"));
    }

    #[test]
    fn math_mode_and_angle_unit_follow_predecessor_contract() {
        assert_eq!(parse_math_mode(Some("geometric")).unwrap(), MathMode::Geometric);
        assert_eq!(
            parse_math_mode(Some("symbolic_identity")).unwrap(),
            MathMode::SymbolicIdentity
        );
        assert_eq!(parse_angle_unit(Some("degrees")).unwrap(), AngleUnit::Degrees);
        assert!(parse_math_mode(Some("bad")).is_err());
        assert!(parse_angle_unit(Some("bad")).is_err());
    }

    #[test]
    fn math_expression_evaluates_with_trace_and_rwif_export() {
        let response = evaluate_math_expression(
            "2*(3+4)^2",
            MathOptions::default(),
        )
        .expect("expression should evaluate");

        assert_eq!(response.object, "csif.math.result");
        assert_eq!(response.result, MathValue::Real(98.0));
        assert!(!response.derivation_trace.is_empty());
        assert_eq!(response.rwif_export.rwif_schema_version.as_deref(), Some(RWIF_SCHEMA_VERSION));
        assert_eq!(response.phase_signature.crystal_state, "YES");
        assert!(!response.path_signature.is_empty());
        assert!(!response.endpoint_signature.is_empty());
        assert!(response.bridge_audit.get("math_jobs").is_some());
    }

    #[test]
    fn t1_conformance_path_signature_distinguishes_constraint_paths() {
        let lhs = evaluate_math_expression("(2 + 3) + 4", MathOptions::default())
            .expect("left expression should evaluate");
        let rhs = evaluate_math_expression("2 + (3 + 4)", MathOptions::default())
            .expect("right expression should evaluate");

        assert_ne!(lhs.path_signature, rhs.path_signature);
    }

    #[test]
    fn t2_conformance_signatures_are_stable_across_repeated_runs() {
        let expression = "(1 * 3) + (2 * 3)";
        let mut path_signatures = HashSet::new();
        let mut endpoint_signatures = HashSet::new();

        for _ in 0..5 {
            let payload = evaluate_math_expression(expression, MathOptions::default())
                .expect("expression should evaluate");
            path_signatures.insert(payload.path_signature);
            endpoint_signatures.insert(payload.endpoint_signature);
        }

        assert_eq!(path_signatures.len(), 1);
        assert_eq!(endpoint_signatures.len(), 1);
    }

    #[test]
    fn t3_conformance_witness_exists_for_path_endpoint_decoupling() {
        let lhs = evaluate_math_expression("(2 * 3) * 4", MathOptions::default())
            .expect("left expression should evaluate");
        let rhs = evaluate_math_expression("2 * (3 * 4)", MathOptions::default())
            .expect("right expression should evaluate");

        assert_eq!(lhs.result, rhs.result);
        assert_ne!(lhs.path_signature, rhs.path_signature);
        assert_ne!(lhs.endpoint_signature, rhs.endpoint_signature);
    }

    #[test]
    fn math_expression_uses_degree_mode_for_trig() {
        let response = evaluate_math_expression(
            "sin(30)+cos(60)",
            MathOptions {
                mode: MathMode::Geometric,
                angle_unit: AngleUnit::Degrees,
            },
        )
        .expect("expression should evaluate");

        match response.result {
            MathValue::Real(value) => assert!((value - 1.0).abs() < 1e-9),
            _ => panic!("expected real result"),
        }
    }

    #[test]
    fn math_expression_respects_power_before_unary_minus() {
        let response = evaluate_math_expression("-2^2", MathOptions::default())
            .expect("precedence expression should evaluate");

        match response.result {
            MathValue::Real(value) => assert!((value + 4.0).abs() < 1e-12),
            MathValue::Complex(value) => {
                assert!((value.re + 4.0).abs() < 1e-12);
                assert!(value.im.abs() < 1e-12);
            }
            _ => panic!("expected scalar result"),
        }
    }

    #[test]
    fn math_expression_uses_geometric_decimal_scaling_for_basic_addition() {
        let response = evaluate_math_expression(
            "0.1+0.2",
            MathOptions {
                mode: MathMode::Geometric,
                angle_unit: AngleUnit::Radians,
            },
        )
        .expect("geometric addition should evaluate");

        match response.result {
            MathValue::Real(value) => assert!((value - 0.3).abs() < 1e-12),
            MathValue::Complex(value) => {
                assert!((value.re - 0.3).abs() < 1e-12);
                assert!(value.im.abs() < 1e-12);
            }
            _ => panic!("expected scalar result"),
        }
        assert!(response.result_latex.contains("0.3"));
    }

    #[test]
    fn math_expression_supports_latex_gaussian_integral_identity() {
        let response = evaluate_math_expression(
            "\\int_{-\\infty}^{\\infty} e^{-x^2} \\, dx = \\sqrt{\\pi}",
            MathOptions::default(),
        )
        .expect("latex gaussian identity should evaluate");

        match response.result {
            MathValue::Real(value) => assert!(value.abs() < 1e-4),
            MathValue::Complex(value) => {
                assert!(value.re.abs() < 1e-4);
                assert!(value.im.abs() < 1e-9);
            }
            _ => panic!("expected scalar result"),
        }
        assert!(response.normalized_expression.contains("integral("));
    }

    #[test]
    fn math_expression_supports_latex_continued_fraction_identity() {
        let response = evaluate_math_expression(
            "\\sqrt{2} = 1 + \\cfrac{1}{2 + \\cfrac{1}{2 + \\cfrac{1}{2 + \\ddots}}}",
            MathOptions::default(),
        )
        .expect("latex continued fraction identity should evaluate");

        match response.result {
            MathValue::Real(value) => assert!(value.abs() < 2e-2),
            MathValue::Complex(value) => {
                assert!(value.re.abs() < 2e-2);
                assert!(value.im.abs() < 1e-9);
            }
            _ => panic!("expected scalar result"),
        }
        assert!(response.normalized_expression.contains("sqrt(2)"));
    }

    #[test]
    fn math_expression_reports_symbolic_hafnian_identity_as_non_executable() {
        let error = evaluate_math_expression(
            "\\operatorname{Haf}(A) = \\sum_{\\substack{\\sigma \\in S_{2n} \\\\ \\sigma(1)<\\cdots<\\sigma(n) \\\\ \\sigma(i)<\\sigma(i+n)}} \\prod_{i=1}^{n} a_{\\sigma(i),\\sigma(i+n)}",
            MathOptions::default(),
        )
        .expect_err("symbolic hafnian identity should not execute directly");

        match error {
            MathError::Parse(message) => {
                assert!(message.contains("switch mode to symbolic_identity"));
            }
            _ => panic!("expected parse error"),
        }
    }

    #[test]
    fn math_expression_rejects_exact_hafnian_above_dimension_cap() {
        let row = (0..18)
            .map(|column| if column == 0 { "0".to_string() } else { "1".to_string() })
            .collect::<Vec<_>>()
            .join(",");
        let matrix_literal = format!("[{}]", vec![format!("[{}]", row); 18].join(","));
        let expression = format!("hafnian({})", matrix_literal);

        let error = evaluate_math_expression(&expression, MathOptions::default())
            .expect_err("18x18 exact hafnian should be rejected");
        match error {
            MathError::Domain(message) => {
                assert!(message.contains("exact hafnian is currently capped at 16x16"));
            }
            _ => panic!("expected domain error"),
        }
    }

    #[test]
    fn math_expression_symbolic_identity_mode_stores_statement_non_numerically() {
        let response = evaluate_math_expression(
            "\\operatorname{Haf}(A) = \\sum_{\\substack{\\sigma \\in S_{2n}}} \\prod_{i=1}^{n} a_{\\sigma(i),\\sigma(i+n)}",
            MathOptions {
                mode: MathMode::SymbolicIdentity,
                angle_unit: AngleUnit::Radians,
            },
        )
        .expect("symbolic identity mode should store statement");

        assert_eq!(response.mode, "symbolic_identity");
        assert_eq!(response.phase_signature.crystal_state, "SYMBOLIC");
        match response.result {
            MathValue::Statement(statement) => {
                assert!(statement.trusted);
                assert!(!statement.executable);
                assert!(statement.statement.contains("\\operatorname{Haf}"));
            }
            _ => panic!("expected symbolic statement result"),
        }
    }

    #[test]
    fn math_expression_supports_complex_literals_and_functions() {
        let response = evaluate_math_expression(
            "conj(2+3i) + arg(1+i)",
            MathOptions::default(),
        )
        .expect("expression should evaluate");

        match response.result {
            MathValue::Complex(value) => {
                assert!((value.re - (2.0 + std::f64::consts::PI / 4.0)).abs() < 1e-9);
                assert!((value.im + 3.0).abs() < 1e-9);
            }
            _ => panic!("expected complex result"),
        }
    }

    #[test]
    fn math_expression_supports_complex_exponential_identity() {
        let response = evaluate_math_expression(
            "exp(i*pi) + 1",
            MathOptions::default(),
        )
        .expect("expression should evaluate");

        match response.result {
            MathValue::Real(value) => assert!(value.abs() < 1e-9),
            _ => panic!("expected real result"),
        }
    }

    #[test]
    fn math_expression_supports_log_gamma_lambertw_and_zeta() {
        let response = evaluate_math_expression(
            "ln(e) + log(100) + gamma(5) + lambertw(1) + zeta(2)",
            MathOptions::default(),
        )
        .expect("special functions should evaluate");

        match response.result {
            MathValue::Real(value) => {
                let expected = 1.0
                    + 2.0
                    + 24.0
                    + 0.567_143_290_409_783_8
                    + (std::f64::consts::PI * std::f64::consts::PI / 6.0);
                assert!((value - expected).abs() < 1e-6);
            }
            _ => panic!("expected real result"),
        }
    }

    #[test]
    fn math_expression_supports_multi_argument_special_functions() {
        let response = evaluate_math_expression(
            "log(8, 2) + polylog(1, 0.5) + gammainc(1, 2) + j0(0) + j_sph(0, 1.1)",
            MathOptions::default(),
        )
        .expect("multi-argument special functions should evaluate");

        match response.result {
            MathValue::Real(value) => {
                let expected = 3.0
                    + 2.0_f64.ln()
                    + (1.0 - (-2.0_f64).exp())
                    + 1.0
                    + (1.1_f64.sin() / 1.1);
                assert!((value - expected).abs() < 1e-6);
            }
            _ => panic!("expected real result"),
        }

        assert!(response
            .phase_signature
            .trajectory
            .iter()
            .any(|step| step.op == "polylogarithm"));
    }

    #[test]
    fn math_expression_supports_besselj_order_argument() {
        let response = evaluate_math_expression("besselj(1, 0)", MathOptions::default())
            .expect("besselj should evaluate");

        match response.result {
            MathValue::Real(value) => assert!(value.abs() < 1e-12),
            _ => panic!("expected real result"),
        }
    }

    #[test]
    fn math_expression_supports_matrix_literals_det_inverse_and_tf() {
        let det_response = evaluate_math_expression("det([[1,2],[3,4]])", MathOptions::default())
            .expect("det should evaluate");
        match det_response.result {
            MathValue::Real(value) => assert!((value + 2.0).abs() < 1e-12),
            _ => panic!("expected real determinant"),
        }

        let inverse_response = evaluate_math_expression("inverse([[1,2],[3,4]])", MathOptions::default())
            .expect("inverse should evaluate");
        match inverse_response.result {
            MathValue::Matrix(matrix) => {
                assert_eq!(matrix.rows.len(), 2);
                assert_eq!(matrix.rows[0].len(), 2);
            }
            _ => panic!("expected matrix inverse"),
        }

        let tf_response = evaluate_math_expression(
            "tf([[1, 0]], [[1, 1]], 1)",
            MathOptions::default(),
        )
        .expect("transfer function helper should evaluate");
        match tf_response.result {
            MathValue::Real(value) => assert!((value - 0.5).abs() < 1e-12),
            _ => panic!("expected real transfer function value"),
        }
        assert!(tf_response.phase_signature.cumulative_theta.abs() > 0.0);

        let hafnian_response = evaluate_math_expression(
            "hafnian([[0,1,1,1],[1,0,1,1],[1,1,0,1],[1,1,1,0]])",
            MathOptions::default(),
        )
        .expect("hafnian should evaluate");
        match hafnian_response.result {
            MathValue::Real(value) => assert!((value - 3.0).abs() < 1e-12),
            MathValue::Complex(value) => {
                assert!((value.re - 3.0).abs() < 1e-12);
                assert!(value.im.abs() < 1e-12);
            }
            _ => panic!("expected scalar hafnian value"),
        }
    }

    #[test]
    fn math_expression_supports_bessel_y_i_and_k_families() {
        let response = evaluate_math_expression(
            "bessely(0, 1) + besseli(0, 1) + besselk(0, 1)",
            MathOptions::default(),
        )
        .expect("bessel y/i/k functions should evaluate");

        match response.result {
            MathValue::Real(value) => {
                assert!(value.is_finite());
                assert!(value > 0.0);
            }
            _ => panic!("expected real result"),
        }
    }

    #[test]
    fn math_expression_supports_complex_inputs_for_bessely_and_besselk() {
        let response = evaluate_math_expression(
            "bessely(1, 0.8+0.4i) + besselk(1, -0.5+0.9i)",
            MathOptions::default(),
        )
        .expect("complex-domain continuation for bessely/besselk should evaluate");

        match response.result {
            MathValue::Complex(value) => {
                assert!(value.re.is_finite());
                assert!(value.im.is_finite());
            }
            MathValue::Real(value) => {
                assert!(value.is_finite());
            }
            _ => panic!("expected scalar result"),
        }
    }

    #[test]
    fn math_expression_supports_aliases_and_extended_specials() {
        let response = evaluate_math_expression(
            "Γ(3.5 + 2i) + ζ(2) + B(5, 3) + erf(1.5 + i) + Si(2.5) + Ci(1.8) + FresnelC(2.0) + FresnelS(1.5) + Ei(0.5) + li(2.0) + W(2.5) + sinc(3.0)",
            MathOptions::default(),
        )
        .expect("extended aliases and specials should evaluate");

        match response.result {
            MathValue::Real(value) => assert!(value.is_finite()),
            MathValue::Complex(value) => {
                assert!(value.re.is_finite());
                assert!(value.im.is_finite());
            }
            _ => panic!("expected scalar result"),
        }
    }

    #[test]
    fn math_expression_supports_definite_integral_with_bound_variable() {
        let response = evaluate_math_expression(
            "integral(0, 1, exp(-(x^2)), x)",
            MathOptions::default(),
        )
        .expect("definite integral expression should evaluate");

        match response.result {
            MathValue::Real(value) => {
                assert!(value.is_finite());
                assert!((value - 0.7468241328).abs() < 5e-4);
            }
            MathValue::Complex(value) => {
                assert!(value.re.is_finite());
                assert!(value.im.abs() < 1e-6);
                assert!((value.re - 0.7468241328).abs() < 5e-4);
            }
            _ => panic!("expected scalar result"),
        }
    }

    #[test]
    fn math_expression_supports_airy_and_theta4() {
        let response = evaluate_math_expression(
            "Ai(1.5) + Bi(0.5) + theta4(0.2, 0.5)",
            MathOptions::default(),
        )
        .expect("airy and theta4 expression should evaluate");

        match response.result {
            MathValue::Real(value) => assert!(value.is_finite()),
            MathValue::Complex(value) => {
                assert!(value.re.is_finite());
                assert!(value.im.is_finite());
            }
            _ => panic!("expected scalar result"),
        }
    }

    #[test]
    fn bridge_audit_includes_step_diagnostics() {
        let response = evaluate_math_expression("gamma(5)", MathOptions::default())
            .expect("expression should evaluate");

        assert!(response
            .bridge_audit
            .get("diagnostics")
            .and_then(Value::as_array)
            .map(|items| !items.is_empty())
            .unwrap_or(false));
        assert!(response
            .bridge_audit
            .get("math_jobs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|job| job.get("derivation_steps"))
            .and_then(Value::as_array)
            .map(|items| !items.is_empty())
            .unwrap_or(false));
    }
}