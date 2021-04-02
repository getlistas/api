use actix_web::web::block as to_future;
use inflector::cases::snakecase::to_snake_case;
use serde::{Deserialize, Serialize};
use std::convert::From;
use validator::Validate;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::errors::Error;
use crate::lib::date;
use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;
use crate::lib::util::create_random_string;
use crate::lib::util::to_slug_case;

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
  // pub fn is_premium(&self) -> bool {
  //   match self.subscription {
  //     Some(ref subscription) => {
  //       let expires_at = subscription.cancellation_effective_at;
  //       if expires_at.is_none() {
  //         return true;
  //       }
  //       let now = date::now();
  //       let expires_at = expires_at.unwrap();

  //       expires_at > now
  //     }
  //     None => false,
  //   }
  // }

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

  pub fn create_slug(email: &str) -> String {
    // TODO: Validate email to unwrap here safely.
    let prefix = email.split('@').next().unwrap();
    let slug = to_slug_case(prefix.to_owned());
    let slug = to_snake_case(slug.as_str());
    let random_string = create_random_string(5).to_lowercase();
    format!("{}_{}", slug, random_string)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUser {
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  #[serde(alias = "_id")]
  pub id: ObjectId,
  pub slug: String,
  pub name: String,
  pub avatar: Option<String>,
}

impl From<User> for PublicUser {
  fn from(user: User) -> Self {
    Self {
      id: user.id.unwrap(),
      slug: user.slug,
      name: user.name,
      avatar: user.avatar,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateUser {
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub id: ObjectId,
  pub email: String,
  pub slug: String,
  pub name: String,
  pub avatar: Option<String>,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub created_at: DateTime,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub updated_at: DateTime,
}

impl From<User> for PrivateUser {
  fn from(user: User) -> Self {
    Self {
      id: user.id.unwrap(),
      email: user.email,
      slug: user.slug,
      name: user.name,
      avatar: user.avatar,
      created_at: user.created_at,
      updated_at: user.updated_at,
    }
  }
}

// This struct is used in actix extractors to retrieve the user ObjectID from
// the authentication token.
// TODO: Move this to auth source code.
#[derive(Clone)]
pub struct UserID(pub ObjectId);
