use actix_web::web::block as to_future;
use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::lib::util::create_random_string;
use crate::{errors, lib::date};

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub password: String,
    pub email: String,
    pub slug: String,
    pub name: String,
    pub avatar: Option<String>,

    // Oauth providers attributes
    pub google_id: Option<String>,

    pub verification_token: Option<String>,
    pub password_reset_token: Option<String>,

    pub created_at: DateTime,
    pub updated_at: DateTime,

    pub verified_at: Option<DateTime>,
    pub locked_at: Option<DateTime>,
    pub verification_token_set_at: Option<DateTime>,
    pub password_reset_token_set_at: Option<DateTime>,
}

impl User {
    pub async fn hash_password(password: String) -> Result<String, errors::ApiError> {
        let hash = to_future(move || bcrypt::hash(password, bcrypt::DEFAULT_COST));

        match hash.await {
            Ok(hash) => Ok(hash),
            Err(err) => Err(errors::ApiError::HashPassword(err)),
        }
    }

    pub fn is_password_match(&self, password: &str) -> bool {
        bcrypt::verify(password, &self.password).unwrap_or(false)
    }

    pub fn set_password_reset_token(&mut self) -> String {
        let now = date::now();
        let token = create_random_string(40);

        self.password_reset_token = Some(token.clone());
        self.password_reset_token_set_at = Some(now);

        token
    }

    pub fn unset_password_reset_token(&mut self) {
        self.password_reset_token = None;
        self.password_reset_token_set_at = None;
    }

    pub fn set_password(&mut self, password: String) {
        let password = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
        self.password = password;
    }

    pub fn to_display(&self) -> UserPublic {
        UserPublic {
            id: self.id.clone().unwrap().to_string(),
            email: self.email.clone(),
            name: self.name.clone(),
            slug: self.slug.clone(),
        }
    }
}

// This struct is used in actix extractors to retrieve the user ObjectID from
// the authentication token.
pub struct UserID(pub ObjectId);

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: String,
    pub email: String,
    pub name: String,
    pub slug: String,
}
