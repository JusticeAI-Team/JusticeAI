use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    app::AppState,
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/imports", get(list_imports))
        .route("/imports/:id", get(get_import_detail))
}

#[derive(Debug, Deserialize)]
struct ImportListQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Clone)]
struct NormalizedImportListQuery {
    page: i64,
    page_size: i64,
    offset: i64,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct ImportListResponse {
    items: Vec<ImportListItem>,
    page: i64,
    page_size: i64,
    total: i64,
}

#[derive(Debug, Serialize, FromRow)]
struct ImportListItem {
    id: Uuid,
    source_type: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ImportDetailResponse {
    id: Uuid,
    source_type: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    files: Vec<ImportFileItem>,
}

#[derive(Debug, Serialize, FromRow)]
struct ImportDetailRow {
    id: Uuid,
    source_type: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct ImportFileItem {
    id: Uuid,
    original_filename: String,
    stored_filename: String,
    stored_path: String,
    file_size: i64,
    mime_type: Option<String>,
    created_at: DateTime<Utc>,
}

fn normalize_import_list_query(query: ImportListQuery) -> NormalizedImportListQuery {
    let page = query.page.unwrap_or(1).max(1);

    let raw_page_size = query.page_size.unwrap_or(20);
    let page_size = if raw_page_size < 1 {
        20
    } else {
        raw_page_size.min(100)
    };

    let offset = page.saturating_sub(1).saturating_mul(page_size);
    let status = query
        .status
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    NormalizedImportListQuery {
        page,
        page_size,
        offset,
        status,
    }
}

async fn list_imports(
    State(state): State<AppState>,
    Query(query): Query<ImportListQuery>,
) -> Result<axum::Json<ApiResponse<ImportListResponse>>, AppError> {
    let query = normalize_import_list_query(query);
    let items = query_import_list(state.db(), &query).await?;

    Ok(ok(ImportListResponse {
        items,
        page: query.page,
        page_size: query.page_size,
        total: 0,
    }))
}

async fn query_import_list(
    db: &sqlx::PgPool,
    query: &NormalizedImportListQuery,
) -> Result<Vec<ImportListItem>, AppError> {
    let _offset = query.offset;
    let _status = query.status.as_deref();
    // Task 1 仅完成参数归一化骨架，分页下推与 status SQL 过滤留给后续任务实现。

    sqlx::query_as::<_, ImportListItem>(
        r#"
        SELECT id, source_type, status, created_at, updated_at
        FROM imports
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

async fn get_import_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::Json<ApiResponse<ImportDetailResponse>>, AppError> {
    let import = query_import_detail(state.db(), id).await?;
    let files = query_import_files(state.db(), id).await?;

    Ok(ok(ImportDetailResponse {
        id: import.id,
        source_type: import.source_type,
        status: import.status,
        created_at: import.created_at,
        updated_at: import.updated_at,
        files,
    }))
}

async fn query_import_detail(db: &sqlx::PgPool, import_id: Uuid) -> Result<ImportDetailRow, AppError> {
    sqlx::query_as::<_, ImportDetailRow>(
        r#"
        SELECT id, source_type, status, created_at, updated_at
        FROM imports
        WHERE id = $1
        "#,
    )
    .bind(import_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)
}

async fn query_import_files(db: &sqlx::PgPool, import_id: Uuid) -> Result<Vec<ImportFileItem>, AppError> {
    sqlx::query_as::<_, ImportFileItem>(
        r#"
        SELECT id, original_filename, stored_filename, stored_path, file_size, mime_type, created_at
        FROM import_files
        WHERE import_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(import_id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}
