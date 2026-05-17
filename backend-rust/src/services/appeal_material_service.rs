use std::path::{Component, Path, PathBuf};

use axum::{
    async_trait,
    extract::{
        multipart::{MultipartError, MultipartRejection},
        FromRequest, Multipart, Request,
    },
};
use chrono::{DateTime, Datelike, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    services::appeal_service::{self, AppealMaterialRow},
    shared::error::AppError,
};

pub const MAX_APPEAL_UPLOAD_FILE_BYTES: usize = 20 * 1024 * 1024;

pub struct AppealMultipart(pub Multipart);

#[async_trait]
impl<S> FromRequest<S> for AppealMultipart
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Multipart::from_request(req, state)
            .await
            .map(Self)
            .map_err(map_multipart_rejection)
    }
}

pub struct MaterialUploadPayload {
    pub category: String,
    pub description: String,
    pub original_filename: String,
    pub extension: String,
    pub mime_type: Option<String>,
    pub bytes: Vec<u8>,
}

pub struct MaterialDownload {
    pub original_filename: String,
    pub mime_type: Option<String>,
    pub bytes: Vec<u8>,
}

struct StorageTarget {
    stored_filename: String,
    relative_path: String,
    absolute_path: PathBuf,
}

#[derive(Debug, FromRow)]
struct MaterialFileRecord {
    original_filename: String,
    stored_path: String,
    mime_type: Option<String>,
}

