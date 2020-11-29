use jsonwebtoken::dangerous_insecure_decode_with_validation;
use serde::{Deserialize, Serialize};

use crate::user::model::User;
use crate::user::model::UserPublic;

type TokenResult = Result<jsonwebtoken::TokenData<Claims>, jsonwebtoken::errors::Error>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    exp: usize, // Expiration time (as UTC timestamp). validate_exp defaults to true in validation
    iat: usize, // Issued at (as UTC timestamp)
    pub user: UserPublic,
}

impl Claims {
    pub fn new(user: &User) -> Self {
        Self {
            exp: (chrono::Local::now() + chrono::Duration::days(7)).timestamp() as usize,
            iat: chrono::Local::now().timestamp() as usize,
            user: user.to_display(),
        }
    }

    pub fn to_public_user(&self) -> UserPublic {
        UserPublic {
            id: self.user.id.clone(),
            email: self.user.email.clone(),
            name: self.user.name.clone(),
            slug: self.user.slug.clone(),
        }
    }
}

pub fn create_token(user: &User) -> String {
    let header = jsonwebtoken::Header::default();
    // TODO: Save private key or secret somewhere safe
    let encoding_key = jsonwebtoken::EncodingKey::from_secret("secret".as_ref());
    let claims = Claims::new(user);

    jsonwebtoken::encode(&header, &claims, &encoding_key).unwrap()
}

pub fn decode_token(token: &str) -> TokenResult {
    let validation = jsonwebtoken::Validation::default();
    // TODO: Save private key or secret somewhere safe
    let decoding_key = jsonwebtoken::DecodingKey::from_secret("secret".as_ref());

    jsonwebtoken::decode::<Claims>(&token, &decoding_key, &validation)
}

pub fn get_token_payload(token: &str) -> TokenResult {
    let validation = jsonwebtoken::Validation::default();

    dangerous_insecure_decode_with_validation::<Claims>(&token, &validation)
}
