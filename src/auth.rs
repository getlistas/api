use actix_web::dev::Payload;
use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use futures::future;
use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::lib::token;
use crate::models::user::User;
use crate::settings::Settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserFromToken {
  pub id: String,
  pub email: String,
  pub name: String,
  pub slug: String,
}

impl From<token::Claims> for UserFromToken {
  fn from(claims: token::Claims) -> Self {
    Self {
      id: claims.user.id,
      email: claims.user.email,
      name: claims.user.name,
      slug: claims.user.slug,
    }
  }
}

impl From<User> for UserFromToken {
  fn from(user: User) -> Self {
    Self {
      id: user.id.unwrap().to_hex(),
      email: user.email,
      name: user.name,
      slug: user.slug,
    }
  }
}

type ActixValidationResult = Result<ServiceRequest, actix_web::Error>;

// pub fn middleware<F, O>() -> HttpAuthentication<BearerAuth, F>
// where
//     F: Fn(ServiceRequest, BearerAuth) -> O,
//     O: Future<Output = Result<ServiceRequest, actix_web::Error>>,
// {
//     HttpAuthentication::bearer(token_validator)
// }

pub async fn validator(req: ServiceRequest, credentials: BearerAuth) -> ActixValidationResult {
  let settings = req
    .app_data::<actix_web::web::Data<Settings>>()
    .ok_or(Error::ReadAppData())?;

  let private_key = settings.auth.secret.as_str();
  let token = credentials.token();
  let token_date = token::decode_token(token, private_key);

  match token_date {
    Ok(_) => Ok(req),
    Err(err) => Err(actix_web::error::ErrorUnauthorized(err)),
  }
}

impl actix_web::FromRequest for UserFromToken {
  type Config = ();
  type Error = Error;
  type Future = future::Ready<Result<Self, Error>>;

  fn from_request(req: &actix_web::HttpRequest, _payload: &mut Payload) -> Self::Future {
    let token_result: Result<String, Error> = req
      .headers()
      .get("authorization")
      .and_then(|header| header.to_str().ok())
      .map(|header| header.replace("Bearer ", ""))
      .ok_or(Error::MissingAuthorizationToken {});

    let token = match token_result {
      Ok(token) => token,
      Err(err) => return future::err(err),
    };

    let token_payload = token::get_token_payload(token.as_str());

    match token_payload.map_err(Error::Jwt) {
      Ok(payload) => future::ok(payload.claims.into()),
      Err(err) => future::err(err),
    }
  }
}

#[derive(Clone)]
pub struct UserID(pub ObjectId);

impl actix_web::FromRequest for UserID {
  type Config = ();
  type Error = Error;
  type Future = future::Ready<Result<Self, Error>>;

  fn from_request(req: &actix_web::HttpRequest, _payload: &mut Payload) -> Self::Future {
    let token: Result<String, Error> = req
      .headers()
      .get("authorization")
      .and_then(|header| header.to_str().ok())
      .map(|header| header.replace("Bearer ", ""))
      .ok_or(Error::MissingAuthorizationToken {});

    let token = match token {
      Ok(token) => token,
      Err(err) => return future::err(err),
    };

    let payload = token::get_token_payload(token.as_str());

    match payload.map_err(Error::Jwt) {
      Ok(payload) => {
        let claims = payload.claims;
        let user_id = ObjectId::with_string(claims.user.id.as_str()).unwrap();
        future::ok(UserID(user_id))
      }
      Err(err) => future::err(err),
    }
  }
}

pub struct AuthenticationMetadata {
  pub is_authenticated: bool,
  pub user_id: Option<ObjectId>,
}

impl AuthenticationMetadata {
  fn unauthorized() -> Self {
    Self {
      is_authenticated: false,
      user_id: None,
    }
  }
}

impl actix_web::FromRequest for AuthenticationMetadata {
  type Config = ();
  type Error = Error;
  type Future = future::Ready<Result<Self, Error>>;

  fn from_request(req: &actix_web::HttpRequest, _payload: &mut Payload) -> Self::Future {
    let token: Result<String, Error> = req
      .headers()
      .get("authorization")
      .and_then(|header| header.to_str().ok())
      .map(|header| header.replace("Bearer ", ""))
      .ok_or(Error::MissingAuthorizationToken {});

    let token = match token {
      Ok(token) => token,
      Err(_) => return future::ok(AuthenticationMetadata::unauthorized()),
    };

    let settings = req
      .app_data::<actix_web::web::Data<Settings>>()
      .ok_or(Error::ReadAppData());

    let private_key = match settings {
      Ok(settings) => settings.auth.secret.as_str(),
      Err(err) => return future::err(err),
    };

    let payload = token::decode_token(token.as_str(), private_key);

    match payload.map_err(Error::Jwt) {
      Ok(payload) => {
        let claims = payload.claims;
        let user_id = ObjectId::with_string(claims.user.id.as_str()).unwrap();
        let authentication = AuthenticationMetadata {
          is_authenticated: true,
          user_id: Some(user_id),
        };
        future::ok(authentication)
      }
      Err(_) => future::ok(AuthenticationMetadata::unauthorized()),
    }
  }
}
