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

const IMPORT_STATUS_UPLOADED: &str = "uploaded";

fn normalize_import_status_filter(status: Option<String>) -> Result<Option<String>, AppError> {
    let status = status.map(|value| value.trim().to_ascii_lowercase());

    match status.as_deref() {
        None | Some("") => Ok(None),
        Some(IMPORT_STATUS_UPLOADED) => Ok(status),
        Some(_) => Err(invalid_status_filter_error()),
    }
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

fn calculate_import_list_offset(page: i64, page_size: i64) -> i64 {
    page.saturating_sub(1).saturating_mul(page_size)
}

fn resolve_import_list_page(page: i64, page_size: i64, total: i64) -> i64 {
    if total <= 0 {
        return 1;
    }

    let page_size = page_size.max(1);
    let total_pages = total.saturating_sub(1) / page_size + 1;
    page.min(total_pages)
}

fn normalize_import_list_query(
    query: ImportListQuery,
) -> Result<NormalizedImportListQuery, AppError> {
    let page = query.page.unwrap_or(1).max(1);

    let raw_page_size = query.page_size.unwrap_or(20);
    let page_size = if raw_page_size < 1 {
        20
    } else {
        raw_page_size.min(100)
    };

    let offset = calculate_import_list_offset(page, page_size);
    let status = normalize_import_status_filter(query.status)?;

    Ok(NormalizedImportListQuery {
        page,
        page_size,
        offset,
        status,
    })
}

fn invalid_status_filter_error() -> AppError {
    AppError::Validation("仅支持 uploaded 状态筛选".to_string())
}

async fn list_imports(
    State(state): State<AppState>,
    Query(query): Query<ImportListQuery>,
) -> Result<axum::Json<ApiResponse<ImportListResponse>>, AppError> {
    let mut query = normalize_import_list_query(query)?;

    let total = match query.status.as_deref() {
        Some(status) => query_import_total_by_status(state.db(), status).await?,
        None => query_import_total(state.db()).await?,
    };

    query.page = resolve_import_list_page(query.page, query.page_size, total);
    query.offset = calculate_import_list_offset(query.page, query.page_size);

    let items = match query.status.as_deref() {
        Some(status) => query_import_list_by_status(state.db(), &query, status).await?,
        None => query_import_list(state.db(), &query).await?,
    };

    Ok(ok(ImportListResponse {
        items,
        page: query.page,
        page_size: query.page_size,
        total,
    }))
}

async fn query_import_total(db: &sqlx::PgPool) -> Result<i64, AppError> {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM imports
        "#,
    )
    .fetch_one(db)
    .await
    .map_err(|_| AppError::Internal)
}

async fn query_import_total_by_status(db: &sqlx::PgPool, status: &str) -> Result<i64, AppError> {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM imports
        WHERE status = $1
        "#,
    )
    .bind(status)
    .fetch_one(db)
    .await
    .map_err(|_| AppError::Internal)
}

