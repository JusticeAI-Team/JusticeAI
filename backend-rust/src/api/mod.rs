pub mod appeal_materials;
pub mod appeal_standardization;
pub mod appeals;
pub mod geo;
pub mod health;
pub mod import;
pub mod mobile;
pub mod platform;
pub mod prosecutor_appeals;
pub mod system;

use axum::Router;

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(system::routes())
        .merge(import::routes())
        .merge(appeals::routes())
        .merge(geo::routes())
        .merge(mobile::routes())
        .merge(appeal_materials::routes())
        .merge(appeal_standardization::routes())
        .merge(prosecutor_appeals::routes())
        .merge(platform::routes())
}
