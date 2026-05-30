use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use digitalcrystal_engine::{
    AppConfig, BankRecord, CrystalRecord, PlatformCatalog, SolverRequest, SolverResponse,
    build_math_error_bridge_audit, classify_math_error, evaluate_math_expression, migrate_bank_to_v2,
    migrate_crystal_to_v2, parse_angle_unit, parse_math_mode, platform_catalog,
    solve_linear_equation, validate_bank, validate_crystal,
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
        .route("/", get(index))
        .route("/labs/special-functions", get(special_functions_lab))
        .route("/labs/riemann-hypothesis", get(riemann_hypothesis_lab))
        .route("/labs/research-findings", get(research_findings_lab))
        .route("/health", get(health))
        .route("/v1/config", get(get_config))
        .route("/v1/platform/modules", get(get_platform_modules))
        .route("/v1/csif/math", post(csif_math))
        .route("/v1/rwif/validate", post(validate_rwif))
        .route("/v1/solve/linear", post(solve_linear))
        .with_state(state)
}

async fn index(State(_state): State<ApiState>) -> Html<String> {
    Html(render_landing_page(&platform_catalog()))
}

async fn special_functions_lab(State(_state): State<ApiState>) -> Html<String> {
    Html(render_special_functions_lab())
}
async fn riemann_hypothesis_lab(State(_state): State<ApiState>) -> Html<String> {
    Html(render_riemann_hypothesis_lab())
}

async fn research_findings_lab(State(_state): State<ApiState>) -> Html<String> {
    Html(render_research_findings_lab())
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

async fn get_platform_modules(State(_state): State<ApiState>) -> Json<PlatformCatalog> {
    Json(platform_catalog())
}

async fn csif_math(
    State(_state): State<ApiState>,
    Json(request): Json<MathRequest>,
) -> impl IntoResponse {
    let mode = match parse_math_mode(request.mode.as_deref()) {
        Ok(value) => value,
        Err(error) => return math_error_response(&request.expression, error),
    };
    let angle_unit = match parse_angle_unit(request.angle_unit.as_deref()) {
        Ok(value) => value,
        Err(error) => return math_error_response(&request.expression, error),
    };
    match evaluate_math_expression(
        &request.expression,
        digitalcrystal_engine::MathOptions { mode, angle_unit },
    ) {
        Ok(response) => (StatusCode::OK, Json(serde_json::to_value(response).expect("math response should serialize"))).into_response(),
        Err(error) => math_error_response(&request.expression, error),
    }
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

#[derive(Debug, Deserialize)]
struct MathRequest {
    expression: String,
    mode: Option<String>,
    angle_unit: Option<String>,
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

fn math_error_response(expression: &str, error: digitalcrystal_engine::MathError) -> axum::response::Response {
    let (status, code) = classify_math_error(&error);
    let bridge_audit = build_math_error_bridge_audit(expression, &error);
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": {
                "code": code,
                "message": error.to_string(),
                "status": status,
            },
            "bridge_audit": bridge_audit,
        })),
    )
        .into_response()
}

fn render_landing_page(catalog: &PlatformCatalog) -> String {
        let shared_capabilities = catalog
                .shared_capabilities
                .iter()
                .map(|item| format!("<li>{item}</li>"))
                .collect::<Vec<_>>()
                .join("");
        let module_cards = catalog
                .modules
                .iter()
                .map(|module| {
                        let capabilities = module
                                .capabilities
                                .iter()
                                .map(|item| format!("<li>{item}</li>"))
                                .collect::<Vec<_>>()
                                .join("");
                        format!(
                            "<article class=\"module-card\"><div class=\"module-head\"><p class=\"module-status\">{}</p><h2>{}</h2></div><p class=\"module-summary\">{}</p><p class=\"module-route\">Primary route: <a href=\"{}\">{}</a></p><ul>{}</ul></article>",
                            module.status, module.title, module.summary, module.primary_route, module.primary_route, capabilities
                        )
                })
                .collect::<Vec<_>>()
                .join("");

        let research_module_card = "
<article class=\"module-card\">
    <div class=\"module-head\">
        <p class=\"module-status\">research track</p>
        <h2>Riemann Hypothesis Lab</h2>
    </div>
    <p class=\"module-summary\">A dedicated workspace for RH-equivalent statement mapping, theorem-attempt planning, and computational validation harness design.</p>
    <p class=\"module-route\">Primary route: <a href=\"/labs/riemann-hypothesis\">/labs/riemann-hypothesis</a></p>
    <ul>
        <li>Equivalent-statement dependency map</li>
        <li>Lemma registry with proof obligations</li>
        <li>Validation harness planning and result logging</li>
    </ul>
</article>";

        let module_cards = format!("{module_cards}{research_module_card}");

        format!(
                "<!doctype html>
<html lang=\"en\">
<head>
    <meta charset=\"utf-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
    <title>{}</title>
    <style>
        :root {{
            color-scheme: light;
            --bg: #f5f1e8;
            --panel: rgba(255, 252, 246, 0.9);
            --ink: #182126;
            --muted: #59656d;
            --line: rgba(24, 33, 38, 0.12);
            --accent: #0e8b6f;
            --accent-soft: #d9f3ea;
        }}
        * {{ box-sizing: border-box; }}
        body {{
            margin: 0;
            font-family: Georgia, 'Iowan Old Style', 'Palatino Linotype', serif;
            color: var(--ink);
            background:
                radial-gradient(circle at top, rgba(14, 139, 111, 0.18), transparent 38%),
                linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
        }}
        main {{ max-width: 1120px; margin: 0 auto; padding: 48px 24px 72px; }}
        .hero {{
            background: var(--panel);
            border: 1px solid var(--line);
            border-radius: 28px;
            padding: 32px;
            box-shadow: 0 18px 48px rgba(24, 33, 38, 0.08);
            backdrop-filter: blur(14px);
        }}
        .eyebrow {{
            margin: 0 0 12px;
            color: var(--accent);
            font-family: 'Courier New', monospace;
            font-size: 12px;
            letter-spacing: 0.18em;
            text-transform: uppercase;
        }}
        h1 {{ font-size: clamp(2.4rem, 5vw, 4.6rem); margin: 0 0 14px; line-height: 0.96; }}
        .lede {{ max-width: 60ch; margin: 0; color: var(--muted); font-size: 1.08rem; line-height: 1.6; }}
        .hero-grid {{ display: grid; grid-template-columns: 1.2fr 0.8fr; gap: 24px; margin-top: 28px; }}
        .capsule, .api-note, .module-card {{
            background: rgba(255, 255, 255, 0.7);
            border: 1px solid var(--line);
            border-radius: 22px;
        }}
        .capsule {{ padding: 22px 24px; }}
        .capsule h2, .api-note h2 {{ margin: 0 0 12px; font-size: 1.1rem; }}
        .capsule ul, .module-card ul {{ margin: 0; padding-left: 18px; color: var(--muted); }}
        .capsule li, .module-card li {{ margin-bottom: 8px; }}
        .api-note {{ padding: 22px 24px; display: flex; flex-direction: column; gap: 10px; }}
        .api-note code {{ font-family: 'Courier New', monospace; font-size: 0.95rem; color: var(--ink); }}
        .lab-link {{
            display: inline-block;
            margin-top: 6px;
            padding: 10px 14px;
            border-radius: 999px;
            border: 1px solid var(--line);
            background: var(--accent-soft);
            color: var(--ink);
            text-decoration: none;
            font-family: 'Courier New', monospace;
            font-size: 0.88rem;
        }}
        .modules {{ margin-top: 28px; display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 18px; }}
        .module-card {{ padding: 22px; }}
        .module-head {{ display: flex; justify-content: space-between; gap: 16px; align-items: baseline; }}
        .module-head h2 {{ margin: 0; font-size: 1.2rem; }}
        .module-status {{ margin: 0; font-family: 'Courier New', monospace; color: var(--accent); text-transform: uppercase; font-size: 0.75rem; letter-spacing: 0.12em; }}
        .module-summary, .module-route {{ color: var(--muted); line-height: 1.5; }}
        .module-route {{ font-family: 'Courier New', monospace; font-size: 0.92rem; }}
        @media (max-width: 820px) {{
            .hero-grid {{ grid-template-columns: 1fr; }}
            main {{ padding: 24px 16px 48px; }}
            .hero {{ padding: 24px; }}
        }}
    </style>
</head>
<body>
    <main>
        <section class=\"hero\">
            <p class=\"eyebrow\">Deterministic scientific platform</p>
            <h1>{}</h1>
            <p class=\"lede\">{}</p>
            <div class=\"hero-grid\">
                <section class=\"capsule\">
                    <h2>Shared engine capabilities</h2>
                    <ul>{}</ul>
                </section>
                <section class=\"api-note\">
                    <h2>Current API surface</h2>
                    <code>GET /v1/platform/modules</code>
                    <code>POST /v1/rwif/validate</code>
                    <code>POST /v1/solve/linear</code>
                    <code>GET /health</code>
                    <a class=\"lab-link\" href=\"/labs/special-functions\">Open Special Functions Lab</a>
                    <a class=\"lab-link\" href=\"/labs/riemann-hypothesis\">Open Riemann Hypothesis Research Lab</a>
                </section>
            </div>
            <section class=\"modules\">{}</section>
        </section>
    </main>
</body>
</html>",
                catalog.title,
                catalog.title,
                catalog.tagline,
                shared_capabilities,
                module_cards
        )
}

