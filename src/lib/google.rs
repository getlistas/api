use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use crate::errors::Error;

// Read more about this implementation
// https://developers.google.com/identity/sign-in/web/backend-auth

const HOSTED_DOMAINS: [&str; 0] = [];

// https://github.com/wyyerd/google-signin-rs/blob/master/src/token.rs
#[derive(Debug, Deserialize)]
pub struct GoogleToken<EF = bool, TM = u64> {
  // These six fields are included in all Google ID Tokens.
  pub iss: String,
  pub sub: String,
  pub azp: String,
  pub aud: String,
  pub iat: TM,
  pub exp: TM,

  // This value indicates the user belongs to a Google Hosted Domain
  pub hd: Option<String>,

  // These seven fields are only included when the user has granted the
  // "profile" and "email" OAuth scopes to the application.
  pub email: Option<String>,
  pub email_verified: Option<EF>, // eg. "true" (but unusually as a string)
  pub name: Option<String>,
  pub picture: Option<String>,
  pub given_name: Option<String>,
  pub family_name: Option<String>,
  pub locale: Option<String>,
}

impl GoogleToken {
  // Check the issuer, audiences, and (optionally) hosted domains of the GoogleToken.
  // Returns false if the client has no configured audiences.
  pub fn is_valid(&self, audiences: [&str; 1]) -> bool {
    // Check the id was authorized by google
    match self.iss.as_str() {
      "accounts.google.com" | "https://accounts.google.com" => {}
      _ => return false,
    }

    // Check the token belongs to the Listas application
    if !audiences.is_empty() && !audiences.contains(&self.aud.as_str()) {
      return false;
    }

    // Check the token belongs to the hosted domain(s)
    if !HOSTED_DOMAINS.is_empty() {
      match self.hd {
        Some(ref domain) if HOSTED_DOMAINS.contains(&domain.as_str()) => {}
        _ => return false,
      }
    }

    true
  }
}

#[derive(Debug, Clone, Deserialize)]
struct CertsObject {
  keys: Vec<Cert>,
}

#[derive(Debug, Clone, Deserialize)]
struct Cert {
  kid: String,
  e: String,
  kty: String,
  alg: String,
  n: String,
  r#use: String,
}

pub async fn validate(token: &str, client_id: &str) -> Result<GoogleToken, Error> {
  let unverified_header = jsonwebtoken::decode_header(token).unwrap();
  let kid = unverified_header.kid.unwrap();
  let certs = get_certs().await.unwrap();

  let cert = certs.into_iter().find(|cert| cert.kid == kid);

  let cert = match cert {
    Some(cert) => cert,
    None => {
      return Err(Error::GoogleAuthentication {});
    }
  };

  let audiences = [client_id];
  let mut validation = Validation::new(Algorithm::RS256);
  validation.set_audience(&audiences);

  let token_data = jsonwebtoken::decode::<GoogleToken>(
    token,
    &DecodingKey::from_rsa_components(&cert.n, &cert.e),
    &validation,
  );

  let token_data = match token_data {
    Ok(token_data) => token_data,
    Err(_) => {
      return Err(Error::GoogleAuthentication {});
    }
  };

  let google_token = token_data.claims;

  if google_token.is_valid(audiences) {
    Ok(google_token)
  } else {
    Err(Error::GoogleAuthentication {})
  }
}

#[derive(Debug, Deserialize)]
struct Certs {
  keys: Vec<Cert>,
}

async fn get_certs() -> Result<Vec<Cert>, reqwest::Error> {
  let res = reqwest::get("https://www.googleapis.com/oauth2/v3/certs")
    .await?
    .json::<Certs>()
    .await;

  res.map(|res| res.keys)
}
