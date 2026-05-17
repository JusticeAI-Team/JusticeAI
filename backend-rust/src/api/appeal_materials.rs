use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    app::AppState,
    services::{
        appeal_material_service::{
            self, AppealMultipart, MAX_APPEAL_UPLOAD_FILE_BYTES,
        },
        appeal_service::{AppealMaterialRow, DEV_APPLICANT_ID},
    },
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/mobile/appeals/:id/materials", post(upload_material))
        .route(
            "/mobile/appeals/:id/materials/:material_id",
            delete(delete_material),
        )
        .route(
            "/mobile/appeals/:id/materials/:material_id/download",
            get(download_mobile_material),
        )
        .route(
            "/prosecutor/appeals/:id/materials/:material_id/download",
            get(download_prosecutor_material),
        )
        .layer(DefaultBodyLimit::max(MAX_APPEAL_UPLOAD_FILE_BYTES + 1024 * 1024))
}

async fn upload_material(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    AppealMultipart(multipart): AppealMultipart,
) -> Result<Json<ApiResponse<AppealMaterialRow>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    let payload = appeal_material_service::read_material_upload(multipart).await?;
    Ok(ok(
        appeal_material_service::save_material(
            state.db(),
            &state.settings().storage.upload_dir,
            applicant_id,
            id,
            payload,
        )
        .await?,
    ))
}

async fn delete_material(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, material_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<AppealMaterialRow>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(
        appeal_material_service::delete_material(state.db(), applicant_id, id, material_id).await?,
    ))
}

async fn download_mobile_material(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, material_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let applicant_id = applicant_id(&headers)?;
    let material = appeal_material_service::download_material_for_applicant(
        state.db(),
        &state.settings().storage.upload_dir,
        applicant_id,
        id,
        material_id,
    )
    .await?;
    material_download_response(material)
}

async fn download_prosecutor_material(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, material_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let staff_id = staff_id(&headers);
    let material = appeal_material_service::download_material_for_staff(
        state.db(),
        &state.settings().storage.upload_dir,
        &staff_id,
        id,
        material_id,
    )
    .await?;
    material_download_response(material)
}

fn applicant_id(headers: &HeaderMap) -> Result<Uuid, AppError> {
    headers
        .get("x-mobile-applicant-id")
        .and_then(|value| value.to_str().ok())
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| AppError::Unauthorized)?
        .map(Ok)
        .unwrap_or(Ok(DEV_APPLICANT_ID))
}

fn staff_id(headers: &HeaderMap) -> String {
    headers
        .get("x-staff-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("dev-staff")
        .to_string()
}

fn staff_role(headers: &HeaderMap) -> String {
    headers
        .get("x-staff-role")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("prosecutor")
        .to_string()
}

fn ensure_staff_allowed(headers: &HeaderMap) -> Result<(), AppError> {
    let role = staff_role(headers);
    if matches!(
        role.as_str(),
        "prosecutor" | "prosecutor_reviewer" | "prosecutor_admin"
    ) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn staff_area_code(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-staff-area-code")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

async fn ensure_staff_area_access(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    appeal_id: Uuid,
) -> Result<(), AppError> {
    if staff_role(headers) == "prosecutor_admin" {
        return Ok(());
    }
    let Some(staff_area) = staff_area_code(headers) else {
        return Ok(());
    };
    let area_code = sqlx::query_scalar::<_, Option<String>>(
        "SELECT area_code FROM appeal_locations WHERE appeal_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(appeal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .flatten();
    if area_code.as_deref() == Some(staff_area.as_str()) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn material_download_response(
    material: appeal_material_service::MaterialDownload,
) -> Result<impl IntoResponse, AppError> {
    let mut headers = HeaderMap::new();
    let mime_type = material
        .mime_type
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("application/octet-stream");
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime_type).map_err(|_| AppError::Internal)?,
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"{}\"",
            ascii_download_filename(&material.original_filename)
        ))
        .map_err(|_| AppError::Internal)?,
    );
    Ok((headers, material.bytes))
}

fn ascii_download_filename(original: &str) -> String {
    let extension = std::path::Path::new(original)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("bin");
    format!("appeal-material.{extension}")
}
