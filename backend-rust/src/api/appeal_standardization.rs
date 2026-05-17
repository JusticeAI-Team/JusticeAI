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
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AppealStandardizationRow>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    Ok(ok(
        appeal_standardization_service::standardize_appeal(&state, id).await?,
    ))
}

async fn list_standardizations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<AppealStandardizationRow>>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    Ok(ok(
        appeal_standardization_service::list_standardizations(state.db(), id).await?,
    ))
}

async fn latest_standardization(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Option<AppealStandardizationRow>>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
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
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    if input.reviewed_by.is_none() {
        input.reviewed_by = Some(staff_id(&headers));
    }
    if input.reviewed_role.is_none() {
        input.reviewed_role = Some(staff_role(&headers));
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

fn staff_id(headers: &HeaderMap) -> String {
    headers
        .get("x-staff-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("dev-staff")
        .to_string()
}
