use axum::{routing::post, Router};

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/import/upload", post(upload_placeholder))
}

async fn upload_placeholder() -> &'static str {
    "upload module placeholder"
}
