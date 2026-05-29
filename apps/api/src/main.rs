use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use digitalcrystal_engine::{
    AppConfig, BankRecord, CrystalRecord, SolverRequest, SolverResponse, migrate_bank_to_v2,
    migrate_crystal_to_v2, solve_linear_equation, validate_bank, validate_crystal,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
struct ApiState {
    config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config_path_from_args();
    let config = AppConfig::load_from_path(&config_path)?;
    let bind_address = config.runtime.bind_address.clone();
    let state = ApiState {
        config: Arc::new(config),
    };

    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    println!("DigitalCrystal API listening on {bind_address}");
    println!("Loaded config path: {config_path}");

    axum::serve(listener, build_router(state)).await?;
    Ok(())
}

fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/config", get(get_config))
        .route("/v1/rwif/validate", post(validate_rwif))
        .route("/v1/solve/linear", post(solve_linear))
        .with_state(state)
}

fn config_path_from_args() -> String {
    std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|pair| {
            if pair[0] == "--config" {
                Some(pair[1].clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "configs/solver.default.toml".to_string())
}

async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "digitalcrystal-api",
        engine_mode: state.config.solver.engine_mode.clone(),
        bind_address: state.config.runtime.bind_address.clone(),
        deterministic_replay: state.config.runtime.deterministic_replay,
    })
}

async fn get_config(State(state): State<ApiState>) -> Json<AppConfig> {
    Json((*state.config).clone())
}

async fn validate_rwif(
    State(_state): State<ApiState>,
    Json(request): Json<ValidateRwifRequest>,
) -> Result<Json<ValidateRwifResponse>, ApiError> {
    match detect_shape(&request.document)? {
        RwifShape::Bank => {
            let parsed: BankRecord = serde_json::from_value(request.document.clone())
                .map_err(ApiError::InvalidDocument)?;
            let bank = if request.migrate_to_v2.unwrap_or(false) {
                migrate_bank_to_v2(parsed)
            } else {
                parsed
            };
            let report = validate_bank(&bank);
            Ok(Json(ValidateRwifResponse {
                shape: "bank",
                valid: report.valid,
                issues: report.issues,
                migrated_document: if request.migrate_to_v2.unwrap_or(false) {
                    Some(serde_json::to_value(bank).map_err(ApiError::InvalidDocument)?)
                } else {
                    None
                },
            }))
        }
        RwifShape::Crystal => {
            let parsed: CrystalRecord = serde_json::from_value(request.document.clone())
                .map_err(ApiError::InvalidDocument)?;
            let crystal = if request.migrate_to_v2.unwrap_or(false) {
                migrate_crystal_to_v2(parsed)
            } else {
                parsed
            };
            let report = validate_crystal(&crystal);
            Ok(Json(ValidateRwifResponse {
                shape: "crystal",
                valid: report.valid,
                issues: report.issues,
                migrated_document: if request.migrate_to_v2.unwrap_or(false) {
                    Some(serde_json::to_value(crystal).map_err(ApiError::InvalidDocument)?)
                } else {
                    None
                },
            }))
        }
    }
}

async fn solve_linear(
    State(state): State<ApiState>,
    Json(request): Json<SolveLinearRequest>,
) -> Json<SolverResponse> {
    Json(solve_linear_equation(
        &SolverRequest {
            variable: request.variable,
            a: request.a,
            b: request.b,
        },
        &state.config.solver,
    ))
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    engine_mode: String,
    bind_address: String,
    deterministic_replay: bool,
}

