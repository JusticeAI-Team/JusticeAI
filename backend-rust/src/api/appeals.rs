use axum::{routing::get, Router};
use serde::Serialize;

use crate::{
    app::AppState,
    shared::response::{ok, ApiResponse},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/appeals/capabilities", get(capabilities))
}

#[derive(Debug, Serialize)]
struct AppealCapabilities {
    statuses: Vec<&'static str>,
    material_categories: Vec<&'static str>,
}

async fn capabilities() -> axum::Json<ApiResponse<AppealCapabilities>> {
    ok(AppealCapabilities {
        statuses: vec![
            "draft",
            "submitted",
            "standardizing",
            "submitted_incomplete",
            "under_review",
            "material_requested",
            "accepted",
            "transferred",
            "processing",
            "resolved",
            "closed",
            "rejected",
        ],
        material_categories: vec![
            "identity",
            "labor_contract",
            "wage_record",
            "attendance",
            "chat_record",
            "bank_statement",
            "work_badge",
            "project_photo",
            "location_screenshot",
            "coworker_statement",
            "other",
        ],
    })
}
