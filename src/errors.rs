use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use serde_json::json;
use wither::bson;
use wither::mongodb;
use wither::mongodb::error::CommandError as MongoCommandError;
use wither::mongodb::error::Error as MongoError;
use wither::mongodb::error::ErrorKind as MongoErrorKind;
use wither::WitherError;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum ApiError {
  #[error("Failed to read application shared data")]
  ReadAppData(),

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

  #[error("Failed authentication google token")]
  GoogleAuthentication {},

  #[error("{0}")]
  HashPassword(#[from] actix_web::error::BlockingError<bcrypt::BcryptError>),

  #[error("Failed to parse URL")]
  ParseURL(),

  #[error("{0}")]
  SubscribeToRSS(#[from] reqwest::Error),
}

impl ApiError {
  fn get_codes(&self) -> (StatusCode, u16) {
    match *self {
      // 4XX
      ApiError::ParseURL() => (StatusCode::BAD_REQUEST, 4041),
      ApiError::ParseObjectID(_) => (StatusCode::BAD_REQUEST, 4042),
      ApiError::WitherError(WitherError::Mongo(MongoError { ref kind, .. })) => {
        let mongo_error = kind.as_ref();
        match mongo_error {
          MongoErrorKind::CommandError(MongoCommandError { code: 11000, .. }) => {
            (StatusCode::BAD_REQUEST, 4043)
          }
          _ => (StatusCode::INTERNAL_SERVER_ERROR, 4044),
        }
      }

      // 401
      ApiError::JWT(_) => (StatusCode::UNAUTHORIZED, 4015),
      ApiError::MissingAuthorizationToken {} => (StatusCode::UNAUTHORIZED, 4016),
      ApiError::GoogleAuthentication {} => (StatusCode::UNAUTHORIZED, 4017),

      // 5XX
      ApiError::WitherError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5001),
      ApiError::ReadAppData() => (StatusCode::INTERNAL_SERVER_ERROR, 5002),
      ApiError::MongoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5003),
      ApiError::HashPassword(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5004),
      ApiError::SubscribeToRSS(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5004),
    }
  }
}

impl actix_web::error::ResponseError for ApiError {
  fn error_response(&self) -> HttpResponse {
    let message = self.to_string();
    let (status_code, code) = self.get_codes();

    if status_code == StatusCode::INTERNAL_SERVER_ERROR {
      error!("Internal server error {}", &message);
    }

    let body = json!({ "code": code, "message": message });
    HttpResponseBuilder::new(status_code).json(body)
  }
}