#[derive(Debug, Deserialize)]
struct ValidateRwifRequest {
    document: Value,
    #[serde(default)]
    migrate_to_v2: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ValidateRwifResponse {
    shape: &'static str,
    valid: bool,
    issues: Vec<digitalcrystal_engine::ConformanceIssue>,
    migrated_document: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct SolveLinearRequest {
    #[serde(default = "default_variable")]
    variable: String,
    a: f64,
    b: f64,
}

enum RwifShape {
    Bank,
    Crystal,
}

fn detect_shape(document: &Value) -> Result<RwifShape, ApiError> {
    if document.get("crystals").is_some() {
        Ok(RwifShape::Bank)
    } else if document.get("edges").is_some() {
        Ok(RwifShape::Crystal)
    } else {
        Err(ApiError::UnsupportedShape)
    }
}

enum ApiError {
    UnsupportedShape,
    InvalidDocument(serde_json::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, payload) = match self {
            Self::UnsupportedShape => (
                StatusCode::BAD_REQUEST,
                json_error("RWIF_SHAPE_UNSUPPORTED", "expected a bank or crystal document"),
            ),
            Self::InvalidDocument(source) => (
                StatusCode::BAD_REQUEST,
                json_error("RWIF_DOCUMENT_INVALID", &source.to_string()),
            ),
        };
        (status, Json(payload)).into_response()
    }
}

fn json_error(code: &str, message: &str) -> Value {
    serde_json::json!({
        "error": {
            "code": code,
            "message": message,
        }
    })
}

fn default_variable() -> String {
    "x".to_string()
}

#[cfg(test)]
mod tests {
    use super::{ApiState, build_router};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use digitalcrystal_engine::{AppConfig, RWIF_EVENT_SCHEMA_VERSION, RWIF_SCHEMA_VERSION};
    use serde_json::{Value, json};
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn health_returns_runtime_status() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload: Value = serde_json::from_slice(&body).expect("json payload should parse");
        assert_eq!(payload.get("status"), Some(&Value::String("ok".to_string())));
    }

    #[tokio::test]
    async fn rwif_validate_migrates_v1_fixture() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/conformance/rwif_v2/fixtures/RWIF-C-001-v1-bank.json"
        ))
        .expect("fixture should parse");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/rwif/validate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "document": fixture,
                            "migrate_to_v2": true,
                        }))
                        .expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload: Value = serde_json::from_slice(&body).expect("json payload should parse");
        assert_eq!(payload.get("valid"), Some(&Value::Bool(true)));
        assert_eq!(payload.get("shape"), Some(&Value::String("bank".to_string())));
        assert_eq!(
            payload
                .get("migrated_document")
                .and_then(|value| value.get("rwif_schema_version"))
                .and_then(Value::as_str),
            Some(RWIF_SCHEMA_VERSION)
        );
        assert_eq!(
            payload
                .get("migrated_document")
                .and_then(|value| value.get("crystals"))
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|crystal| crystal.get("edges"))
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|edge| edge.get("phase_trajectory"))
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|event| event.get("schema_version"))
                .and_then(Value::as_str),
            Some(RWIF_EVENT_SCHEMA_VERSION)
        );
    }

    #[tokio::test]
    async fn rwif_validate_reports_missing_integer_wrap_mode() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/conformance/rwif_v2/fixtures/RWIF-C-002-v2-invalid-missing-integer-wrap.json"
        ))
        .expect("fixture should parse");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/rwif/validate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "document": fixture,
                        }))
                        .expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload: Value = serde_json::from_slice(&body).expect("json payload should parse");
        assert_eq!(payload.get("valid"), Some(&Value::Bool(false)));
        assert!(payload
            .get("issues")
            .and_then(Value::as_array)
            .expect("issues should exist")
            .iter()
            .any(|issue| issue.get("code") == Some(&Value::String("RWIF_EDGE_INTEGER_WRAP_MODE_MISSING".to_string()))));
    }

    #[tokio::test]
    async fn linear_solver_endpoint_returns_route_audit() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/solve/linear")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "variable": "x",
                            "a": 2.0,
                            "b": -4.0,
                        }))
                        .expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload: Value = serde_json::from_slice(&body).expect("json payload should parse");
        assert_eq!(payload.get("decision_label"), Some(&Value::String("solved_linear_equation".to_string())));
        assert_eq!(payload.get("stop_reason"), Some(&Value::String("PathFound".to_string())));
        assert_eq!(payload.get("solved_value"), Some(&json!(2.0)));
        assert!(payload
            .get("route_audit")
            .and_then(|value| value.get("selected_path"))
            .and_then(Value::as_array)
            .map(|items| !items.is_empty())
            .unwrap_or(false));
    }
}