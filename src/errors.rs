use actix_web::dev::HttpResponseBuilder;
use actix_web::error::BlockingError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use lettre_email::error::Error as LettreEmailError;
use serde_json::json;
use wither::bson;
use wither::mongodb;
use wither::mongodb::error::CommandError as MongoCommandError;
use wither::mongodb::error::Error as MongoError;
use wither::mongodb::error::ErrorKind as MongoErrorKind;
use wither::WitherError;

use crate::mailer::MailerError;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum Error {
  #[error("Failed to read application shared data")]
  ReadAppData(),

  #[error("{0}")]
  WitherError(#[from] WitherError),

  #[error("{0}")]
  MongoError(#[from] mongodb::error::Error),

  #[error("{0}")]
  ParseObjectID(#[from] bson::oid::Error),

  #[error("{0}")]
  SerializeMongoResponse(#[from] bson::de::Error),

  #[error("Authorization token missing")]
  MissingAuthorizationToken {},

  #[error("{0}")]
  JWT(#[from] jsonwebtoken::errors::Error),

  #[error("Failed authenticating Google token")]
  GoogleAuthentication {},

  #[error("{0}")]
  HashPassword(#[from] BlockingError<bcrypt::BcryptError>),

  #[error("Failed to parse URL")]
  ParseURL(),

  #[error("Failed to parse query string {0}")]
  ParseQueryString(#[from] serde_qs::Error),

  #[error("{0}")]
  ContactRSSIntegration(#[from] reqwest::Error),

  #[error("RSS Integration error: {0}")]
  RSSIntegration(String),

  #[error("Error sending email")]
  SendEmail(#[from] MailerError),

  #[error("Failed to build email {0}")]
  BuildEmail(#[from] LettreEmailError),
}

impl Error {
  fn get_codes(&self) -> (StatusCode, u16) {
    match *self {
      // 4XX
      Error::ParseURL() => (StatusCode::BAD_REQUEST, 4041),
      Error::ParseQueryString(_) => (StatusCode::BAD_REQUEST, 4042),
      Error::ParseObjectID(_) => (StatusCode::BAD_REQUEST, 4043),
      Error::WitherError(WitherError::Mongo(MongoError { ref kind, .. })) => {
        let mongo_error = kind.as_ref();
        match mongo_error {
          MongoErrorKind::CommandError(MongoCommandError { code: 11000, .. }) => {
            (StatusCode::BAD_REQUEST, 4044)
          }
          _ => (StatusCode::INTERNAL_SERVER_ERROR, 4045),
        }
      }

      // 401
      Error::JWT(_) => (StatusCode::UNAUTHORIZED, 4015),
      Error::MissingAuthorizationToken {} => (StatusCode::UNAUTHORIZED, 4016),
      Error::GoogleAuthentication {} => (StatusCode::UNAUTHORIZED, 4017),

      // 5XX
      Error::WitherError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5001),
      Error::ReadAppData() => (StatusCode::INTERNAL_SERVER_ERROR, 5002),
      Error::MongoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5003),
      Error::HashPassword(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5004),
      Error::ContactRSSIntegration(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5005),
      Error::RSSIntegration(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5006),
      Error::SendEmail(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5007),
      Error::BuildEmail(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5008),
      Error::SerializeMongoResponse(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5009),
    }
  }
}

impl actix_web::error::ResponseError for Error {
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
