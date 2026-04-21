use std::collections::HashMap;

use axum::{extract::State, routing::get, Router};
use chrono::Utc;
use serde::Serialize;

use crate::{
    app::AppState,
    shared::response::{ok, ApiResponse},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/system/info", get(system_info))
}

#[derive(Debug, Serialize)]
struct SystemInfoResponse {
    app: AppInfo,
    database: DatabaseInfo,
    hugegraph: HugeGraphInfo,
    milvus: MilvusInfo,
    vllm: VllmInfo,
    storage: StorageInfo,
    runtime: RuntimeInfo,
    platform_settings: PlatformSettingsSnapshot,
    integrations: Vec<IntegrationSnapshot>,
    data_overview: DataOverview,
}

#[derive(Debug, Serialize)]
struct AppInfo {
    name: String,
    env: String,
    host: String,
    port: u16,
}

#[derive(Debug, Serialize)]
struct DatabaseInfo {
    max_connections: u32,
    acquire_timeout_secs: u64,
}

#[derive(Debug, Serialize)]
struct HugeGraphInfo {
    base_url: String,
    gremlin_url: String,
}

#[derive(Debug, Serialize)]
struct MilvusInfo {
    address: String,
}

#[derive(Debug, Serialize)]
struct VllmInfo {
    base_url: String,
    model_name: String,
    request_style: &'static str,
    chat_endpoint: &'static str,
}

#[derive(Debug, Serialize)]
struct StorageInfo {
    upload_dir: String,
    report_dir: String,
    training_dir: String,
}

#[derive(Debug, Serialize)]
struct RuntimeInfo {
    version: &'static str,
    package_name: &'static str,
    generated_at: String,
}

#[derive(Debug, Serialize)]
struct PlatformSettingsSnapshot {
    platform_name: String,
    environment: String,
    upload_dir: String,
    report_dir: String,
    training_dir: String,
}

#[derive(Debug, Serialize)]
struct IntegrationSnapshot {
    key: String,
    endpoint: String,
    configured: bool,
}

#[derive(Debug, Serialize)]
struct DataOverview {
    import_batches: i64,
    risk_cases: i64,
    alerts: i64,
    dispatch_tasks: i64,
    reports: i64,
    extraction_runs: i64,
}

