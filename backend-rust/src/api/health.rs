use axum::{extract::State, routing::get, Router};
use chrono::Utc;
use serde::Serialize;

use crate::{
    app::AppState,
    shared::response::{ok, ApiResponse},
};

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
    dependency_details: Vec<DependencyDetail>,
    storage: Vec<StorageStatus>,
    data_overview: DataOverview,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DependencyStatuses {
    postgres: &'static str,
    hugegraph: &'static str,
    vllm: &'static str,
    milvus: &'static str,
}

#[derive(Debug, Serialize)]
struct DependencyDetail {
    key: &'static str,
    label: &'static str,
    status: &'static str,
    endpoint: String,
    checked_at: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct StorageStatus {
    key: &'static str,
    path: String,
    exists: bool,
    writable: bool,
}

#[derive(Debug, Serialize)]
struct DataOverview {
    import_batches: i64,
    risk_cases: i64,
    alerts: i64,
    reports: i64,
}

async fn health(State(state): State<AppState>) -> axum::Json<ApiResponse<HealthResponse>> {
    let checked_at = Utc::now().to_rfc3339();
    let postgres_up = check_postgres(state.db()).await;
    let hugegraph_up = check_http_endpoint(
        &state,
        format!("{}/", state.settings().hugegraph.base_url.trim_end_matches('/')),
    )
    .await;
    let vllm_up = check_http_endpoint(
        &state,
        format!("{}/models", state.settings().vllm.base_url.trim_end_matches('/')),
    )
    .await;
    let milvus_status = if state.settings().milvus.address.trim().is_empty() {
        "not_configured"
    } else {
        "not_checked"
    };

    let postgres = if postgres_up { "up" } else { "down" };
    let hugegraph = if hugegraph_up { "up" } else { "down" };
    let vllm = if vllm_up { "up" } else { "down" };

    let status = if postgres_up && hugegraph_up && vllm_up {
        "ok"
    } else if postgres_up {
        "degraded"
    } else {
        "critical"
    };

    let data_overview = DataOverview {
        import_batches: scalar_count(state.db(), "SELECT COUNT(*) FROM imports").await,
        risk_cases: scalar_count(state.db(), "SELECT COUNT(*) FROM risk_cases").await,
        alerts: scalar_count(state.db(), "SELECT COUNT(*) FROM alerts").await,
        reports: scalar_count(state.db(), "SELECT COUNT(*) FROM generated_reports").await,
    };

    ok(HealthResponse {
        status,
        service: "justiceai-backend",
        app_env: state.settings().app.env.clone(),
        timestamp: checked_at.clone(),
        dependencies: DependencyStatuses {
            postgres,
            hugegraph,
            vllm,
            milvus: milvus_status,
        },
        dependency_details: vec![
            DependencyDetail {
                key: "postgres",
                label: "PostgreSQL",
                status: postgres,
                endpoint: state.settings().database.url.clone(),
                checked_at: checked_at.clone(),
                message: "database connectivity is required for all platform modules".to_string(),
            },
            DependencyDetail {
                key: "hugegraph",
                label: "HugeGraph",
                status: hugegraph,
                endpoint: state.settings().hugegraph.base_url.clone(),
                checked_at: checked_at.clone(),
                message: "graph sync is reserved through the HTTP endpoint".to_string(),
            },
            DependencyDetail {
                key: "vllm",
                label: "vLLM/OpenAI-Compatible",
                status: vllm,
                endpoint: state.settings().vllm.base_url.clone(),
                checked_at: checked_at.clone(),
                message: "model integration is expected to use an OpenAI-compatible ChatCompletion endpoint".to_string(),
            },
            DependencyDetail {
                key: "milvus",
                label: "Milvus",
                status: milvus_status,
                endpoint: state.settings().milvus.address.clone(),
                checked_at: checked_at.clone(),
                message: "vector search remains a reserved integration point".to_string(),
            },
        ],
        storage: vec![
            storage_status("upload_dir", &state.settings().storage.upload_dir),
            storage_status("report_dir", &state.settings().storage.report_dir),
            storage_status("training_dir", &state.settings().storage.training_dir),
        ],
        data_overview,
        notes: vec![
            "Health is intentionally structured for the platform readiness screen.".to_string(),
            "Milvus probing remains placeholder until a concrete client is integrated.".to_string(),
        ],
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

async fn scalar_count(db: &sqlx::PgPool, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(db)
        .await
        .unwrap_or(0)
}

fn storage_status(key: &'static str, path: &str) -> StorageStatus {
    let path_ref = std::path::Path::new(path);
    let exists = path_ref.exists();
    let writable = exists && path_ref.is_dir();

    StorageStatus {
        key,
        path: path.to_string(),
        exists,
        writable,
    }
}
