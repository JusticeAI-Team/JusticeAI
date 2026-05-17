use crate::shared::error::AppError;

pub const DRAFT: &str = "draft";
pub const SUBMITTED: &str = "submitted";
#[allow(dead_code)]
pub const STANDARDIZING: &str = "standardizing";
pub const SUBMITTED_INCOMPLETE: &str = "submitted_incomplete";
pub const UNDER_REVIEW: &str = "under_review";
pub const MATERIAL_REQUESTED: &str = "material_requested";
pub const ACCEPTED: &str = "accepted";
pub const TRANSFERRED: &str = "transferred";
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
        Err(AppError::Conflict(format!(
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
        Err(AppError::Conflict(format!(
            "current status {status} cannot request supplement materials"
        )))
    }
}

pub fn ensure_supplement_allowed(status: &str) -> Result<(), AppError> {
    if status == MATERIAL_REQUESTED {
        Ok(())
    } else {
        Err(AppError::Conflict(format!(
            "current status {status} cannot submit supplement"
        )))
    }
}

pub fn prosecutor_action_to_status(action: &str, current: &str) -> Result<&'static str, AppError> {
    match action {
        "start_processing" if matches!(current, ACCEPTED | UNDER_REVIEW | TRANSFERRED) => {
            Ok(PROCESSING)
        }
        "close" if matches!(current, RESOLVED | REJECTED) => Ok(CLOSED),
        "reject" if current != CLOSED => Ok(REJECTED),
        _ => Err(AppError::Conflict(format!(
            "action {action} is not allowed from status {current}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_allows_submitted_and_under_review_states() {
        assert!(ensure_accept_allowed(SUBMITTED).is_ok());
        assert!(ensure_accept_allowed(SUBMITTED_INCOMPLETE).is_ok());
        assert!(ensure_accept_allowed(UNDER_REVIEW).is_ok());
        assert!(ensure_accept_allowed(MATERIAL_REQUESTED).is_ok());
    }

    #[test]
    fn invalid_accept_returns_conflict() {
        assert!(matches!(
            ensure_accept_allowed(DRAFT),
            Err(AppError::Conflict(_))
        ));
    }

    #[test]
    fn supplement_only_allows_material_requested() {
        assert!(ensure_supplement_allowed(MATERIAL_REQUESTED).is_ok());
        assert!(matches!(
            ensure_supplement_allowed(ACCEPTED),
            Err(AppError::Conflict(_))
        ));
    }

    #[test]
    fn prosecutor_actions_are_state_machine_driven() {
        assert_eq!(
            prosecutor_action_to_status("start_processing", ACCEPTED).unwrap(),
            PROCESSING
        );
        assert_eq!(
            prosecutor_action_to_status("start_processing", TRANSFERRED).unwrap(),
            PROCESSING
        );
        assert_eq!(prosecutor_action_to_status("close", RESOLVED).unwrap(), CLOSED);
        assert!(matches!(
            prosecutor_action_to_status("start_processing", DRAFT),
            Err(AppError::Conflict(_))
        ));
    }
}
