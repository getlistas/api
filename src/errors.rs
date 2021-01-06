use actix_web::dev::HttpResponseBuilder;
use actix_web::http;
use actix_web::HttpResponse;
use wither::bson;
use wither::mongodb;
use wither::WitherError;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum ApiError {
    #[error("Failed to read application shared Data")]
    ReadAppData(),

    #[error("Failed to parse request body")]
    ParseRequestBody(),

    #[error("{0}")]
    WitherError(#[from] WitherError),

    #[error("{0}")]
    MongoError(#[from] mongodb::error::Error),

    #[error("{0}")]
    ParseObjectID(#[from] bson::oid::Error),

    #[error("Authorization token missing")]
    MissingAuthorizationToken {},

    #[error("{0}")]
    JWT(#[from] jsonwebtoken::errors::Error),
}

impl actix_web::error::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .set_header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> http::StatusCode {
        match *self {
            ApiError::ReadAppData() => http::StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ParseRequestBody() => http::StatusCode::BAD_REQUEST,
            ApiError::WitherError(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::MongoError(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ParseObjectID(_) => http::StatusCode::BAD_REQUEST,
            ApiError::JWT(_) => http::StatusCode::UNAUTHORIZED,
            ApiError::MissingAuthorizationToken {} => http::StatusCode::UNAUTHORIZED,
        }
    }
}
