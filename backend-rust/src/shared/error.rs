use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Internal,
    NotFound,
    Validation(String),
    Conflict(String),
    #[allow(dead_code)]
    DependencyUnavailable(String),
    #[allow(dead_code)]
    Unauthorized,
    #[allow(dead_code)]
    Forbidden,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    success: bool,
    code: &'static str,
    message: String,
    details: Option<String>,
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::DependencyUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Internal => "INTERNAL_ERROR",
            Self::NotFound => "NOT_FOUND",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::Conflict(_) => "CONFLICT",
            Self::DependencyUnavailable(_) => "DEPENDENCY_UNAVAILABLE",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::Internal => "服务器内部错误".to_string(),
            Self::NotFound => "资源不存在".to_string(),
            Self::Validation(message) => message.clone(),
            Self::Conflict(message) => message.clone(),
            Self::DependencyUnavailable(_) => "依赖服务不可用".to_string(),
            Self::Unauthorized => "未授权访问".to_string(),
            Self::Forbidden => "禁止访问".to_string(),
        }
    }

    fn details(&self) -> Option<String> {
        match self {
            Self::Validation(message) => Some(message.clone()),
            Self::Conflict(message) => Some(message.clone()),
            Self::DependencyUnavailable(message) => Some(message.clone()),
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorBody {
            success: false,
            code: self.code(),
            message: self.message(),
            details: self.details(),
        };

        (status, Json(body)).into_response()
    }
}
