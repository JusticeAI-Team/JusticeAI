use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    app::AppState,
    services::appeal_standardization_service::{
        self, AppealStandardizationRow, ReviewStandardizationInput,
    },
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/prosecutor/appeals/:id/standardize",
            post(run_standardization),
        )
        .route(
            "/prosecutor/appeals/:id/standardizations",
            get(list_standardizations),
        )
        .route(
            "/prosecutor/appeals/:id/standardizations/latest",
            get(latest_standardization),
        )
        .route(
            "/prosecutor/appeals/:id/standardizations/:standardization_id/review",
            post(review_standardization),
        )
}

async fn run_standardization(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AppealStandardizationRow>>, AppError> {
    Ok(ok(
        appeal_standardization_service::standardize_appeal(&state, id).await?,
    ))
}

async fn list_standardizations(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<AppealStandardizationRow>>>, AppError> {
    Ok(ok(
        appeal_standardization_service::list_standardizations(state.db(), id).await?,
    ))
}

async fn latest_standardization(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Option<AppealStandardizationRow>>>, AppError> {
    Ok(ok(
        appeal_standardization_service::latest_standardization(state.db(), id).await?,
    ))
}

async fn review_standardization(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, standardization_id)): Path<(Uuid, Uuid)>,
    Json(mut input): Json<ReviewStandardizationInput>,
) -> Result<Json<ApiResponse<AppealStandardizationRow>>, AppError> {
    if input.reviewed_by.is_none() {
        input.reviewed_by = Some(staff_id(&headers));
    }
    Ok(ok(
        appeal_standardization_service::review_standardization(
            state.db(),
            id,
            standardization_id,
            input,
        )
        .await?,
    ))
}

fn staff_id(headers: &HeaderMap) -> String {
    headers
        .get("x-staff-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("dev-staff")
        .to_string()
}