async fn query_import_list(
    db: &sqlx::PgPool,
    query: &NormalizedImportListQuery,
) -> Result<Vec<ImportListItem>, AppError> {
    sqlx::query_as::<_, ImportListItem>(
        r#"
        SELECT id, source_type, status, created_at, updated_at
        FROM imports
        ORDER BY created_at DESC, id DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(query.page_size)
    .bind(query.offset)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

async fn query_import_list_by_status(
    db: &sqlx::PgPool,
    query: &NormalizedImportListQuery,
    status: &str,
) -> Result<Vec<ImportListItem>, AppError> {
    sqlx::query_as::<_, ImportListItem>(
        r#"
        SELECT id, source_type, status, created_at, updated_at
        FROM imports
        WHERE status = $1
        ORDER BY created_at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(status)
    .bind(query.page_size)
    .bind(query.offset)
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

async fn query_import_detail(
    db: &sqlx::PgPool,
    import_id: Uuid,
) -> Result<ImportDetailRow, AppError> {
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

async fn query_import_files(
    db: &sqlx::PgPool,
    import_id: Uuid,
) -> Result<Vec<ImportFileItem>, AppError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_import_list_query_uses_defaults() {
        let query = ImportListQuery {
            page: None,
            page_size: None,
            status: None,
        };

        let normalized = normalize_import_list_query(query).unwrap();

        assert_eq!(normalized.page, 1);
        assert_eq!(normalized.page_size, 20);
        assert_eq!(normalized.offset, 0);
        assert_eq!(normalized.status, None);
    }

    #[test]
    fn normalize_import_list_query_clamps_page_zero_to_one() {
        let query = ImportListQuery {
            page: Some(0),
            page_size: None,
            status: None,
        };

        let normalized = normalize_import_list_query(query).unwrap();

        assert_eq!(normalized.page, 1);
    }

    #[test]
    fn normalize_import_list_query_resets_page_size_zero_to_default() {
        let query = ImportListQuery {
            page: None,
            page_size: Some(0),
            status: None,
        };

        let normalized = normalize_import_list_query(query).unwrap();

        assert_eq!(normalized.page_size, 20);
    }

    #[test]
    fn normalize_import_list_query_caps_page_size_at_hundred() {
        let query = ImportListQuery {
            page: None,
            page_size: Some(101),
            status: None,
        };

        let normalized = normalize_import_list_query(query).unwrap();

        assert_eq!(normalized.page_size, 100);
    }

    #[test]
    fn normalize_import_list_query_keeps_non_empty_status() {
        let query = ImportListQuery {
            page: None,
            page_size: None,
            status: Some("uploaded".to_string()),
        };

        let normalized = normalize_import_list_query(query).unwrap();

        assert_eq!(normalized.status, Some("uploaded".to_string()));

        let uppercase_query = ImportListQuery {
            page: None,
            page_size: None,
            status: Some("UPLOADED".to_string()),
        };

        let uppercase_normalized = normalize_import_list_query(uppercase_query).unwrap();

        assert_eq!(uppercase_normalized.status, Some("uploaded".to_string()));
    }

    #[test]
    fn normalize_import_list_query_drops_empty_status() {
        let query = ImportListQuery {
            page: None,
            page_size: None,
            status: Some(String::new()),
        };

        let normalized = normalize_import_list_query(query).unwrap();

        assert_eq!(normalized.status, None);
    }

    #[test]
    fn normalize_import_list_query_rejects_unknown_status() {
        let query = ImportListQuery {
            page: None,
            page_size: None,
            status: Some("processing".to_string()),
        };

        let error = normalize_import_list_query(query).unwrap_err();

        assert!(
            matches!(error, AppError::Validation(message) if message == "仅支持 uploaded 状态筛选")
        );
    }

    #[test]
    fn normalize_import_list_query_drops_blank_status() {
        let query = ImportListQuery {
            page: None,
            page_size: None,
            status: Some("   \t\n  ".to_string()),
        };

        let normalized = normalize_import_list_query(query).unwrap();

        assert_eq!(normalized.status, None);
    }

    #[test]
    fn normalize_import_list_query_calculates_offset_from_page_and_page_size() {
        let query = ImportListQuery {
            page: Some(3),
            page_size: Some(20),
            status: None,
        };

        let normalized = normalize_import_list_query(query).unwrap();

        assert_eq!(normalized.offset, 40);
    }

    #[test]
    fn resolve_import_list_page_returns_first_page_when_total_is_zero() {
        assert_eq!(resolve_import_list_page(5, 20, 0), 1);
    }

    #[test]
    fn resolve_import_list_page_keeps_page_within_range() {
        assert_eq!(resolve_import_list_page(2, 20, 41), 2);
    }

    #[test]
    fn resolve_import_list_page_clamps_out_of_range_page_to_last_page() {
        assert_eq!(resolve_import_list_page(9, 20, 41), 3);
    }

    #[test]
    fn resolve_import_list_page_treats_zero_page_size_as_one() {
        assert_eq!(resolve_import_list_page(5, 0, 3), 3);
    }
}
