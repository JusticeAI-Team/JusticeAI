use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    app::AppState,
    services::appeal_service::{
        self, AppealEventRow, AppealLocationRow, AppealMaterialRow, AppealNotificationRow,
        ApplicantProfile, CreateDraftInput, LaborAppealRow, SaveDraftInput, SaveLocationInput,
        SubmitInput, DEV_APPLICANT_ID,
    },
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/mobile/profile", get(profile))
        .route("/mobile/appeals", get(my_appeals))
        .route("/mobile/appeals/drafts", post(create_draft))
        .route("/mobile/appeals/:id", get(appeal_detail))
        .route("/mobile/appeals/:id/draft", put(save_draft))
        .route("/mobile/appeals/:id/submit", post(submit))
        .route("/mobile/appeals/:id/location", put(save_location))
        .route("/mobile/appeals/:id/timeline", get(timeline))
        .route("/mobile/appeals/:id/supplement", post(supplement))
        .route("/mobile/notifications", get(notifications))
        .route("/mobile/notifications/:id/read", post(read_notification))
}

#[derive(Debug, Serialize)]
struct MobileAppealDetail {
    appeal: LaborAppealRow,
    location: Option<AppealLocationRow>,
    materials: Vec<AppealMaterialRow>,
    timeline: Vec<AppealEventRow>,
}

#[derive(Debug, Serialize)]
struct MobileProfileResponse {
    profile: ApplicantProfile,
}

async fn profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<MobileProfileResponse>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    let profile = appeal_service::ensure_dev_applicant(state.db(), applicant_id).await?;
    Ok(ok(MobileProfileResponse { profile }))
}

async fn create_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateDraftInput>,
) -> Result<Json<ApiResponse<LaborAppealRow>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(appeal_service::create_draft(state.db(), applicant_id, input).await?))
}

async fn save_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<SaveDraftInput>,
) -> Result<Json<ApiResponse<LaborAppealRow>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(
        appeal_service::save_draft(state.db(), Some(applicant_id), id, input).await?,
    ))
}

async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<SubmitInput>,
) -> Result<Json<ApiResponse<LaborAppealRow>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(
        appeal_service::submit_appeal(state.db(), applicant_id, id, input).await?,
    ))
}

async fn save_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<SaveLocationInput>,
) -> Result<Json<ApiResponse<AppealLocationRow>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(
        appeal_service::save_location(state.db(), applicant_id, id, input).await?,
    ))
}

async fn my_appeals(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<LaborAppealRow>>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(appeal_service::list_mobile_appeals(state.db(), applicant_id).await?))
}

async fn appeal_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MobileAppealDetail>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    appeal_service::ensure_appeal_owner_if_needed(state.db(), id, Some(applicant_id)).await?;
    Ok(ok(MobileAppealDetail {
        appeal: appeal_service::get_appeal(state.db(), id).await?,
        location: appeal_service::maybe_location(state.db(), id).await?,
        materials: appeal_service::list_materials(state.db(), id).await?,
        timeline: appeal_service::list_events(state.db(), id, true).await?,
    }))
}

async fn timeline(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<AppealEventRow>>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    appeal_service::ensure_appeal_owner_if_needed(state.db(), id, Some(applicant_id)).await?;
    Ok(ok(appeal_service::list_events(state.db(), id, true).await?))
}

async fn notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<AppealNotificationRow>>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(
        appeal_service::list_notifications(state.db(), applicant_id).await?,
    ))
}

async fn read_notification(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AppealNotificationRow>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(
        appeal_service::mark_notification_read(state.db(), applicant_id, id).await?,
    ))
}

async fn supplement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<LaborAppealRow>>, AppError> {
    let applicant_id = applicant_id(&headers)?;
    Ok(ok(appeal_service::supplement(state.db(), applicant_id, id).await?))
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
