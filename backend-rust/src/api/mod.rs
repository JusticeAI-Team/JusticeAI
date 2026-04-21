pub mod health;
pub mod import;
pub mod import_query;
pub mod platform;
pub mod system;
pub mod workflow;
pub mod workspace;

use axum::Router;

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(system::routes())
        .merge(import::routes())
        .merge(import_query::routes())
        .merge(platform::routes())
}
