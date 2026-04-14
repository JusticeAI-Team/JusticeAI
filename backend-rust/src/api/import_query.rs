use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
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

#[derive(Debug, Serialize)]
struct ImportListResponse {
    items: Vec<ImportListItem>,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
struct ImportFileItem {
    id: Uuid,
    original_filename: String,
    stored_filename: String,
    stored_path: String,
    file_size: i64,
    mime_type: Option<String>,
    created_at: DateTime<Utc>,
}

async fn list_imports(
    State(_state): State<AppState>,
) -> Result<axum::Json<ApiResponse<ImportListResponse>>, AppError> {
    Ok(ok(ImportListResponse { items: Vec::new() }))
}

async fn get_import_detail(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<axum::Json<ApiResponse<ImportDetailResponse>>, AppError> {
    Err(AppError::NotFound)
}