fn render_special_functions_lab() -> String {
        "<!doctype html>
<html lang=\"en\">
<head>
    <meta charset=\"utf-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
    <title>Special Functions Lab</title>
    <style>
        :root {
            --bg: #f4efe4;
            --panel: rgba(255,255,255,0.78);
            --ink: #1c2428;
            --muted: #5d676d;
            --line: rgba(28,36,40,0.12);
            --accent: #0c7a63;
            --accent-soft: #daf0e8;
            --mono: 'Courier New', monospace;
        }
        * { box-sizing: border-box; }
        body {
            margin: 0;
            font-family: Georgia, 'Iowan Old Style', 'Palatino Linotype', serif;
            color: var(--ink);
            background:
                radial-gradient(circle at top right, rgba(12,122,99,0.14), transparent 32%),
                linear-gradient(180deg, #fbf8f1 0%, var(--bg) 100%);
        }
        main { max-width: 1180px; margin: 0 auto; padding: 28px 18px 48px; }
        .hero, .panel {
            background: var(--panel);
            border: 1px solid var(--line);
            border-radius: 26px;
            box-shadow: 0 20px 48px rgba(28,36,40,0.08);
            backdrop-filter: blur(12px);
        }
        .hero { padding: 28px; margin-bottom: 18px; }
        .eyebrow { margin: 0 0 10px; color: var(--accent); font-family: var(--mono); font-size: 12px; letter-spacing: 0.16em; text-transform: uppercase; }
        h1 { margin: 0 0 10px; font-size: clamp(2.2rem, 4vw, 4rem); line-height: 0.95; }
        .lede { margin: 0; max-width: 62ch; color: var(--muted); line-height: 1.6; }
        .grid { display: grid; grid-template-columns: 1.2fr 0.8fr; gap: 18px; }
        .panel { padding: 22px; }
        label { display: block; font-size: 12px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--muted); margin-bottom: 8px; }
        input, select, button, textarea {
            width: 100%;
            border-radius: 16px;
            border: 1px solid var(--line);
            padding: 14px 16px;
            font: inherit;
            background: rgba(255,255,255,0.82);
            color: var(--ink);
        }
        textarea, input { font-family: var(--mono); }
        textarea { min-height: 110px; resize: vertical; line-height: 1.45; }
        .controls { display: grid; grid-template-columns: 1fr 160px 160px; gap: 12px; margin-bottom: 14px; }
        .actions { display: flex; gap: 10px; margin-bottom: 14px; }
        button { cursor: pointer; background: var(--accent); color: white; border-color: transparent; }
        button.secondary { background: var(--accent-soft); color: var(--ink); }
        .helper-bar { margin: 0 0 14px; padding: 12px; border: 1px solid var(--line); border-radius: 16px; background: rgba(255,255,255,0.65); }
        .helper-title { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); margin-bottom: 10px; }
        .mode-note { margin-top: 8px; font-size: 12px; line-height: 1.45; color: #36574f; }
        .mode-note strong { color: #1f4b43; }
        .literal-palette { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 12px; }
        .literal-palette button { width: auto; padding: 8px 10px; font-size: 12px; border-radius: 999px; }
        .domain-hints { border: 1px solid rgba(12,122,99,0.18); border-radius: 14px; padding: 12px 14px; background: rgba(12,122,99,0.06); color: #1f4b43; }
        .domain-hints .hint-title { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: #2f6b60; margin-bottom: 8px; }
        .domain-hints ul { margin: 0; padding-left: 18px; }
        .domain-hints li { margin: 4px 0; }
        .trust-dialog { border: 0; border-radius: 20px; padding: 0; width: min(900px, 92vw); box-shadow: 0 30px 90px rgba(0,0,0,0.28); }
        .trust-dialog::backdrop { background: rgba(11,20,25,0.42); }
        .trust-dialog-shell { padding: 20px; background: linear-gradient(180deg, rgba(251,253,252,0.98), rgba(244,248,247,0.98)); }
        .trust-dialog-header { display: flex; align-items: center; justify-content: space-between; gap: 14px; margin-bottom: 14px; }
        .trust-dialog-header h2 { margin: 0; font-size: 20px; }
        .trust-dialog-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 14px; }
        .trust-dialog-panel { border: 1px solid rgba(11,20,25,0.12); border-radius: 16px; padding: 12px; background: rgba(255,255,255,0.85); }
        .trust-dialog-panel label { display: block; font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); margin-bottom: 8px; }
        .trust-dialog-panel pre { max-height: 360px; overflow: auto; }
        .trust-dialog-actions { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 14px; }
        .mini-button { width: auto; padding: 7px 10px; font-size: 12px; border-radius: 999px; }
        .trust-chip { width: auto; padding: 6px 8px; font-size: 11px; border-radius: 999px; }
        .samples { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 8px; }
        .samples button { width: auto; padding: 10px 12px; font-size: 13px; }
        .metric-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-top: 12px; }
        .metric { border: 1px solid var(--line); border-radius: 18px; padding: 14px; background: rgba(255,255,255,0.65); }
        .metric .label { font-size: 11px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.08em; }
        .metric .value { margin-top: 8px; font-family: var(--mono); font-size: 18px; }
        pre { margin: 0; white-space: pre-wrap; word-break: break-word; font-family: var(--mono); font-size: 13px; line-height: 1.5; }
        .trace { max-height: 520px; overflow: auto; }
        .trace-item { border-bottom: 1px solid var(--line); padding: 10px 0; }
        .trace-item:last-child { border-bottom: 0; }
        .trace-rule { display: flex; align-items: center; gap: 8px; font-family: var(--mono); font-size: 12px; color: var(--accent); }
        .trace-expr { margin: 4px 0; font-family: var(--mono); }
        .trace-result { color: var(--muted); font-size: 13px; }
        .phase-controls { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; }
        .phase-controls select { max-width: 220px; }
        .phase-history-list { display: grid; gap: 8px; margin-top: 12px; max-height: 180px; overflow: auto; }
        .phase-history-item { border: 1px solid var(--line); border-radius: 14px; padding: 10px 12px; background: rgba(255,255,255,0.7); }
        .phase-history-item strong { display: block; font-family: var(--mono); font-size: 12px; color: var(--accent); margin-bottom: 4px; }
        .phase-history-item span { display: block; color: var(--muted); font-size: 13px; }
        .identity-actions { display: flex; flex-wrap: wrap; gap: 10px; margin: 10px 0 12px; }
        .identity-actions button { width: auto; }
        .identity-list { max-height: 260px; overflow: auto; display: grid; gap: 8px; }
        .identity-item { border: 1px solid var(--line); border-radius: 14px; padding: 10px 12px; background: rgba(255,255,255,0.74); }
        .identity-item strong { display: block; font-family: var(--mono); font-size: 12px; color: var(--accent); margin-bottom: 4px; }
        .identity-item span { display: block; font-family: var(--mono); font-size: 12px; color: var(--ink); line-height: 1.45; }
        .identity-item-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 4px; }
        .identity-remove { width: auto; padding: 4px 8px; border-radius: 999px; font-size: 11px; }
        .footer-note { margin-top: 12px; color: var(--muted); font-size: 13px; }
        @media (max-width: 960px) {
            .grid { grid-template-columns: 1fr; }
            .controls { grid-template-columns: 1fr; }
            .metric-grid { grid-template-columns: 1fr; }
        }
    </style>