pub async fn read_material_upload(mut multipart: Multipart) -> Result<MaterialUploadPayload, AppError> {
    let mut category = "other".to_string();
    let mut description = String::new();
    let mut file: Option<(String, String, Option<String>, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.map_err(map_multipart_error)? {
        match field.name() {
            Some("category") => {
                category = field.text().await.map_err(map_multipart_error)?;
            }
            Some("description") => {
                description = field.text().await.map_err(map_multipart_error)?;
            }
            Some("file") => {
                let original_filename = sanitize_original_filename(
                    field.file_name().ok_or_else(|| {
                        AppError::Validation("material filename is required".to_string())
                    })?,
                )?;
                let extension = extract_extension(&original_filename).ok_or_else(|| {
                    AppError::Validation("material file extension is required".to_string())
                })?;
                if !allowed_extension(&extension) {
                    return Err(AppError::Validation(
                        "only image, PDF, Word, Excel and text materials are supported".to_string(),
                    ));
                }
                let mime_type = field.content_type().map(str::to_string);
                if let Some(mime_type) = &mime_type {
                    if !allowed_mime(mime_type) {
                        return Err(AppError::Validation("unsupported material MIME type".to_string()));
                    }
                }
                let bytes = field.bytes().await.map_err(map_multipart_error)?.to_vec();
                if bytes.is_empty() {
                    return Err(AppError::Validation("material file cannot be empty".to_string()));
                }
                if bytes.len() > MAX_APPEAL_UPLOAD_FILE_BYTES {
                    return Err(AppError::Validation("material file cannot exceed 20 MB".to_string()));
                }
                file = Some((original_filename, extension, mime_type, bytes));
            }
            _ => {}
        }
    }

    if !allowed_category(&category) {
        return Err(AppError::Validation("unsupported material category".to_string()));
    }

    let (original_filename, extension, mime_type, bytes) =
        file.ok_or_else(|| AppError::Validation("file field is required".to_string()))?;

    Ok(MaterialUploadPayload {
        category,
        description,
        original_filename,
        extension,
        mime_type,
        bytes,
    })
}

pub async fn save_material(
    db: &PgPool,
    upload_dir: &str,
    applicant_id: Uuid,
    appeal_id: Uuid,
    payload: MaterialUploadPayload,
) -> Result<AppealMaterialRow, AppError> {
    appeal_service::ensure_appeal_owner_if_needed(db, appeal_id, Some(applicant_id)).await?;
    let now = Utc::now();
    let material_id = Uuid::new_v4();
    let storage = build_storage_target(upload_dir, &payload.extension, now);
    write_file(&storage, &payload.bytes)?;

    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    if sqlx::query(
        r#"
        INSERT INTO appeal_materials (
            id, appeal_id, category, description, original_filename, stored_filename,
            stored_path, file_size, mime_type, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(material_id)
    .bind(appeal_id)
    .bind(&payload.category)
    .bind(&payload.description)
    .bind(&payload.original_filename)
    .bind(&storage.stored_filename)
    .bind(&storage.relative_path)
    .bind(payload.bytes.len() as i64)
    .bind(&payload.mime_type)
    .bind(now)
    .execute(&mut *tx)
    .await
    .is_err()
    {
        cleanup_file(&storage.absolute_path);
        return Err(AppError::Internal);
    }

    appeal_service::insert_event_tx(
        &mut tx,
        appeal_id,
        "material_uploaded",
        "applicant",
        &applicant_id.to_string(),
        "上传材料",
        &format!("已上传材料类别：{}", payload.category),
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    appeal_service::recompute_score(db, appeal_id).await?;

    let mut rows = appeal_service::list_materials(db, appeal_id).await?;
    rows.retain(|row| row.id == material_id);
    rows.into_iter().next().ok_or(AppError::NotFound)
}

pub async fn delete_material(
    db: &PgPool,
    applicant_id: Uuid,
    appeal_id: Uuid,
    material_id: Uuid,
) -> Result<AppealMaterialRow, AppError> {
    appeal_service::ensure_appeal_owner_if_needed(db, appeal_id, Some(applicant_id)).await?;
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    let material = sqlx::query_as::<_, AppealMaterialRow>(
        r#"
        UPDATE appeal_materials
        SET deleted_at = COALESCE(deleted_at, $4)
        WHERE id = $1 AND appeal_id = $2 AND deleted_at IS NULL
        RETURNING id, appeal_id, category, description, original_filename, stored_filename,
                  file_size, mime_type, deleted_at, created_at
        "#,
    )
    .bind(material_id)
    .bind(appeal_id)
    .bind(applicant_id)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    appeal_service::insert_event_tx(
        &mut tx,
        appeal_id,
        "material_deleted",
        "applicant",
        &applicant_id.to_string(),
        "删除材料",
        &format!("已删除材料类别：{}", material.category),
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    appeal_service::recompute_score(db, appeal_id).await?;
    Ok(material)
}

pub async fn download_material_for_applicant(
    db: &PgPool,
    upload_dir: &str,
    applicant_id: Uuid,
    appeal_id: Uuid,
    material_id: Uuid,
) -> Result<MaterialDownload, AppError> {
    appeal_service::ensure_appeal_owner_if_needed(db, appeal_id, Some(applicant_id)).await?;
    load_material_file(db, upload_dir, appeal_id, material_id).await
}

pub async fn download_material_for_staff(
    db: &PgPool,
    upload_dir: &str,
    staff_id: &str,
    appeal_id: Uuid,
    material_id: Uuid,
) -> Result<MaterialDownload, AppError> {
    appeal_service::get_appeal(db, appeal_id).await?;
    let material = load_material_file(db, upload_dir, appeal_id, material_id).await?;
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    appeal_service::insert_event_tx(
        &mut tx,
        appeal_id,
        "material_downloaded",
        "staff",
        staff_id,
        "下载材料",
        "工作人员下载了申诉材料",
        false,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    Ok(material)
}

async fn load_material_file(
    db: &PgPool,
    upload_dir: &str,
    appeal_id: Uuid,
    material_id: Uuid,
) -> Result<MaterialDownload, AppError> {
    let record = sqlx::query_as::<_, MaterialFileRecord>(
        r#"
        SELECT original_filename, stored_path, mime_type
        FROM appeal_materials
        WHERE id = $1 AND appeal_id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(material_id)
    .bind(appeal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let relative_path = Path::new(&record.stored_path);
    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(AppError::Internal);
    }

    let absolute_path = Path::new(upload_dir).join(relative_path);
    let bytes = std::fs::read(absolute_path).map_err(|_| AppError::NotFound)?;
    Ok(MaterialDownload {
        original_filename: record.original_filename,
        mime_type: record.mime_type,
        bytes,
    })
}

fn build_storage_target(upload_dir: &str, extension: &str, now: DateTime<Utc>) -> StorageTarget {
    let stored_filename = format!("{}.{}", Uuid::new_v4(), extension);
    let relative_path = format!(
        "appeal-materials/{}/{:02}/{:02}/{}",
        now.year(),
        now.month(),
        now.day(),
        stored_filename
    );
    let absolute_path = Path::new(upload_dir).join(&relative_path);
    StorageTarget {
        stored_filename,
        relative_path,
        absolute_path,
    }
}

fn write_file(target: &StorageTarget, bytes: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = target.absolute_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| AppError::Internal)?;
    }
    std::fs::write(&target.absolute_path, bytes).map_err(|_| AppError::Internal)
}

fn cleanup_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn allowed_category(category: &str) -> bool {
    matches!(
        category,
        "identity"
            | "labor_contract"
            | "wage_record"
            | "attendance"
            | "chat_record"
            | "bank_statement"
            | "work_badge"
            | "project_photo"
            | "location_screenshot"
            | "coworker_statement"
            | "other"
    )
}

fn allowed_extension(ext: &str) -> bool {
    matches!(
        ext,
        "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "pdf"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "txt"
            | "csv"
    )
}

fn allowed_mime(mime: &str) -> bool {
    mime.starts_with("image/")
        || matches!(
            mime,
            "application/pdf"
                | "application/msword"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                | "application/vnd.ms-excel"
                | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                | "text/plain"
                | "text/csv"
                | "application/octet-stream"
        )
}

fn extract_extension(filename: &str) -> Option<String> {
    Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

fn sanitize_original_filename(filename: &str) -> Result<String, AppError> {
    let trimmed = filename.trim();
    if trimmed.is_empty() || trimmed.len() > 255 {
        return Err(AppError::Validation("invalid material filename".to_string()));
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(AppError::Validation("invalid material filename".to_string()));
    }
    Ok(trimmed.to_string())
}

fn map_multipart_error(error: MultipartError) -> AppError {
    AppError::Validation(format!("failed to read multipart material form: {}", error.body_text()))
}

fn map_multipart_rejection(error: MultipartRejection) -> AppError {
    AppError::Validation(format!("failed to read multipart material form: {}", error.body_text()))
}
