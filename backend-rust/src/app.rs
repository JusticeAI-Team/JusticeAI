use std::sync::Arc;

use axum::{
    http::{HeaderValue, Method, StatusCode},
    Router,
};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::api;
use crate::config_loader::load_config;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub app: AppConfig,
    pub database: DatabaseConfig,
    pub hugegraph: HugeGraphConfig,
    pub milvus: MilvusConfig,
    pub vllm: VllmConfig,
    pub storage: StorageConfig,
}

impl Settings {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config = load_config()?;
        config.try_deserialize()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub env: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HugeGraphConfig {
    pub base_url: String,
    pub gremlin_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MilvusConfig {
    pub address: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VllmConfig {
    pub base_url: String,
    pub model_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub upload_dir: String,
    pub report_dir: String,
    pub training_dir: String,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    settings: Settings,
    db: PgPool,
    http_client: Client,
}

impl AppState {
    pub fn new(settings: Settings, db: PgPool) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("构建 HTTP 客户端失败");

        Self {
            inner: Arc::new(AppStateInner {
                settings,
                db,
                http_client,
            }),
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.inner.settings
    }

    pub fn db(&self) -> &PgPool {
        &self.inner.db
    }

    pub fn http_client(&self) -> &Client {
        &self.inner.http_client
    }
}

pub fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(HeaderValue::from_static("*"))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .nest("/api", api::routes())
        .with_state(state)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(30),
        ))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
