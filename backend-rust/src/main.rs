mod api;
mod app;
mod bootstrap;
mod config_loader;
mod domain;
mod services;
mod shared;

use std::{net::SocketAddr, path::Path, time::Duration};

use anyhow::Context;
use app::{build_app, AppState, Settings};
use bootstrap::initialize_workspace_schema;
use tokio::{net::TcpListener, signal};
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let settings = Settings::load().context("加载 JusticeAI 后端配置失败")?;

    ensure_runtime_directories(&settings).context("初始化运行目录失败")?;

    info!(
        app_name = %settings.app.name,
        app_env = %settings.app.env,
        host = %settings.app.host,
        port = settings.app.port,
        "JusticeAI 后端启动中"
    );

    let database_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .acquire_timeout(Duration::from_secs(settings.database.acquire_timeout_secs))
        .connect(&settings.database.url)
        .await
        .context("连接 PostgreSQL 失败")?;

    info!("PostgreSQL 连接成功");

    initialize_workspace_schema(&database_pool)
        .await
        .context("初始化工作台数据结构失败")?;

    info!("工作台基础数据结构已就绪");

    let state = AppState::new(settings.clone(), database_pool);
    let app = build_app(state);

    let addr: SocketAddr = format!("{}:{}", settings.app.host, settings.app.port)
        .parse()
        .context("解析监听地址失败")?;

    let listener = TcpListener::bind(addr).await.context("绑定监听地址失败")?;

    info!(listen = %addr, "JusticeAI 后端已启动");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP 服务运行异常")?;

    info!("JusticeAI 后端已安全关闭");
    Ok(())
}

fn ensure_runtime_directories(settings: &Settings) -> anyhow::Result<()> {
    create_dir_if_missing(&settings.storage.upload_dir)
        .with_context(|| format!("创建上传目录失败: {}", settings.storage.upload_dir))?;

    create_dir_if_missing(&settings.storage.report_dir)
        .with_context(|| format!("创建报告目录失败: {}", settings.storage.report_dir))?;

    create_dir_if_missing(&settings.storage.training_dir)
        .with_context(|| format!("创建训练目录失败: {}", settings.storage.training_dir))?;

    info!(path = %settings.storage.upload_dir, "上传目录已就绪");
    info!(path = %settings.storage.report_dir, "报告目录已就绪");
    info!(path = %settings.storage.training_dir, "训练目录已就绪");

    Ok(())
}

fn create_dir_if_missing(path: &str) -> anyhow::Result<()> {
    let path_ref = Path::new(path);

    if !path_ref.exists() {
        std::fs::create_dir_all(path_ref)?;
    }

    Ok(())
}

fn init_tracing() {
    let env_filter =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn,tower_http=info".to_string());

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .compact()
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            error!(error = %err, "监听 Ctrl+C 失败");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};

        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                error!(error = %err, "监听 SIGTERM 失败");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("收到退出信号，开始执行优雅关闭");
}
