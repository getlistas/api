use actix_web::web::block as to_future;
use serde::{Deserialize, Serialize};
use validator::Validate;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::errors::Error;
use crate::lib::date;
use crate::lib::util::create_random_string;

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
  pub id: String,
  pub plan_id: String,
  pub checkout_id: String,
  pub status: String,
  pub updated_at: DateTime,

  // Subscription created attributes
  pub next_bill_at: Option<DateTime>,
  pub cancel_url: Option<String>,
  pub update_url: Option<String>,

  // Subscription cancelled attributes
  pub cancellation_effective_at: Option<DateTime>,
}

#[derive(Debug, Model, Validate, Serialize, Deserialize)]
pub struct User {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub password: String,
  #[validate(email)]
  pub email: String,
  #[validate(length(min = 1, max = 50))]
  pub slug: String,
  pub name: String,
  pub avatar: Option<String>,
  pub google_id: Option<String>,
  pub verification_token: Option<String>,
  pub password_reset_token: Option<String>,
  pub created_at: DateTime,
  pub updated_at: DateTime,
  pub verified_at: Option<DateTime>,
  pub locked_at: Option<DateTime>,
  pub verification_token_set_at: Option<DateTime>,
  pub password_reset_token_set_at: Option<DateTime>,
  pub subscription: Option<Subscription>,
}

impl User {
  pub fn is_premium(&self) -> bool {
    match self.subscription {
      Some(ref subscription) => {
        let expires_at = subscription.cancellation_effective_at;
        if expires_at.is_none() {
          return true;
        }
        let now = date::now();
        let expires_at = expires_at.unwrap();

        expires_at > now
      }
      None => false,
    }
  }

  pub async fn hash_password(password: String) -> Result<String, Error> {
    let hash = to_future(move || bcrypt::hash(password, bcrypt::DEFAULT_COST));

    match hash.await {
      Ok(hash) => Ok(hash),
      Err(err) => Err(Error::HashPassword(err)),
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
#[derive(Clone)]
pub struct UserID(pub ObjectId);

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPublic {
  pub id: String,
  pub email: String,
  pub name: String,
  pub slug: String,
}
