use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    routing::{delete, post},
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