async fn system_info(State(state): State<AppState>) -> axum::Json<ApiResponse<SystemInfoResponse>> {
    let settings = state.settings();
    let platform_values = load_settings_map(state.db(), "platform").await;
    let integration_values = load_settings_map(state.db(), "integrations").await;

    ok(SystemInfoResponse {
        app: AppInfo {
            name: settings.app.name.clone(),
            env: settings.app.env.clone(),
            host: settings.app.host.clone(),
            port: settings.app.port,
        },
        database: DatabaseInfo {
            max_connections: settings.database.max_connections,
            acquire_timeout_secs: settings.database.acquire_timeout_secs,
        },
        hugegraph: HugeGraphInfo {
            base_url: integration_values
                .get("hugegraph_base_url")
                .cloned()
                .unwrap_or_else(|| settings.hugegraph.base_url.clone()),
            gremlin_url: integration_values
                .get("hugegraph_gremlin_url")
                .cloned()
                .unwrap_or_else(|| settings.hugegraph.gremlin_url.clone()),
        },
        milvus: MilvusInfo {
            address: integration_values
                .get("milvus_address")
                .cloned()
                .unwrap_or_else(|| settings.milvus.address.clone()),
        },
        vllm: VllmInfo {
            base_url: integration_values
                .get("model_base_url")
                .cloned()
                .unwrap_or_else(|| settings.vllm.base_url.clone()),
            model_name: integration_values
                .get("model_name")
                .cloned()
                .unwrap_or_else(|| settings.vllm.model_name.clone()),
            request_style: "openai_chat_completion_compatible",
            chat_endpoint: "/v1/chat/completions",
        },
        storage: StorageInfo {
            upload_dir: platform_values
                .get("upload_dir")
                .cloned()
                .unwrap_or_else(|| settings.storage.upload_dir.clone()),
            report_dir: platform_values
                .get("report_dir")
                .cloned()
                .unwrap_or_else(|| settings.storage.report_dir.clone()),
            training_dir: platform_values
                .get("training_dir")
                .cloned()
                .unwrap_or_else(|| settings.storage.training_dir.clone()),
        },
        runtime: RuntimeInfo {
            version: env!("CARGO_PKG_VERSION"),
            package_name: env!("CARGO_PKG_NAME"),
            generated_at: Utc::now().to_rfc3339(),
        },
        platform_settings: PlatformSettingsSnapshot {
            platform_name: platform_values
                .get("platform_name")
                .cloned()
                .unwrap_or_else(|| settings.app.name.clone()),
            environment: platform_values
                .get("environment")
                .cloned()
                .unwrap_or_else(|| settings.app.env.clone()),
            upload_dir: platform_values
                .get("upload_dir")
                .cloned()
                .unwrap_or_else(|| settings.storage.upload_dir.clone()),
            report_dir: platform_values
                .get("report_dir")
                .cloned()
                .unwrap_or_else(|| settings.storage.report_dir.clone()),
            training_dir: platform_values
                .get("training_dir")
                .cloned()
                .unwrap_or_else(|| settings.storage.training_dir.clone()),
        },
        integrations: vec![
            integration_snapshot(
                "postgres",
                settings.database.url.clone(),
                !settings.database.url.trim().is_empty(),
            ),
            integration_snapshot(
                "hugegraph",
                integration_values
                    .get("hugegraph_base_url")
                    .cloned()
                    .unwrap_or_else(|| settings.hugegraph.base_url.clone()),
                !(integration_values
                    .get("hugegraph_base_url")
                    .cloned()
                    .unwrap_or_else(|| settings.hugegraph.base_url.clone()))
                .trim()
                .is_empty(),
            ),
            integration_snapshot(
                "milvus",
                integration_values
                    .get("milvus_address")
                    .cloned()
                    .unwrap_or_else(|| settings.milvus.address.clone()),
                !(integration_values
                    .get("milvus_address")
                    .cloned()
                    .unwrap_or_else(|| settings.milvus.address.clone()))
                .trim()
                .is_empty(),
            ),
            integration_snapshot(
                "model_service",
                integration_values
                    .get("model_base_url")
                    .cloned()
                    .unwrap_or_else(|| settings.vllm.base_url.clone()),
                !(integration_values
                    .get("model_base_url")
                    .cloned()
                    .unwrap_or_else(|| settings.vllm.base_url.clone()))
                .trim()
                .is_empty(),
            ),
        ],
        data_overview: DataOverview {
            import_batches: scalar_count(state.db(), "SELECT COUNT(*) FROM imports").await,
            risk_cases: scalar_count(state.db(), "SELECT COUNT(*) FROM risk_cases").await,
            alerts: scalar_count(state.db(), "SELECT COUNT(*) FROM alerts").await,
            dispatch_tasks: scalar_count(state.db(), "SELECT COUNT(*) FROM dispatch_tasks").await,
            reports: scalar_count(state.db(), "SELECT COUNT(*) FROM generated_reports").await,
            extraction_runs: scalar_count(state.db(), "SELECT COUNT(*) FROM extraction_runs").await,
        },
    })
}

fn integration_snapshot(key: &str, endpoint: String, configured: bool) -> IntegrationSnapshot {
    IntegrationSnapshot {
        key: key.to_string(),
        endpoint,
        configured,
    }
}

async fn load_settings_map(db: &sqlx::PgPool, category: &str) -> HashMap<String, String> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT setting_key, setting_value FROM platform_settings WHERE category = $1",
    )
    .bind(category)
    .fetch_all(db)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect()
}

async fn scalar_count(db: &sqlx::PgPool, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(db)
        .await
        .unwrap_or(0)
}
