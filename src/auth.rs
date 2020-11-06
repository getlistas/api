use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use alcoholic_jwt::{token_kid, validate, Validation, JWKS};
use serde::{Deserialize, Serialize};

use crate::errors::ApiError;
use crate::settings::Settings;

type ValidationResult = Result<bool, alcoholic_jwt::ValidationError>;
type ActixValidationResult = Result<ServiceRequest, actix_web::Error>;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,  // Expiration time (as UTC timestamp)
    iat: usize,  // Issued at (as UTC timestamp)
    iss: String, // Issuer
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