</head>
<body>
    <main>
        <section class=\"hero\">
            <p class=\"eyebrow\">Special Functions Lab</p>
            <h1>Complex-aware deterministic evaluation</h1>
            <p class=\"lede\">This first lab page is wired directly to <code>/v1/csif/math</code>. It now exposes complex literals, matrix literals, <code>det</code>, <code>inverse</code>, <code>hafnian</code>, <code>tf</code>, <code>ln</code>, <code>log</code>, <code>gamma</code>, <code>lambertw</code>, <code>zeta</code>, <code>polylog</code>, <code>gammainc</code>, and Bessel <code>J/Y/I/K</code> helpers with structured derivation traces, bridge audit, and RWIF-shaped export.</p>
        </section>
        <section class=\"grid\">
            <section class=\"panel\">
                <div class=\"controls\">
                    <div>
                        <label for=\"expression\">Expression</label>
                        <textarea id=\"expression\" rows=\"4\">exp(i*pi) + 1</textarea>
                    </div>
                    <div>
                        <label for=\"mode\">Mode</label>
                        <select id=\"mode\"><option value=\"algebraic\">algebraic</option><option value=\"geometric\">geometric</option><option value=\"symbolic_identity\">symbolic identity</option></select>
                        <div class=\"mode-note\" id=\"modeNote\"><strong>Geometric mode</strong> uses exact decimal scaling for ordinary real arithmetic, so values like <code>0.1 + 0.2</code> stay at <code>0.3</code> instead of inheriting floating drift from a purely algebraic evaluation.</div>
                    </div>
                    <div>
                        <label for=\"angleUnit\">Angle Unit</label>
                        <select id=\"angleUnit\"><option value=\"radians\">radians</option><option value=\"degrees\">degrees</option></select>
                    </div>
                </div>
                <div class=\"actions\">
                    <button id=\"runButton\">Evaluate</button>
                    <button class=\"secondary\" id=\"exportButton\">Copy RWIF JSON</button>
                    <button class=\"secondary\" id=\"downloadButton\">Download Raw JSON</button>
                </div>
                <div class=\"helper-bar\">
                    <div class=\"helper-title\">Expression palette and shorthand support</div>
                    <div class=\"literal-palette\" id=\"literalPalette\">
                        <button class=\"secondary mini-button\" data-insert=\"Γ(\">Γ</button>
                        <button class=\"secondary mini-button\" data-insert=\"ζ(\">ζ</button>
                        <button class=\"secondary mini-button\" data-insert=\"integral(\">∫</button>
                        <button class=\"secondary mini-button\" data-insert=\"Ai(\">Ai</button>
                        <button class=\"secondary mini-button\" data-insert=\"Bi(\">Bi</button>
                        <button class=\"secondary mini-button\" data-insert=\"theta4(\">θ₄</button>
                        <button class=\"secondary mini-button\" data-insert=\"sqrt(\">√</button>
                        <button class=\"secondary mini-button\" data-insert=\"pi\">π</button>
                        <button class=\"secondary mini-button\" data-insert=\"i\">i</button>
                        <button class=\"secondary mini-button\" data-insert=\"(\">(</button>
                        <button class=\"secondary mini-button\" data-insert=\")\">)</button>
                        <button class=\"secondary mini-button\" data-insert=\",\">,</button>
                        <button class=\"secondary mini-button\" data-insert=\"[\">[</button>
                        <button class=\"secondary mini-button\" data-insert=\"]\">]</button>
                    </div>
                    <div class=\"domain-hints\" id=\"domainHints\">
                        <div class=\"hint-title\">Domain and branch notes</div>
                        <ul>
                            <li>Common LaTeX shorthand such as <code>\\Gamma</code>, <code>\\zeta</code>, <code>\\operatorname{Ai}</code>, and <code>\\frac</code> is normalized when possible.</li>
                            <li>Functions like <code>ci</code>, <code>li</code>, <code>Ai</code>, <code>Bi</code>, and <code>theta4</code> carry domain restrictions; the page will show them before you run.</li>
                            <li>Use <code>exp(-(x^2))</code> rather than <code>exp(-x^2)</code> when you mean a Gaussian.</li>
                        </ul>
                    </div>
                </div>
                <div class=\"samples\">
                    <button class=\"secondary sample\" data-expression=\"2*(3+4)^2\">Scalar trace</button>
                    <button class=\"secondary sample\" data-expression=\"conj(2+3i) + arg(1+i)\">Complex conjugate + arg</button>
                    <button class=\"secondary sample\" data-expression=\"ln(e) + log(100) + gamma(5)\">Logs + gamma</button>
                    <button class=\"secondary sample\" data-expression=\"lambertw(1) + zeta(2)\">Lambert W + zeta</button>
                    <button class=\"secondary sample\" data-expression=\"polylog(1, 0.5) + gammainc(1, 2)\">Polylog + gammainc</button>
                    <button class=\"secondary sample\" data-expression=\"besselj(2, 1.1) + j_sph(1, 1.1)\">Bessel family</button>
                    <button class=\"secondary sample\" data-expression=\"bessely(0, 1) + besseli(0, 1) + besselk(0, 1)\">Bessel Y/I/K</button>
                    <button class=\"secondary sample\" data-expression=\"det([[1,2],[3,4]]) + tf([[1,0]], [[1,1]], 1)\">Matrix + transfer</button>
                    <button class=\"secondary sample\" data-expression=\"hafnian([[0,1,1,1],[1,0,1,1],[1,1,0,1],[1,1,1,0]])\">Hafnian sample</button>
                    <button class=\"secondary sample\" data-expression=\"exp(i*pi) + 1\">Euler identity</button>
                    <button class=\"secondary sample\" data-expression=\"\\Gamma(z+1)=z\\Gamma(z)\" data-mode=\"symbolic_identity\" data-angle=\"radians\">Symbolic identity</button>
                    <button class=\"secondary sample\" data-expression=\"0.1+0.2\" data-mode=\"geometric\" data-angle=\"radians\">Exact decimal scaling</button>
                    <button class=\"secondary sample\" data-expression=\"sin(30)+cos(60)\" data-mode=\"geometric\" data-angle=\"degrees\">Degree-mode trig</button>
                </div>
                <div class=\"metric-grid\">
                    <div class=\"metric\"><div class=\"label\">Result</div><div class=\"value\" id=\"resultValue\">-</div></div>
                    <div class=\"metric\"><div class=\"label\">Final Theta</div><div class=\"value\" id=\"thetaValue\">-</div></div>
                    <div class=\"metric\"><div class=\"label\">Crystal State</div><div class=\"value\" id=\"stateValue\">-</div></div>
                </div>
                <div class=\"panel\" style=\"margin-top:14px; background: rgba(255,255,255,0.55);\">
                    <label>Phase Trajectory</label>
                    <div class=\"phase-controls\">
                        <span style=\"font-size:12px; letter-spacing:0.08em; text-transform:uppercase; color:var(--muted);\">Metric</span>
                        <select id=\"phaseMetric\">
                            <option value=\"discrete\">Discrete operator slots</option>
                            <option value=\"cumulative\">Cumulative composition</option>
                        </select>
                    </div>
                    <div style=\"display:flex; justify-content:center;\">
                        <svg width=\"220\" height=\"130\" viewBox=\"0 0 220 130\" role=\"img\" aria-label=\"Phase arc\">
                            <circle cx=\"110\" cy=\"110\" r=\"78\" fill=\"none\" stroke=\"rgba(28,36,40,0.18)\" stroke-width=\"2\" />
                            <polyline id=\"phaseHistory\" fill=\"none\" stroke=\"rgba(12,122,99,0.45)\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />
                            <path id=\"phaseArc\" fill=\"none\" stroke=\"#0c7a63\" stroke-width=\"5\" stroke-linecap=\"round\" />
                            <line id=\"phaseNeedle\" x1=\"110\" y1=\"110\" x2=\"188\" y2=\"110\" stroke=\"#0c7a63\" stroke-width=\"3\" stroke-linecap=\"round\" />
                            <g id=\"phaseDots\"></g>
                            <circle cx=\"110\" cy=\"110\" r=\"4\" fill=\"rgba(28,36,40,0.6)\" />
                            <text id=\"phaseLabel\" x=\"110\" y=\"24\" text-anchor=\"middle\" font-family=\"Courier New, monospace\" font-size=\"12\" fill=\"#5d676d\">θ = 0.0000</text>
                        </svg>
                    </div>
                    <div class=\"phase-history-list\" id=\"phaseSteps\"></div>
                </div>
                <p class=\"footer-note\" id=\"statusLine\">Waiting for evaluation.</p>
            </section>
            <section class=\"panel\">
                <label>Derivation Trace</label>
                <div class=\"trace\" id=\"tracePanel\"></div>
            </section>
        </section>
        <section class=\"grid\" style=\"margin-top:18px;\">
            <section class=\"panel\">
                <label>Bridge Audit</label>
                <pre id=\"bridgePanel\">-</pre>
            </section>
            <section class=\"panel\">
                <label>RWIF Export</label>
                <pre id=\"rwifPanel\">-</pre>
            </section>
            <section class=\"panel\">
                <label>Raw JSON</label>
                <pre id=\"rawPanel\">-</pre>
            </section>
        </section>
        <section class=\"panel\" style=\"margin-top:18px;\">
            <label>Trusted Identities</label>
            <div class=\"identity-actions\">
                <button class=\"secondary\" id=\"copyIdentitiesButton\">Copy Trusted Identities</button>
                <button class=\"secondary\" id=\"downloadIdentitiesButton\">Download Trusted Identities</button>
                <button class=\"secondary\" id=\"clearIdentitiesButton\">Clear Session Identities</button>
            </div>
            <div class=\"identity-list\" id=\"identityList\"></div>
        </section>
        <dialog id=\"trustDialog\" class=\"trust-dialog\">
            <div class=\"trust-dialog-shell\">
                <div class=\"trust-dialog-header\">
                    <h2 id=\"trustDialogTitle\">Numeric trust details</h2>
                    <button class=\"secondary mini-button\" id=\"trustDialogClose\">Close</button>
                </div>
                <div class=\"trust-dialog-grid\">
                    <section class=\"trust-dialog-panel\">
                        <label>Metadata</label>
                        <pre id=\"trustDialogMeta\">-</pre>
                    </section>
                    <section class=\"trust-dialog-panel\">
                        <label>Expression / LaTeX</label>
                        <pre id=\"trustDialogLatex\">-</pre>
                    </section>
                    <section class=\"trust-dialog-panel\">
                        <label>Flux Probe</label>
                        <pre id=\"trustDialogFlux\">-</pre>
                    </section>
                </div>
                <div class=\"trust-dialog-actions\">
                    <button id=\"trustCopyJson\">Copy JSON</button>
                    <button class=\"secondary\" id=\"trustCopyLatex\">Copy LaTeX</button>
                    <button class=\"secondary\" id=\"trustDownloadJson\">Download JSON</button>
                </div>
            </div>
        </dialog>
    </main>
    <script>
        const expressionEl = document.getElementById('expression');
        const modeEl = document.getElementById('mode');
        const angleEl = document.getElementById('angleUnit');
        const resultValueEl = document.getElementById('resultValue');
        const thetaValueEl = document.getElementById('thetaValue');
        const stateValueEl = document.getElementById('stateValue');
        const tracePanelEl = document.getElementById('tracePanel');
        const bridgePanelEl = document.getElementById('bridgePanel');
        const rwifPanelEl = document.getElementById('rwifPanel');
        const rawPanelEl = document.getElementById('rawPanel');
        const statusLineEl = document.getElementById('statusLine');
        const phaseMetricEl = document.getElementById('phaseMetric');
        const phaseHistoryEl = document.getElementById('phaseHistory');
        const phaseArcEl = document.getElementById('phaseArc');
        const phaseNeedleEl = document.getElementById('phaseNeedle');
        const phaseDotsEl = document.getElementById('phaseDots');
        const phaseLabelEl = document.getElementById('phaseLabel');
        const phaseStepsEl = document.getElementById('phaseSteps');
        const domainHintsEl = document.getElementById('domainHints');
        const modeNoteEl = document.getElementById('modeNote');
        const identityListEl = document.getElementById('identityList');
        const copyIdentitiesButtonEl = document.getElementById('copyIdentitiesButton');
        const downloadIdentitiesButtonEl = document.getElementById('downloadIdentitiesButton');
        const clearIdentitiesButtonEl = document.getElementById('clearIdentitiesButton');
        const trustDialogEl = document.getElementById('trustDialog');
        const trustDialogTitleEl = document.getElementById('trustDialogTitle');
        const trustDialogMetaEl = document.getElementById('trustDialogMeta');
        const trustDialogLatexEl = document.getElementById('trustDialogLatex');
        const trustDialogFluxEl = document.getElementById('trustDialogFlux');
        const trustDialogCloseEl = document.getElementById('trustDialogClose');
        const trustCopyJsonEl = document.getElementById('trustCopyJson');
        const trustCopyLatexEl = document.getElementById('trustCopyLatex');
        const trustDownloadJsonEl = document.getElementById('trustDownloadJson');
        let lastPayload = null;
        let lastTraceSteps = [];
        let activeTrustStep = null;
        let trustedIdentities = [];

        function formatResult(value) {
            if (typeof value === 'number') return String(value);
            if (value && typeof value === 'object' && Number.isFinite(value.re) && Number.isFinite(value.im)) {
                const sign = value.im < 0 ? '' : '+';
                return `${value.re}${sign}${value.im}i`;
            }
            if (value && typeof value === 'object' && typeof value.statement === 'string' && typeof value.trusted === 'boolean') {
                const qualifier = value.trusted ? 'trusted symbolic statement' : 'symbolic statement';
                return `${value.statement} (${qualifier})`;
            }
            return JSON.stringify(value, null, 2);
        }

        function renderTrace(steps) {
            lastTraceSteps = Array.isArray(steps) ? steps : [];
            if (!Array.isArray(steps) || !steps.length) {
                tracePanelEl.innerHTML = '<p class=\"footer-note\">No derivation trace.</p>';
                return;
            }
            tracePanelEl.innerHTML = steps.map((step) => `
                <article class=\"trace-item\">
                    <div class=\"trace-rule\">
                        <span>${step.step}. ${step.rule}</span>
                        ${step.numeric_trust ? `<button class=\"secondary mini-button trust-chip\" data-trust-step=\"${step.step}\">Trust</button>` : ''}
                    </div>
                    <div class=\"trace-expr\">${step.expression}</div>
                    <div class=\"trace-result\">${step.result_text}</div>
                </article>
            `).join('');
        }

        function openTrustDialog(stepNumber) {
            const step = lastTraceSteps.find((item) => item.step === stepNumber);
            if (!step || !step.numeric_trust) return;
            activeTrustStep = step;
            trustDialogTitleEl.textContent = `Step #${step.step} • ${step.rule}`;
            trustDialogMetaEl.textContent = JSON.stringify(step.numeric_trust, null, 2);
            trustDialogLatexEl.textContent = step.latex || step.expression || '-';
            trustDialogFluxEl.textContent = step.numeric_trust?.hafnian_flux_probe
                ? JSON.stringify(step.numeric_trust.hafnian_flux_probe, null, 2)
                : 'No flux probe for this step.';
            if (typeof trustDialogEl.showModal === 'function') {
                trustDialogEl.showModal();
            } else {
                trustDialogEl.setAttribute('open', 'open');
            }
        }

        function closeTrustDialog() {
            activeTrustStep = null;
            if (typeof trustDialogEl.close === 'function') {
                trustDialogEl.close();
            } else {
                trustDialogEl.removeAttribute('open');
            }
        }

        function insertExpressionToken(token) {
            const start = expressionEl.selectionStart ?? expressionEl.value.length;
            const end = expressionEl.selectionEnd ?? expressionEl.value.length;
            const before = expressionEl.value.slice(0, start);
            const after = expressionEl.value.slice(end);
            expressionEl.value = `${before}${token}${after}`;
            const cursor = start + token.length;
            expressionEl.setSelectionRange(cursor, cursor);
            expressionEl.focus();
            updateDomainHints();
        }

        function updateDomainHints() {
            const text = expressionEl.value.toLowerCase();
            const hints = [];
            const pushHint = (label, message) => hints.push({ label, message });

            if (modeEl.value === 'geometric') pushHint('Geometric mode', 'Ordinary real arithmetic uses exact decimal scaling, which keeps inputs such as 0.1 + 0.2 at 0.3 instead of exposing floating drift from a purely algebraic path.');
            if (modeEl.value === 'symbolic_identity') pushHint('Symbolic identity mode', 'Equations are stored and displayed as trusted non-numeric statements instead of being executed as arithmetic.');
            if (text.includes('gamma') || text.includes('Γ'.toLowerCase())) pushHint('Gamma', 'Gamma uses Lanczos-style approximation and reflection around Re(z) < 0.5.');
            if (text.includes('zeta')) pushHint('Zeta', 'Zeta uses a Dirichlet eta series with analytic continuation.');
            if (text.includes('polylog')) pushHint('Polylog', 'Polylog is series-based for small |z| and continuation-backed otherwise.');
            if (text.includes('erf')) pushHint('Erf', 'Erf is computed with a finite power series.');
            if (text.includes('si(') || text.includes('ci(') || text.includes('fresnel')) pushHint('Quadrature', 'Si, Ci, FresnelC, and FresnelS use Simpson-rule quadrature.');
            if (text.includes('ei(') || text.includes('li(')) pushHint('Integrals', 'Ei and li are approximation-based and singular at specific points.');
            if (text.includes('ai(') || text.includes('bi(')) pushHint('Airy', 'Ai and Bi are supported on restricted real inputs in this slice.');
            if (text.includes('theta4')) pushHint('Theta4', 'theta4 requires |q| < 1 and uses a truncated theta series.');
            if (text.includes('integral(')) pushHint('Integral', 'integral(a,b,expr,x) uses Simpson quadrature and needs a named bound variable.');
            if (text.includes('bessely') || text.includes('besselk')) pushHint('Bessel Y/K', 'Y and K use continuation formulas; branch behavior is still relevant near singular points.');
            if (text.includes('-') && text.includes('^')) pushHint('Precedence', 'Use explicit grouping for negative powers, e.g. -(x^2).');

            if (!hints.length) {
                hints.push({ label: 'Ready', message: 'This expression looks syntactically ordinary. Evaluate when ready.' });
            }

            domainHintsEl.innerHTML = `
                <div class=\"hint-title\">Domain and branch notes</div>
                <ul>${hints.map((hint) => `<li><strong>${hint.label}:</strong> ${hint.message}</li>`).join('')}</ul>
            `;
        }

        function updateModeNote() {
            if (modeEl.value === 'geometric') {
                modeNoteEl.innerHTML = '<strong>Geometric mode</strong> uses exact decimal scaling for ordinary real arithmetic, so values like <code>0.1 + 0.2</code> stay at <code>0.3</code> instead of inheriting floating drift from a purely algebraic evaluation.';
                return;
            }
            if (modeEl.value === 'symbolic_identity') {
                modeNoteEl.innerHTML = '<strong>Symbolic identity mode</strong> stores formulas as trusted non-numeric statements and skips executable arithmetic evaluation.';
                return;
            }
            modeNoteEl.innerHTML = '<strong>Algebraic mode</strong> executes numeric evaluation directly and is best for concrete arithmetic and function results.';
        }

        function renderTrustedIdentities() {
            if (!trustedIdentities.length) {
                identityListEl.innerHTML = '<p class=\"footer-note\">No trusted identities stored yet in this session.</p>';
                return;
            }

            identityListEl.innerHTML = trustedIdentities.map((item, index) => `
                <article class=\"identity-item\">
                    <div class=\"identity-item-head\">
                        <strong>#${index + 1} ${item.mode}</strong>
                        <button class=\"secondary identity-remove\" data-identity-remove=\"${index}\">Remove</button>
                    </div>
                    <span>${item.statement}</span>
                </article>
            `).join('');
        }

        function removeTrustedIdentity(index) {
            if (!Number.isInteger(index) || index < 0 || index >= trustedIdentities.length) return;
            trustedIdentities.splice(index, 1);
            renderTrustedIdentities();
            statusLineEl.textContent = 'Trusted identity removed.';
        }

        function clearTrustedIdentities() {
            if (!trustedIdentities.length) {
                statusLineEl.textContent = 'No trusted identities to clear.';
                return;
            }
            trustedIdentities = [];
            renderTrustedIdentities();
            statusLineEl.textContent = 'Session trusted identities cleared.';
        }

        function maybeCaptureTrustedIdentity(payload) {
            const statement = payload?.result?.statement;
            const trusted = payload?.result?.trusted === true;
            const executable = payload?.result?.executable === false;
            if (!statement || !trusted || !executable) return;

            const alreadyPresent = trustedIdentities.some((item) => item.statement === statement);
            if (alreadyPresent) return;

            trustedIdentities.push({
                statement,
                mode: payload.mode || 'symbolic_identity',
                timestamp: new Date().toISOString(),
            });
            renderTrustedIdentities();
        }

        async function copyTrustedIdentities() {
            if (!trustedIdentities.length) {
                statusLineEl.textContent = 'No trusted identities available to copy yet.';
                return;
            }

            const text = trustedIdentities
                .map((item, index) => `${index + 1}. ${item.statement}`)
                .join('\\n');
            await navigator.clipboard.writeText(text);
            statusLineEl.textContent = 'Trusted identities copied to clipboard.';
        }

        function downloadTrustedIdentities() {
            if (!trustedIdentities.length) {
                statusLineEl.textContent = 'No trusted identities available to download yet.';
                return;
            }

            const blob = new Blob([JSON.stringify(trustedIdentities, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const anchor = document.createElement('a');
            anchor.href = url;
            anchor.download = 'trusted-identities.json';
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();
            URL.revokeObjectURL(url);
            statusLineEl.textContent = 'Trusted identities downloaded.';
        }

        function pointForTheta(theta, radius = 78) {
            const cx = 110;
            const cy = 110;
            return {
                x: cx + radius * Math.cos(-theta),
                y: cy + radius * Math.sin(-theta),
            };
        }

        function phaseMetricKey() {
            return phaseMetricEl.value === 'cumulative' ? 'cumulative_theta' : 'phase_theta';
        }

        function renderPhaseSteps(trajectory) {
            if (!Array.isArray(trajectory) || !trajectory.length) {
                phaseStepsEl.innerHTML = '<p class=\"footer-note\">No phase trajectory history.</p>';
                return;
            }

            const metricKey = phaseMetricKey();

            phaseStepsEl.innerHTML = trajectory.map((step) => `
                <article class=\"phase-history-item\">
                    <strong>#${step.monotonic_index} ${step.op}</strong>
                    <span>θ = ${Number(step[metricKey] || 0).toFixed(4)}</span>
                    <span>${step.output}</span>
                </article>
            `).join('');
        }

        function drawPhaseHistory(signature) {
            const cx = 110;
            const cy = 110;
            const radius = 78;
            const history = Array.isArray(signature?.trajectory) ? signature.trajectory : [];
            const metricKey = phaseMetricKey();
            const finalTheta = phaseMetricEl.value === 'cumulative'
                ? Number(signature?.cumulative_theta || 0)
                : Number(signature?.final_theta || 0);
            const samples = [0, ...history.map((step) => Number(step[metricKey] || 0)), finalTheta];
            const points = samples.map((value) => pointForTheta(value));
            phaseHistoryEl.setAttribute('points', points.map((point) => `${point.x.toFixed(2)},${point.y.toFixed(2)}`).join(' '));
            phaseDotsEl.innerHTML = history.map((step) => {
                const point = pointForTheta(Number(step[metricKey] || 0));
                return `<circle cx=\"${point.x.toFixed(2)}\" cy=\"${point.y.toFixed(2)}\" r=\"3\" fill=\"#0c7a63\"><title>#${step.monotonic_index} ${step.op}</title></circle>`;
            }).join('');

            const endX = cx + radius * Math.cos(-finalTheta);
            const endY = cy + radius * Math.sin(-finalTheta);
            phaseNeedleEl.setAttribute('x2', endX.toFixed(2));
            phaseNeedleEl.setAttribute('y2', endY.toFixed(2));
            if (Math.abs(finalTheta) < 0.0001) {
                phaseArcEl.setAttribute('d', '');
            } else {
                const startX = cx + radius;
                const startY = cy;
                const largeArc = Math.abs(finalTheta) > Math.PI ? 1 : 0;
                const sweep = finalTheta > 0 ? 0 : 1;
                phaseArcEl.setAttribute('d', `M ${startX} ${startY} A ${radius} ${radius} 0 ${largeArc} ${sweep} ${endX.toFixed(2)} ${endY.toFixed(2)}`);
            }
            phaseLabelEl.textContent = `θ = ${finalTheta.toFixed(4)}`;
            renderPhaseSteps(history);
        }

        function normalizeExpressionInput(raw) {
            if (typeof raw !== 'string') return '';
            return raw
                .replace(/[\\u200B-\\u200D\\uFEFF]/g, '')
                .replace(/\\u00A0/g, ' ')
                .replace(/\\u2212/g, '-')
                .replace(/[\\u00D7\\u2715\\u22C5\\u2217]/g, '*')
                .replace(/\\u00F7/g, '/')
                .trim();
        }

        function downloadRawJson() {
            if (!lastPayload) return;
            const blob = new Blob([JSON.stringify(lastPayload, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const anchor = document.createElement('a');
            anchor.href = url;
            anchor.download = 'digitalcrystal-math-result.json';
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();
            URL.revokeObjectURL(url);
            statusLineEl.textContent = 'Raw JSON downloaded.';
        }

        async function runEvaluation() {
            statusLineEl.textContent = 'Evaluating deterministically...';
            try {
                const normalizedExpression = normalizeExpressionInput(expressionEl.value);
                expressionEl.value = normalizedExpression;
                const response = await fetch('/v1/csif/math', {
                    method: 'POST',
                    headers: { 'content-type': 'application/json' },
                    body: JSON.stringify({
                        expression: normalizedExpression,
                        mode: modeEl.value,
                        angle_unit: angleEl.value,
                    }),
                });
                const payload = await response.json();
                lastPayload = payload;

                if (!response.ok || payload?.error) {
                    const code = payload?.error?.code || 'MATH_EVAL_ERROR';
                    const message = payload?.error?.message || 'Evaluation failed.';
                    resultValueEl.textContent = `${code}: ${message}`;
                    thetaValueEl.textContent = '-';
                    stateValueEl.textContent = '-';
                    tracePanelEl.innerHTML = '';
                    bridgePanelEl.textContent = JSON.stringify(payload.bridge_audit || payload, null, 2);
                    rwifPanelEl.textContent = '-';
                    rawPanelEl.textContent = JSON.stringify(payload, null, 2);
                    drawPhaseHistory({ trajectory: [], final_theta: 0, cumulative_theta: 0 });
                    statusLineEl.textContent = `${code}: ${message}`;
                    return;
                }

                resultValueEl.textContent = formatResult(payload.result);
                thetaValueEl.textContent = Number(payload.phase_signature?.final_theta || 0).toFixed(6);
                stateValueEl.textContent = payload.phase_signature?.crystal_state || '-';
                renderTrace(payload.derivation_trace);
                bridgePanelEl.textContent = JSON.stringify(payload.bridge_audit, null, 2);
                rwifPanelEl.textContent = JSON.stringify(payload.rwif_export, null, 2);
                rawPanelEl.textContent = JSON.stringify(payload, null, 2);
                drawPhaseHistory(payload.phase_signature || { trajectory: [], final_theta: 0, cumulative_theta: 0 });
                maybeCaptureTrustedIdentity(payload);
                statusLineEl.textContent = 'Deterministic evaluation complete.';
            } catch (error) {
                const message = error instanceof Error ? error.message : 'Network or runtime error.';
                const wrapped = {
                    error: {
                        code: 'MATH_UI_RUNTIME_ERROR',
                        message,
                    },
                };
                lastPayload = wrapped;
                resultValueEl.textContent = `MATH_UI_RUNTIME_ERROR: ${message}`;
                thetaValueEl.textContent = '-';
                stateValueEl.textContent = '-';
                tracePanelEl.innerHTML = '';
                bridgePanelEl.textContent = JSON.stringify(wrapped, null, 2);
                rwifPanelEl.textContent = '-';
                rawPanelEl.textContent = JSON.stringify(wrapped, null, 2);
                drawPhaseHistory({ trajectory: [], final_theta: 0, cumulative_theta: 0 });
                statusLineEl.textContent = `MATH_UI_RUNTIME_ERROR: ${message}`;
            }
        }

        document.getElementById('runButton').addEventListener('click', runEvaluation);
        document.getElementById('exportButton').addEventListener('click', async () => {
            if (!lastPayload?.rwif_export) return;
            await navigator.clipboard.writeText(JSON.stringify(lastPayload.rwif_export, null, 2));
            statusLineEl.textContent = 'RWIF export copied to clipboard.';
        });
        document.getElementById('downloadButton').addEventListener('click', downloadRawJson);
        document.querySelectorAll('.sample').forEach((button) => {
            button.addEventListener('click', () => {
                expressionEl.value = button.dataset.expression || expressionEl.value;
                modeEl.value = button.dataset.mode || 'algebraic';
                angleEl.value = button.dataset.angle || 'radians';
                updateModeNote();
                updateDomainHints();
                runEvaluation();
            });
        });
        modeEl.addEventListener('change', () => {
            updateModeNote();
            updateDomainHints();
        });
        document.getElementById('literalPalette').addEventListener('click', (event) => {
            const target = event.target.closest('[data-insert]');
            if (!target) return;
            insertExpressionToken(target.dataset.insert || '');
        });
        expressionEl.addEventListener('input', updateDomainHints);
        tracePanelEl.addEventListener('click', (event) => {
            const target = event.target.closest('[data-trust-step]');
            if (!target) return;
            openTrustDialog(Number(target.dataset.trustStep));
        });
        trustDialogCloseEl.addEventListener('click', closeTrustDialog);
        trustCopyJsonEl.addEventListener('click', async () => {
            if (!activeTrustStep?.numeric_trust) return;
            await navigator.clipboard.writeText(JSON.stringify(activeTrustStep.numeric_trust, null, 2));
            statusLineEl.textContent = 'Trust metadata copied to clipboard.';
        });
        trustCopyLatexEl.addEventListener('click', async () => {
            if (!activeTrustStep) return;
            await navigator.clipboard.writeText(activeTrustStep.latex || activeTrustStep.expression || '');
            statusLineEl.textContent = 'Step expression copied to clipboard.';
        });
        trustDownloadJsonEl.addEventListener('click', () => {
            if (!activeTrustStep?.numeric_trust) return;
            const blob = new Blob([JSON.stringify(activeTrustStep.numeric_trust, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const anchor = document.createElement('a');
            anchor.href = url;
            anchor.download = `trust-step-${activeTrustStep.step}.json`;
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();
            URL.revokeObjectURL(url);
            statusLineEl.textContent = 'Trust metadata downloaded.';
        });
        copyIdentitiesButtonEl.addEventListener('click', copyTrustedIdentities);
        downloadIdentitiesButtonEl.addEventListener('click', downloadTrustedIdentities);
        clearIdentitiesButtonEl.addEventListener('click', clearTrustedIdentities);
        identityListEl.addEventListener('click', (event) => {
            const target = event.target.closest('[data-identity-remove]');
            if (!target) return;
            removeTrustedIdentity(Number(target.dataset.identityRemove));
        });
        phaseMetricEl.addEventListener('change', () => {
            if (lastPayload?.phase_signature) {
                drawPhaseHistory(lastPayload.phase_signature);
            }
        });
        updateModeNote();
        updateDomainHints();
        renderTrustedIdentities();
        runEvaluation();
    </script>
</body>
</html>".to_string()
}

fn render_riemann_hypothesis_lab() -> String {
    "<!doctype html>
<html lang=\"en\">
<head>
    <meta charset=\"utf-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
    <title>Riemann Hypothesis Research Lab</title>
    <style>
        :root {
            --bg: #f2efe8;
            --panel: rgba(255,255,255,0.86);
            --ink: #1d272d;
            --muted: #5a676f;
            --line: rgba(29,39,45,0.13);
            --accent: #0b6f5f;
            --accent-soft: #d8efe8;
            --mono: 'Courier New', monospace;
        }
        * { box-sizing: border-box; }
        body {
            margin: 0;
            color: var(--ink);
            font-family: Georgia, 'Iowan Old Style', 'Palatino Linotype', serif;
            background:
                radial-gradient(circle at top left, rgba(11,111,95,0.14), transparent 34%),
                linear-gradient(180deg, #fbf9f4 0%, var(--bg) 100%);
        }
        main { max-width: 1080px; margin: 0 auto; padding: 30px 18px 48px; }
        .hero, .panel {
            background: var(--panel);
            border: 1px solid var(--line);
            border-radius: 24px;
            box-shadow: 0 18px 44px rgba(29,39,45,0.08);
            backdrop-filter: blur(10px);
        }
        .hero { padding: 26px; margin-bottom: 16px; }
        .eyebrow {
            margin: 0 0 10px;
            color: var(--accent);
            font-family: var(--mono);
            font-size: 12px;
            letter-spacing: 0.14em;
            text-transform: uppercase;
        }
        h1 { margin: 0 0 10px; font-size: clamp(2rem, 4.6vw, 3.4rem); line-height: 0.96; }
        .lede { margin: 0; max-width: 68ch; color: var(--muted); line-height: 1.58; }
        .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
        .panel { padding: 18px; }
        h2 { margin: 0 0 10px; font-size: 1.1rem; }
        .problem { margin-top: 14px; border: 1px solid rgba(11,111,95,0.2); border-radius: 16px; padding: 14px 16px; background: rgba(11,111,95,0.06); }
        .problem p { margin: 8px 0; color: #2f4a53; line-height: 1.56; }
        .prize { margin-top: 12px; border: 1px solid rgba(29,39,45,0.16); border-radius: 16px; padding: 14px 16px; background: rgba(255,255,255,0.74); }
        .prize p { margin: 8px 0; color: #2f4a53; line-height: 1.56; }
        ul { margin: 0; padding-left: 18px; color: var(--muted); line-height: 1.55; }
        li { margin-bottom: 6px; }
        .mono { font-family: var(--mono); font-size: 0.93rem; color: #31434d; }
        .actions { margin-top: 16px; display: flex; flex-wrap: wrap; gap: 10px; }
        a.button {
            text-decoration: none;
            display: inline-block;
            padding: 10px 14px;
            border-radius: 999px;
            border: 1px solid var(--line);
            background: var(--accent-soft);
            color: var(--ink);
            font-family: var(--mono);
            font-size: 0.88rem;
        }
        @media (max-width: 900px) {
            .grid { grid-template-columns: 1fr; }
            main { padding: 22px 14px 40px; }
            .hero, .panel { border-radius: 20px; }
        }
    </style>
</head>
<body>
    <main>
        <section class=\"hero\">
            <p class=\"eyebrow\">Research Lab</p>
            <h1>Riemann Hypothesis Research Lab</h1>
            <p class=\"lede\">This page is a dedicated workspace for hypothesis mapping, theorem-attempt structure, computational validation tracks, and reproducible artifact planning around RH-adjacent investigations.</p>
            <section class=\"problem\">
                <p><strong>Riemann Hypothesis statement:</strong> every non-trivial zero of the Riemann zeta function has real part <span class=\"mono\">Re(s) = 1/2</span>.</p>
                <p><strong>Why it matters:</strong> this condition constrains deep behavior in prime distribution and links analysis, number theory, and spectral structure.</p>
                <p><strong>Our method here:</strong> deterministic trace experiments, explicit witness catalogs, and theorem-check workflows grounded in geometric logic.</p>
            </section>
            <section class=\"prize\">
                <p><strong>Prize offering:</strong> the Riemann Hypothesis is one of the Clay Mathematics Institute Millennium Prize Problems, carrying a <span class=\"mono\">$1,000,000</span> award for a valid resolution.</p>
                <p><strong>What the winner must prove:</strong> either a correct proof that all non-trivial zeta zeros satisfy <span class=\"mono\">Re(s) = 1/2</span>, or a correct disproof by producing and rigorously validating a single non-trivial zero with <span class=\"mono\">Re(s) != 1/2</span>.</p>
                <p><strong>Acceptance standard:</strong> the argument must survive full peer review and broad mathematical verification under the Clay prize rules.</p>
            </section>
            <div class=\"actions\">
                <a class=\"button\" href=\"/\">Back to Landing Page</a>
                <a class=\"button\" href=\"/labs/special-functions\">Open Special Functions Lab</a>
                <a class=\"button\" href=\"/labs/research-findings\">Open Research Findings Hub</a>
            </div>
        </section>

        <section class=\"grid\">
            <article class=\"panel\">
                <h2>Research Tracks</h2>
                <ul>
                    <li>Equivalent-statement map and dependency graph.</li>
                    <li>Candidate-lemma registry with explicit proof obligations.</li>
                    <li>Computational checks for zeta-line and zero statistics.</li>
                    <li>Counterexample search surfaces and failure logging.</li>
                </ul>
            </article>
            <article class=\"panel\">
                <h2>Findings Linked To This Lab</h2>
                <ul>
                    <li><a href=\"/labs/research-findings#logic-framework\">Logic-Geometry Invariants Framework</a></li>
                    <li><a href=\"/labs/research-findings#witness-catalog\">Logic-Geometry Witness Catalog</a></li>
                    <li><a href=\"/labs/research-findings#phase-atlas\">Phase-Transition Atlas and Threshold CIs</a></li>
                    <li><a href=\"/labs/research-findings#asymmetry-isolation\">Asymmetry Isolation Protocol</a></li>
                </ul>
            </article>
        </section>
    </main>
</body>
</html>".to_string()
}

fn render_research_findings_lab() -> String {
    "<!doctype html>
<html lang=\"en\">
<head>
    <meta charset=\"utf-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
    <title>Research Findings Hub</title>
    <style>
        :root {
            --bg: #f4efe5;
            --panel: rgba(255,255,255,0.88);
            --ink: #1d272d;
            --muted: #596870;
            --line: rgba(29,39,45,0.13);
            --accent: #0b6f5f;
            --accent-soft: #d8efe8;
            --mono: 'Courier New', monospace;
        }
        * { box-sizing: border-box; }
        body {
            margin: 0;
            color: var(--ink);
            font-family: Georgia, 'Iowan Old Style', 'Palatino Linotype', serif;
            background:
                radial-gradient(circle at top right, rgba(11,111,95,0.15), transparent 36%),
                linear-gradient(180deg, #fbf9f4 0%, var(--bg) 100%);
        }
        main { max-width: 1100px; margin: 0 auto; padding: 30px 18px 48px; }
        .hero, .card {
            background: var(--panel);
            border: 1px solid var(--line);
            border-radius: 22px;
            box-shadow: 0 18px 42px rgba(29,39,45,0.08);
            backdrop-filter: blur(10px);
        }
        .hero { padding: 24px; margin-bottom: 14px; }
        .eyebrow { margin: 0 0 10px; color: var(--accent); font-family: var(--mono); font-size: 12px; letter-spacing: 0.14em; text-transform: uppercase; }
        h1 { margin: 0 0 8px; font-size: clamp(2rem, 4.8vw, 3.2rem); line-height: 0.96; }
        .lede { margin: 0; max-width: 68ch; color: var(--muted); line-height: 1.58; }
        .actions { margin-top: 14px; display: flex; flex-wrap: wrap; gap: 10px; }
        a.button {
            text-decoration: none;
            display: inline-block;
            padding: 10px 14px;
            border-radius: 999px;
            border: 1px solid var(--line);
            background: var(--accent-soft);
            color: var(--ink);
            font-family: var(--mono);
            font-size: 0.88rem;
        }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 14px; }
        .card { padding: 16px; }
        h2 { margin: 0 0 8px; font-size: 1.12rem; }
        p { margin: 0 0 8px; color: #344952; line-height: 1.56; }
        .path { font-family: var(--mono); font-size: 0.86rem; color: #36505a; background: rgba(216,239,232,0.5); border: 1px solid rgba(11,111,95,0.2); border-radius: 10px; padding: 8px 10px; }
        @media (max-width: 900px) { main { padding: 22px 14px 40px; } }
    </style>
</head>
<body>
    <main>
        <section class=\"hero\">
            <p class=\"eyebrow\">Research Findings</p>
            <h1>Geometric Logic Findings Hub</h1>
            <p class=\"lede\">This hub links the active findings that inform our Riemann-Hypothesis-oriented workflow: deterministic invariants, witness catalogs, phase transition maps, and asymmetry isolation experiments.</p>
            <div class=\"actions\">
                <a class=\"button\" href=\"/labs/riemann-hypothesis\">Back to RH Research Lab</a>
                <a class=\"button\" href=\"/\">Back to Landing Page</a>
            </div>
        </section>

        <section class=\"grid\">
            <article class=\"card\" id=\"logic-framework\">
                <h2>Logic-Geometry Invariants Framework</h2>
                <p>Formalizes Path Signature Invariant (tree/order-sensitive) and Endpoint Invariant (value-sensitive), giving testable theorem candidates T1/T2/T3.</p>
                <div class=\"path\">docs/findings/LOGIC_GEOMETRY_INVARIANTS_FRAMEWORK.md</div>
            </article>
            <article class=\"card\" id=\"witness-catalog\">
                <h2>Witness Catalog (First Experimental Pass)</h2>
                <p>Machine-generated witness pairs showing deterministic PSI/EI behavior, including path-distinct/value-equal families aligned with the geometric-logic thesis.</p>
                <div class=\"path\">docs/findings/LOGIC_GEOMETRY_WITNESS_CATALOG_NOTE.md</div>
                <div class=\"path\">docs/findings/artifacts/logic_geometry_witness_report.json</div>
            </article>
            <article class=\"card\" id=\"phase-atlas\">
                <h2>Phase-Transition Atlas</h2>
                <p>Maps stability regimes and estimates coherence cliff thresholds with confidence bands across dimensions.</p>
                <div class=\"path\">docs/findings/HAFNIAN_FLUX_PHASE_TRANSITION_ATLAS_NOTE.md</div>
                <div class=\"path\">docs/findings/artifacts/hafnian_flux_transition_thresholds.json</div>
            </article>
            <article class=\"card\" id=\"asymmetry-isolation\">
                <h2>Asymmetry Isolation Protocol</h2>
                <p>Separates symmetry-gap perturbations from coherence regimes to test causality claims in a controlled sweep.</p>
                <div class=\"path\">docs/findings/HAFNIAN_FLUX_PROBE_ASYMMETRY_ISOLATION_NOTE.md</div>
                <div class=\"path\">docs/findings/artifacts/hafnian_flux_asymmetry_sweep_summary.json</div>
            </article>
            <article class=\"card\" id=\"rh-proof-program\">
                <h2>RH Proof Program Status</h2>
                <p>Tracks objective proof obligations and whether the current pipeline is prize-ready, based on concrete repository artifacts.</p>
                <div class=\"path\">docs/findings/RH_PROOF_PROGRAM_V0_1.md</div>
                <div class=\"path\">docs/findings/artifacts/rh_proof_pipeline_status.json</div>
            </article>
        </section>
    </main>
</body>
</html>".to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{ApiState, build_router};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use digitalcrystal_engine::{AppConfig, RWIF_EVENT_SCHEMA_VERSION, RWIF_SCHEMA_VERSION};
    use serde_json::{Value, json};
    use tower::util::ServiceExt;

    async fn eval_math_payload(expression: &str) -> Value {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/csif/math")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "expression": expression,
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
        serde_json::from_slice(&body).expect("json payload should parse")
    }

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
    async fn index_returns_platform_landing_page() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let html = String::from_utf8(body.to_vec()).expect("body should be utf-8");
        assert!(html.contains("DigitalCrystal"));
        assert!(html.contains("Special Functions Lab"));
        assert!(html.contains("/labs/special-functions"));
        assert!(html.contains("/labs/riemann-hypothesis"));
        assert!(html.contains("Riemann Hypothesis Lab"));
        assert!(html.contains("research track"));
    }

    #[tokio::test]
    async fn special_functions_lab_page_is_served() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/labs/special-functions")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let html = String::from_utf8(body.to_vec()).expect("body should be utf-8");
        assert!(html.contains("Special Functions Lab"));
        assert!(html.contains("/v1/csif/math"));
        assert!(html.contains("Download Raw JSON"));
        assert!(html.contains("phaseArc"));
        assert!(html.contains("phaseHistory"));
        assert!(html.contains("phaseMetric"));
        assert!(html.contains("phaseSteps"));
        assert!(html.contains("rawPanel"));
    }

    #[tokio::test]
    async fn riemann_hypothesis_lab_page_is_served() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/labs/riemann-hypothesis")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let html = String::from_utf8(body.to_vec()).expect("body should be utf-8");
        assert!(html.contains("Riemann Hypothesis Research Lab"));
        assert!(html.contains("Re(s) = 1/2"));
        assert!(html.contains("$1,000,000"));
        assert!(html.contains("Clay Mathematics Institute Millennium Prize Problems"));
        assert!(html.contains("/labs/special-functions"));
        assert!(html.contains("/labs/research-findings"));
    }

    #[tokio::test]
    async fn research_findings_lab_page_is_served() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/labs/research-findings")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let html = String::from_utf8(body.to_vec()).expect("body should be utf-8");
        assert!(html.contains("Geometric Logic Findings Hub"));
        assert!(html.contains("LOGIC_GEOMETRY_WITNESS_CATALOG_NOTE"));
        assert!(html.contains("RH Proof Program Status"));
        assert!(html.contains("rh_proof_pipeline_status.json"));
        assert!(html.contains("/labs/riemann-hypothesis"));
    }

    #[tokio::test]
    async fn platform_modules_endpoint_returns_catalog() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/platform/modules")
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
        assert_eq!(payload.get("platform_id"), Some(&Value::String("digitalcrystal".to_string())));
        assert!(payload
            .get("modules")
            .and_then(Value::as_array)
            .map(|modules| modules.iter().any(|module| module.get("module_id") == Some(&Value::String("special-functions".to_string()))))
            .unwrap_or(false));
    }

    #[tokio::test]
    async fn csif_math_returns_deterministic_math_payload() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/csif/math")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "expression": "2*(3+4)^2",
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
        assert_eq!(payload.get("object"), Some(&Value::String("csif.math.result".to_string())));
        assert_eq!(payload.get("result"), Some(&json!(98.0)));
        assert_eq!(payload.get("engine"), Some(&Value::String("digitalcrystal_math_v2".to_string())));
        assert!(payload.get("phase_signature").is_some());
        assert!(payload.get("path_signature").and_then(Value::as_str).is_some());
        assert!(payload.get("endpoint_signature").and_then(Value::as_str).is_some());
        assert!(payload.get("rwif_export").is_some());
        assert!(payload.get("bridge_audit").is_some());
    }

    #[tokio::test]
    async fn t1_conformance_uses_path_signature_for_constraint_distinguishability() {
        let lhs = eval_math_payload("(2 + 3) + 4").await;
        let rhs = eval_math_payload("2 + (3 + 4)").await;

        assert_ne!(
            lhs.get("path_signature").and_then(Value::as_str),
            rhs.get("path_signature").and_then(Value::as_str)
        );
    }

    #[tokio::test]
    async fn t2_conformance_explicit_signatures_are_stable_across_runs() {
        let expression = "(1 * 3) + (2 * 3)";
        let mut path_signatures = HashSet::new();
        let mut endpoint_signatures = HashSet::new();

        for _ in 0..5 {
            let payload = eval_math_payload(expression).await;
            path_signatures.insert(
                payload
                    .get("path_signature")
                    .and_then(Value::as_str)
                    .expect("path signature should be present")
                    .to_string(),
            );
            endpoint_signatures.insert(
                payload
                    .get("endpoint_signature")
                    .and_then(Value::as_str)
                    .expect("endpoint signature should be present")
                    .to_string(),
            );
        }

        assert_eq!(path_signatures.len(), 1);
        assert_eq!(endpoint_signatures.len(), 1);
    }

    #[tokio::test]
    async fn t3_conformance_witness_uses_explicit_signatures() {
        let lhs = eval_math_payload("(2 * 3) * 4").await;
        let rhs = eval_math_payload("2 * (3 * 4)").await;

        assert_eq!(lhs.get("result"), rhs.get("result"));
        assert_ne!(
            lhs.get("path_signature").and_then(Value::as_str),
            rhs.get("path_signature").and_then(Value::as_str)
        );
        assert_ne!(
            lhs.get("endpoint_signature").and_then(Value::as_str),
            rhs.get("endpoint_signature").and_then(Value::as_str)
        );
    }

    #[tokio::test]
    async fn csif_math_rejects_invalid_mode() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/csif/math")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "expression": "1+1",
                            "mode": "bad",
                        }))
                        .expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload: Value = serde_json::from_slice(&body).expect("json payload should parse");
        assert_eq!(
            payload
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(Value::as_str),
            Some("MATH_PARSE_ERROR")
        );
        assert!(payload.get("bridge_audit").is_some());
    }

    #[tokio::test]
    async fn csif_math_supports_complex_results() {
        let app = build_router(ApiState {
            config: std::sync::Arc::new(AppConfig::default()),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/csif/math")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "expression": "conj(2+3i) + arg(1+i)",
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
        assert!(payload
            .get("result")
            .and_then(Value::as_object)
            .map(|value| value.contains_key("re") && value.contains_key("im"))
            .unwrap_or(false));
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