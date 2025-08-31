use actix_web::{error::ResponseError, HttpResponse};
use derive_more::Display;
use mongodb::error::Error as MongoError;
use serde_json::json;

#[derive(Debug, Display)]
pub enum ApiError {
    #[display(fmt = "Internal server error")]
    InternalError,

    #[display(fmt = "Bad request: {}", _0)]
    BadRequest(String),

    #[display(fmt = "Not found: {}", _0)]
    NotFound(String),

    #[display(fmt = "Database error: {}", _0)]
    DatabaseError(String),
}

impl From<MongoError> for ApiError {
    fn from(error: MongoError) -> Self {
        log::error!("MongoDB error: {}", error);
        ApiError::DatabaseError(error.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        log::error!("Anyhow error: {}", error);
        ApiError::InternalError
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::InternalError => {
                HttpResponse::InternalServerError().json(json!({
                    "error": "Internal server error"
                }))
            }
            ApiError::BadRequest(ref message) => {
                HttpResponse::BadRequest().json(json!({
                    "error": message
                }))
            }
            ApiError::NotFound(ref message) => {
                HttpResponse::NotFound().json(json!({
                    "error": message
                }))
            }
            ApiError::DatabaseError(ref message) => {
                HttpResponse::InternalServerError().json(json!({
                    "error": format!("Database error: {}", message)
                }))
            }
        }
    }
}
