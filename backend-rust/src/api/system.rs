use axum::{extract::State, routing::get, Router};
use serde::Serialize;

use crate::{app::AppState, shared::response::{ok, ApiResponse}};

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
}

async fn system_info(State(state): State<AppState>) -> axum::Json<ApiResponse<SystemInfoResponse>> {
    let settings = state.settings();

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
            base_url: settings.hugegraph.base_url.clone(),
            gremlin_url: settings.hugegraph.gremlin_url.clone(),
        },
        milvus: MilvusInfo {
            address: settings.milvus.address.clone(),
        },
        vllm: VllmInfo {
            base_url: settings.vllm.base_url.clone(),
            model_name: settings.vllm.model_name.clone(),
        },
        storage: StorageInfo {
            upload_dir: settings.storage.upload_dir.clone(),
            report_dir: settings.storage.report_dir.clone(),
            training_dir: settings.storage.training_dir.clone(),
        },
        runtime: RuntimeInfo { version: "0.1.0" },
    })
}
