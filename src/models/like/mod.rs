pub mod model;

use serde::{Deserialize, Serialize};
use validator::Validate;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;

#[derive(Debug, Clone, Model, Validate, Serialize, Deserialize)]
#[model(index(keys=r#"doc!{ "user": 1, "list": 1 }"#, options=r#"doc!{ "unique": true }"#))]
pub struct Like {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub user: ObjectId,
  pub list: ObjectId,
  pub created_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicLike {
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub id: ObjectId,
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub user: ObjectId,
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub list: ObjectId,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub created_at: DateTime,
}

impl From<Like> for PublicLike {
  fn from(like: Like) -> Self {
    Self {
      id: like.id.unwrap(),
      user: like.user,
      list: like.list,
      created_at: like.created_at,
    }
  }
}
