use axum::{routing::get, Router};
use serde::Serialize;

use crate::{
    app::AppState,
    shared::response::{ok, ApiResponse},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/geo/beijing-districts", get(beijing_districts))
}

#[derive(Debug, Serialize)]
struct District {
    area_code: &'static str,
    area_name: &'static str,
}

async fn beijing_districts() -> axum::Json<ApiResponse<Vec<District>>> {
    ok(vec![
        District {
            area_code: "110101",
            area_name: "北京市东城区",
        },
        District {
            area_code: "110102",
            area_name: "北京市西城区",
        },
        District {
            area_code: "110105",
            area_name: "北京市朝阳区",
        },
        District {
            area_code: "110112",
            area_name: "北京市通州区",
        },
        District {
            area_code: "110115",
            area_name: "北京市大兴区",
        },
    ])
}
