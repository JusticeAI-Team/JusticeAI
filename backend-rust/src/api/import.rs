use std::path::PathBuf;

use axum::{
    extract::{Multipart, State},
    routing::post,
    Router,
};
use chrono::{Datelike, Utc};
use uuid::Uuid;

use crate::{
    app::AppState,
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/import/upload", post(upload))
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

async fn upload(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<axum::Json<ApiResponse<String>>, AppError> {
    let payload = read_upload_field(multipart).await?;
    let storage = build_storage_target(state.settings().storage.upload_dir.as_str(), &payload.extension);
    write_upload_file(&storage, &payload.bytes)?;
    Ok(ok(storage.relative_path))
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

fn build_storage_target(upload_dir: &str, extension: &str) -> StorageTarget {
    let now = Utc::now();
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

    std::fs::write(&target.absolute_path, bytes).map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn read_upload_field(mut multipart: Multipart) -> Result<UploadFilePayload, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("读取上传表单失败".to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let original_filename = field
            .file_name()
            .map(str::to_string)
            .ok_or_else(|| AppError::Validation("上传文件名不能为空".to_string()))?;

        if original_filename.trim().is_empty() {
            return Err(AppError::Validation("上传文件名不能为空".to_string()));
        }

        let extension = extract_extension(&original_filename)
            .ok_or_else(|| AppError::Validation("仅支持 xlsx、xls、csv 文件".to_string()))?;

        if !allowed_extension(&extension) {
            return Err(AppError::Validation("仅支持 xlsx、xls、csv 文件".to_string()));
        }

        let mime_type = field.content_type().map(str::to_string);
        let bytes = field
            .bytes()
            .await
            .map_err(|_| AppError::Internal)?
            .to_vec();

        if bytes.is_empty() {
            return Err(AppError::Validation("上传文件不能为空".to_string()));
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
