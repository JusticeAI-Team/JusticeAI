use axum::{extract::State, routing::get, Router};
use chrono::Utc;
use serde::Serialize;

use crate::{app::AppState, shared::response::{ok, ApiResponse}};

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    app_env: String,
    timestamp: String,
    dependencies: DependencyStatuses,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DependencyStatuses {
    postgres: &'static str,
    hugegraph: &'static str,
    vllm: &'static str,
    milvus: &'static str,
}

async fn health(State(state): State<AppState>) -> axum::Json<ApiResponse<HealthResponse>> {
    let postgres = if check_postgres(state.db()).await {
        "up"
    } else {
        "down"
    };
    let hugegraph = if check_http_endpoint(
        &state,
        format!("{}/", state.settings().hugegraph.base_url.trim_end_matches('/')),
    )
    .await
    {
        "up"
    } else {
        "down"
    };
    let vllm = if check_http_endpoint(
        &state,
        format!("{}/models", state.settings().vllm.base_url.trim_end_matches('/')),
    )
    .await
    {
        "up"
    } else {
        "down"
    };
    let milvus = "not_checked";

    let status = if postgres == "up" && hugegraph == "up" && vllm == "up" {
        "ok"
    } else {
        "degraded"
    };

    ok(HealthResponse {
        status,
        service: "justiceai-backend",
        app_env: state.settings().app.env.clone(),
        timestamp: Utc::now().to_rfc3339(),
        dependencies: DependencyStatuses {
            postgres,
            hugegraph,
            vllm,
            milvus,
        },
        notes: vec!["Milvus 健康探测将在后续版本接入".to_string()],
    })
}

async fn check_postgres(db: &sqlx::PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(db)
        .await
        .is_ok()
}

async fn check_http_endpoint(state: &AppState, url: String) -> bool {
    state
        .http_client()
        .get(url)
        .send()
        .await
        .map(|response| {
            response.status().is_success()
                || response.status().is_redirection()
                || response.status().as_u16() == 401
                || response.status().as_u16() == 404
        })
        .unwrap_or(false)
}
