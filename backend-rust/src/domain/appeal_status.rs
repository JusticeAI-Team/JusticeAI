use crate::shared::error::AppError;

pub const DRAFT: &str = "draft";
pub const SUBMITTED: &str = "submitted";
pub const SUBMITTED_INCOMPLETE: &str = "submitted_incomplete";
pub const UNDER_REVIEW: &str = "under_review";
pub const MATERIAL_REQUESTED: &str = "material_requested";
pub const ACCEPTED: &str = "accepted";
pub const PROCESSING: &str = "processing";
pub const RESOLVED: &str = "resolved";
pub const CLOSED: &str = "closed";
pub const REJECTED: &str = "rejected";

pub fn ensure_accept_allowed(status: &str) -> Result<(), AppError> {
    if matches!(
        status,
        SUBMITTED | SUBMITTED_INCOMPLETE | UNDER_REVIEW | MATERIAL_REQUESTED
    ) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "current status {status} cannot be accepted"
        )))
    }
}

pub fn ensure_request_materials_allowed(status: &str) -> Result<(), AppError> {
    if matches!(
        status,
        SUBMITTED | SUBMITTED_INCOMPLETE | UNDER_REVIEW | MATERIAL_REQUESTED | ACCEPTED
    ) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "current status {status} cannot request supplement materials"
        )))
    }
}

pub fn ensure_supplement_allowed(status: &str) -> Result<(), AppError> {
    if status == MATERIAL_REQUESTED {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "current status {status} cannot submit supplement"
        )))
    }
}

pub fn prosecutor_action_to_status(action: &str, current: &str) -> Result<&'static str, AppError> {
    match action {
        "start_processing" if matches!(current, ACCEPTED | UNDER_REVIEW) => Ok(PROCESSING),
        "close" if matches!(current, RESOLVED | REJECTED) => Ok(CLOSED),
        "reject" if current != CLOSED => Ok(REJECTED),
        _ => Err(AppError::Validation(format!(
            "action {action} is not allowed from status {current}"
        ))),
    }
}
