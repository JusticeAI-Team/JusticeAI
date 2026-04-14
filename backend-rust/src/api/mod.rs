pub mod health;
pub mod import;
pub mod system;

use axum::Router;

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(import::routes())
        .merge(system::routes())
}
