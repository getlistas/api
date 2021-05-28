pub mod model;
pub mod queries;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::convert::From;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::lib::date;
use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_bson_datetime_option_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;
use crate::lib::util;
use crate::models::resource::PrivateResource;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fork {
  pub list: ObjectId,
  pub user: ObjectId,
}

#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{ "user": 1 }"#))]
#[model(index(
  keys = r#"doc!{ "user": 1, "slug": 1 }"#,
  options = r#"doc!{ "unique": true }"#
))]
pub struct List {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub user: ObjectId,
  pub title: String,
  pub slug: String,
  pub description: Option<String>,
  pub tags: Vec<String>,
  pub is_public: bool,
  pub fork: Option<Fork>,
  pub created_at: DateTime,
  pub updated_at: DateTime,
  pub archived_at: Option<DateTime>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct ListUpdate {
  pub title: Option<String>,
  pub description: Option<String>,
  pub tags: Option<Vec<String>>,
  pub is_public: Option<bool>,
  pub updated_at: Option<DateTime>,
}

impl ListUpdate {
  pub fn new(update: &mut Self) -> &mut Self {
    if update.tags.is_some() {
      update.tags = Some(
        update
          .tags
          .clone()
          .map(util::sanitize_tags)
          .unwrap_or_default(),
      );
    }

    update.updated_at = Some(date::now());
    update
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrivateList {
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub id: ObjectId,
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub user: ObjectId,
  pub title: String,
  pub slug: String,
  pub description: Option<String>,
  pub tags: Vec<String>,
  pub is_public: bool,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub created_at: DateTime,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub updated_at: DateTime,
  #[serde(serialize_with = "serialize_bson_datetime_option_as_iso_string")]
  pub archived_at: Option<DateTime>,
  pub fork: Option<PublicFork>,
  pub forks_count: i64,
  pub subscriptions_count: i64,
  pub likes_count: i64,
  pub resource_metadata: ListResourceMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResourceMetadata {
  pub count: i64,
  pub completed_count: i64,
  #[serde(serialize_with = "serialize_bson_datetime_option_as_iso_string")]
  pub last_completed_at: Option<DateTime>,
  pub next: Option<PrivateResource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicList {
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub id: ObjectId,
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub user: ObjectId,
  pub title: String,
  pub slug: String,
  pub description: Option<String>,
  pub tags: Vec<String>,
  pub fork: Option<PublicFork>,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub created_at: DateTime,
}

impl From<List> for PublicList {
  fn from(list: List) -> Self {
    Self {
      id: list.id.unwrap(),
      user: list.user,
      title: list.title,
      slug: list.slug,
      description: list.description,
      tags: list.tags,
      fork: list.fork.map(Into::into),
      created_at: list.created_at,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicFork {
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub list: ObjectId,
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub user: ObjectId,
}

impl From<Fork> for PublicFork {
  fn from(fork: Fork) -> Self {
    Self {
      user: fork.user,
      list: fork.list,
    }
  }
}
