use std::path::PathBuf;

use axum::{
    extract::{multipart::MultipartError, DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    routing::post,
    Router,
};
use chrono::{DateTime, Datelike, Utc};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    app::AppState,
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

const MAX_UPLOAD_FILE_BYTES: usize = 10 * 1024 * 1024;
const MAX_UPLOAD_REQUEST_BYTES: usize = MAX_UPLOAD_FILE_BYTES + 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ingestion/upload", post(upload))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_REQUEST_BYTES))
}

struct UploadFilePayload {
    original_filename: String,
    extension: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
}

struct StorageTarget {
    stored_filename: String,
    relative_path: String,
    absolute_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct UploadResponse {
    import_id: Uuid,
    status: &'static str,
    file: UploadedFileInfo,
}

#[derive(Debug, Serialize)]
struct UploadedFileInfo {
    id: Uuid,
    original_filename: String,
    stored_filename: String,
    stored_path: String,
    file_size: i64,
    mime_type: Option<String>,
}

async fn upload(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<axum::Json<ApiResponse<UploadResponse>>, AppError> {
    let payload = read_upload_field(multipart).await?;
    let now = Utc::now();
    let storage = build_storage_target(
        state.settings().storage.upload_dir.as_str(),
        &payload.extension,
        now,
    );
    let import_id = Uuid::new_v4();
    let file_id = Uuid::new_v4();

    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;

    insert_import_record(&mut tx, import_id, now).await?;
    write_upload_file(&storage, &payload.bytes)?;

    if insert_import_file_record(&mut tx, file_id, import_id, &payload, &storage, now)
        .await
        .is_err()
    {
        cleanup_uploaded_file(&storage);
        return Err(AppError::Internal);
    }

    if let Err(err) = tx.commit().await {
        tracing::warn!(
            path = %storage.absolute_path.display(),
            error = %err,
            "导入事务提交失败，保留已写入文件以避免潜在已提交记录指向缺失文件"
        );
        return Err(AppError::Internal);
    }

    Ok(ok(UploadResponse {
        import_id,
        status: "uploaded",
        file: UploadedFileInfo {
            id: file_id,
            original_filename: payload.original_filename,
            stored_filename: storage.stored_filename,
            stored_path: storage.relative_path,
            file_size: payload.bytes.len() as i64,
            mime_type: payload.mime_type,
        },
    }))
}

fn allowed_extension(ext: &str) -> bool {
    matches!(ext, "xlsx" | "xls" | "csv")
}

fn extract_extension(filename: &str) -> Option<String> {
    std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

fn sanitize_original_filename(filename: &str) -> Result<String, AppError> {
    let trimmed = filename.trim();

    if trimmed.is_empty() {
        return Err(AppError::Validation("上传文件名不能为空".to_string()));
    }

    if trimmed.len() > 255 {
        return Err(AppError::Validation("上传文件名过长".to_string()));
    }

    if trimmed
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(AppError::Validation("上传文件名包含非法字符".to_string()));
    }

    Ok(trimmed.to_string())
}

fn build_storage_target(upload_dir: &str, extension: &str, now: DateTime<Utc>) -> StorageTarget {
    let uuid = Uuid::new_v4().to_string();
    let stored_filename = format!("{}.{}", uuid, extension);
    let relative_path = format!(
        "{}/{:02}/{:02}/{}",
        now.year(),
        now.month(),
        now.day(),
        stored_filename
    );
    let absolute_path = std::path::Path::new(upload_dir).join(&relative_path);

    StorageTarget {
        stored_filename,
        relative_path,
        absolute_path,
    }
}

fn write_upload_file(target: &StorageTarget, bytes: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = target.absolute_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| AppError::Internal)?;
    }

    if std::fs::write(&target.absolute_path, bytes).is_err() {
        cleanup_uploaded_file(target);
        return Err(AppError::Internal);
    }

    Ok(())
}

fn cleanup_uploaded_file(target: &StorageTarget) {
    match std::fs::remove_file(&target.absolute_path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            tracing::warn!(
                path = %target.absolute_path.display(),
                error = %err,
                "上传失败后的补偿删除未成功"
            );
        }
    }
}

fn map_multipart_error(error: MultipartError) -> AppError {
    match error.status() {
        StatusCode::PAYLOAD_TOO_LARGE => AppError::Validation("上传文件不能超过 10 MB".to_string()),
        StatusCode::BAD_REQUEST => AppError::Validation("读取上传表单失败".to_string()),
        _ => AppError::Internal,
    }
}

