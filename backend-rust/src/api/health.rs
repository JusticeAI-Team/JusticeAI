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
    ops_signals: OpsSignals,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DependencyStatuses {
    postgres: &'static str,
    hugegraph: &'static str,
    vllm: &'static str,
    embedding: &'static str,
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

#[derive(Debug, Serialize)]
struct OpsSignals {
    failed_platform_jobs: i64,
    recent_ai_standardization_failures: i64,
    recent_material_upload_failures: i64,
}

async fn health(State(state): State<AppState>) -> axum::Json<ApiResponse<HealthResponse>> {
    let checked_at = Utc::now().to_rfc3339();
    let postgres_up = check_postgres(state.db()).await;
    let hugegraph_up = check_http_endpoint(
        &state,
        format!(
            "{}/",
            state.settings().hugegraph.base_url.trim_end_matches('/')
        ),
    )
    .await;
    let vllm_up = check_http_endpoint(
        &state,
        format!(
            "{}/models",
            state.settings().vllm.base_url.trim_end_matches('/')
        ),
    )
    .await;
    let embedding_up = check_embedding_endpoint(&state).await;
    let milvus_up = check_milvus_endpoint(&state).await;
    let milvus_status = if state.settings().milvus.address.trim().is_empty() {
        "not_configured"
    } else if milvus_up {
        "up"
    } else {
        "down"
    };

    let postgres = if postgres_up { "up" } else { "down" };
    let hugegraph = if hugegraph_up { "up" } else { "down" };
    let vllm = if vllm_up { "up" } else { "down" };
    let embedding = if state.settings().embedding.base_url.trim().is_empty()
        || state.settings().embedding.model_name.trim().is_empty()
    {
        "not_configured"
    } else if embedding_up {
        "up"
    } else {
        "down"
    };

    let status = if postgres_up && hugegraph_up && vllm_up && embedding_up && milvus_up {
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
    let ops_signals = OpsSignals {
        failed_platform_jobs: scalar_count(
            state.db(),
            "SELECT COUNT(*) FROM platform_jobs WHERE status = 'failed'",
        )
        .await,
        recent_ai_standardization_failures: scalar_count(
            state.db(),
            "SELECT COUNT(*) FROM appeal_standardizations WHERE status = 'failed' OR error_message IS NOT NULL",
        )
        .await,
        recent_material_upload_failures: scalar_count(
            state.db(),
            "SELECT COUNT(*) FROM platform_jobs WHERE job_type = 'appeal_material_upload' AND status = 'failed'",
        )
        .await,
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
            embedding,
            milvus: milvus_status,
        },
        dependency_details: vec![
            DependencyDetail {
                key: "postgres",
                label: "PostgreSQL",
                status: postgres,
                endpoint: redact_database_url(&state.settings().database.url),
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
                key: "embedding",
                label: "Embedding/OpenAI-Compatible",
                status: embedding,
                endpoint: state.settings().embedding.base_url.clone(),
                checked_at: checked_at.clone(),
                message: "embedding integration is expected to use an OpenAI-compatible /embeddings endpoint".to_string(),
            },
            DependencyDetail {
                key: "milvus",
                label: "Milvus",
                status: milvus_status,
                endpoint: state.settings().milvus.address.clone(),
                checked_at: checked_at.clone(),
                message: "Milvus vector store is probed through the REST v2 collection endpoint".to_string(),
            },
        ],
        storage: vec![
            storage_status("upload_dir", &state.settings().storage.upload_dir),
            storage_status("report_dir", &state.settings().storage.report_dir),
            storage_status("training_dir", &state.settings().storage.training_dir),
        ],
        data_overview,
        ops_signals,
        notes: vec![
            "Health is intentionally structured for the platform readiness screen.".to_string(),
            "Milvus and Embedding are actively probed so the ops console can show real vector-chain readiness.".to_string(),
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

async fn check_embedding_endpoint(state: &AppState) -> bool {
    if state.settings().embedding.base_url.trim().is_empty()
        || state.settings().embedding.model_name.trim().is_empty()
    {
        return false;
    }

    let endpoint = if state.settings().embedding.endpoint.starts_with('/') {
        state.settings().embedding.endpoint.clone()
    } else {
        format!("/{}", state.settings().embedding.endpoint)
    };
    let url = format!(
        "{}{}",
        state.settings().embedding.base_url.trim_end_matches('/'),
        endpoint
    );
    let payload = serde_json::json!({
        "model": state.settings().embedding.model_name,
        "input": "JusticeAI health check"
    });
    let mut request = state.http_client().post(url).json(&payload);
    if !state.settings().embedding.api_key.trim().is_empty() {
        request = request.bearer_auth(state.settings().embedding.api_key.trim());
    }

    request
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

async fn check_milvus_endpoint(state: &AppState) -> bool {
    if state.settings().milvus.address.trim().is_empty() {
        return false;
    }

    let url = format!(
        "{}/v2/vectordb/collections/list",
        state.settings().milvus.address.trim_end_matches('/')
    );
    let mut request = state.http_client().post(url).json(&serde_json::json!({}));
    if let Some(token) = resolve_milvus_token(state) {
        request = request.bearer_auth(token);
    }

    request
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

fn resolve_milvus_token(state: &AppState) -> Option<String> {
    if !state.settings().milvus.token.trim().is_empty() {
        Some(state.settings().milvus.token.trim().to_string())
    } else if !state.settings().milvus.username.trim().is_empty()
        && !state.settings().milvus.password.trim().is_empty()
    {
        Some(format!(
            "{}:{}",
            state.settings().milvus.username.trim(),
            state.settings().milvus.password.trim()
        ))
    } else {
        None
    }
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

fn redact_database_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let Some((credentials, host)) = rest.split_once('@') else {
        return url.to_string();
    };
    let user = credentials.split(':').next().unwrap_or("user");
    format!("{scheme}://{user}:***@{host}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_url_redaction_hides_password() {
        let redacted = redact_database_url("postgres://justiceai:secret@postgres:5432/justiceai");
        assert_eq!(redacted, "postgres://justiceai:***@postgres:5432/justiceai");
        assert!(!redacted.contains("secret"));
    }
}
