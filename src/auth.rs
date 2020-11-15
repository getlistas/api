use actix_web::dev::Payload;
use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use alcoholic_jwt::{token_kid, validate, Validation, JWKS};
use futures::future;
use serde::{Deserialize, Serialize};

use crate::components::user::model::ReqUser;
use crate::errors::ApiError;
use crate::settings::Settings;

type ValidationResult = Result<bool, alcoholic_jwt::ValidationError>;
type ActixValidationResult = Result<ServiceRequest, actix_web::Error>;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,  // Expiration time (as UTC timestamp)
    iat: usize,  // Issued at (as UTC timestamp)
    iss: String, // Issuer
    user: ReqUser,
}

pub async fn validator(req: ServiceRequest, credentials: BearerAuth) -> ActixValidationResult {
    let token = credentials.token();
    let settings = req
        .app_data::<actix_web::web::Data<Settings>>()
        .ok_or(ApiError::ReadAppData())?;

    let jwks = get_jwks().await.expect("failed to fetch jwks");
    let authority = settings.auth.authority.clone();
    let is_valid_token = validate_token(jwks, token, authority).await;

    let config = Config::default()
        .realm("Restricted area")
        .scope("email photo");

    match is_valid_token {
        Ok(true) => Ok(req),
        // TODO: Improve from config error handling
        _ => Err(AuthenticationError::from(config).into()),
    }
}

pub async fn validate_token(jwks: JWKS, token: &str, authority: String) -> ValidationResult {
    let validations = vec![
        Validation::Issuer(authority),
        Validation::SubjectPresent,
        Validation::NotExpired,
    ];

    let kid = token_kid(&token)?.ok_or(alcoholic_jwt::ValidationError::InvalidComponents)?;

    let key = jwks
        .find(&kid)
        .ok_or(alcoholic_jwt::ValidationError::InvalidSignature)?;

    let res = validate(token, key, validations);
    Ok(res.is_ok())
}

async fn get_jwks() -> Result<JWKS, reqwest::Error> {
    // TODO: Implement an in-memory cache
    reqwest::get("https://doneq.us.auth0.com/.well-known/jwks.json")
        .await?
        .json::<JWKS>()
        .await
}

impl actix_web::FromRequest for ReqUser {
    type Config = ();
    type Error = ApiError;
    type Future = future::Ready<Result<Self, ApiError>>;

    fn from_request(req: &actix_web::HttpRequest, _payload: &mut Payload) -> Self::Future {
        let token_result: Result<String, ApiError> = req
            .headers()
            .get("authorization")
            .and_then(|header| header.to_str().ok())
            .map(|header| header.replace("Bearer ", ""))
            .ok_or_else(|| ApiError::MissingAuthorizationToken {});

        let token = match token_result {
            Ok(token) => token,
            Err(err) => return future::err(err),
        };

        let decoded_token_result =
            jsonwebtoken::dangerous_insecure_decode_with_validation::<Claims>(
                &token,
                &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256),
            )
            .map_err(ApiError::JWT);

        match decoded_token_result {
            Ok(token_decoded) => future::ok(token_decoded.claims.user),
            Err(err) => return future::err(err),
        }
    }
}