async fn read_upload_field(mut multipart: Multipart) -> Result<UploadFilePayload, AppError> {
    while let Some(field) = multipart.next_field().await.map_err(map_multipart_error)? {
        if field.name() != Some("file") {
            continue;
        }

        let original_filename = field
            .file_name()
            .ok_or_else(|| AppError::Validation("上传文件名不能为空".to_string()))?;
        let original_filename = sanitize_original_filename(original_filename)?;

        let extension = extract_extension(&original_filename)
            .ok_or_else(|| AppError::Validation("仅支持 xlsx、xls、csv 文件".to_string()))?;

        if !allowed_extension(&extension) {
            return Err(AppError::Validation(
                "仅支持 xlsx、xls、csv 文件".to_string(),
            ));
        }

        let mime_type = field.content_type().map(str::to_string);
        let bytes = field.bytes().await.map_err(map_multipart_error)?.to_vec();

        if bytes.is_empty() {
            return Err(AppError::Validation("上传文件不能为空".to_string()));
        }

        if bytes.len() > MAX_UPLOAD_FILE_BYTES {
            return Err(AppError::Validation("上传文件不能超过 10 MB".to_string()));
        }

        return Ok(UploadFilePayload {
            original_filename,
            extension,
            mime_type,
            bytes,
        });
    }

    Err(AppError::Validation("必须提供 file 文件字段".to_string()))
}

async fn insert_import_record(
    tx: &mut Transaction<'_, Postgres>,
    import_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO imports (id, source_type, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(import_id)
    .bind("manual_upload")
    .bind("uploaded")
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(())
}

async fn insert_import_file_record(
    tx: &mut Transaction<'_, Postgres>,
    file_id: Uuid,
    import_id: Uuid,
    payload: &UploadFilePayload,
    storage: &StorageTarget,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO import_files (
            id, import_id, original_filename, stored_filename, stored_path,
            file_size, mime_type, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(file_id)
    .bind(import_id)
    .bind(&payload.original_filename)
    .bind(&storage.stored_filename)
    .bind(&storage.relative_path)
    .bind(payload.bytes.len() as i64)
    .bind(&payload.mime_type)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn allowed_extension_accepts_supported_types() {
        assert!(allowed_extension("csv"));
        assert!(allowed_extension("xls"));
        assert!(allowed_extension("xlsx"));
        assert!(!allowed_extension("txt"));
    }

    #[test]
    fn extract_extension_normalizes_case() {
        assert_eq!(extract_extension("案件台账.XLSX"), Some("xlsx".to_string()));
    }

    #[test]
    fn extract_extension_returns_none_without_suffix() {
        assert_eq!(extract_extension("案件台账"), None);
    }

    #[test]
    fn sanitize_original_filename_trims_whitespace() {
        let sanitized = sanitize_original_filename("  案件台账.xlsx  ").unwrap();

        assert_eq!(sanitized, "案件台账.xlsx");
    }

    #[test]
    fn sanitize_original_filename_rejects_blank_name() {
        let error = sanitize_original_filename("   \t\n ").unwrap_err();

        assert!(matches!(error, AppError::Validation(message) if message == "上传文件名不能为空"));
    }

    #[test]
    fn sanitize_original_filename_rejects_illegal_characters() {
        let error = sanitize_original_filename("案件/台账.xlsx").unwrap_err();

        assert!(
            matches!(error, AppError::Validation(message) if message == "上传文件名包含非法字符")
        );
    }

    #[test]
    fn sanitize_original_filename_rejects_backslash() {
        let error = sanitize_original_filename("案件\\台账.xlsx").unwrap_err();

        assert!(
            matches!(error, AppError::Validation(message) if message == "上传文件名包含非法字符")
        );
    }

    #[test]
    fn sanitize_original_filename_rejects_too_long_name() {
        let filename = format!("{}.xlsx", "案".repeat(252));

        let error = sanitize_original_filename(&filename).unwrap_err();

        assert!(matches!(error, AppError::Validation(message) if message == "上传文件名过长"));
    }

    #[test]
    fn build_storage_target_uses_given_date_parts() {
        let now = Utc
            .with_ymd_and_hms(2026, 4, 17, 8, 30, 0)
            .single()
            .unwrap();

        let target = build_storage_target("uploads", "csv", now);

        assert!(target.stored_filename.ends_with(".csv"));
        assert!(target.relative_path.starts_with("2026/04/17/"));
        assert_eq!(
            target.absolute_path,
            PathBuf::from("uploads").join(&target.relative_path)
        );
    }
}
