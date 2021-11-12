pub mod model;

use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::skip_serializing_none;
use url::Url;
use validator::Validate;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_bson_datetime_option_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;
use crate::lib::util::parse_url;
use crate::lib::{date, util};

#[derive(Debug, Clone, Model, Validate, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{ "user": 1 }"#))]
#[model(index(keys = r#"doc!{ "user": 1, "list": 1, "completed_at": 1 }"#))]
#[model(index(keys = r#"doc!{ "user": 1, "list": 1, "url": 1 }"#))]
pub struct Resource {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub user: ObjectId,
  pub list: ObjectId,
  #[validate(url)]
  pub url: String,
  pub title: Option<String>,
  pub position: i32,
  pub description: Option<String>,
  pub thumbnail: Option<String>,
  pub tags: Vec<String>,
  pub html: Option<String>,
  pub text: Option<String>,
  pub author: Option<String>,
  pub length: Option<i32>,
  pub publisher: Option<String>,
  pub created_at: DateTime,
  pub updated_at: DateTime,
  pub completed_at: Option<DateTime>,
  pub populated_at: Option<DateTime>,
}

impl Resource {
  // TODO: Use serde to parse this struct to JSON
  pub fn to_json(&self) -> serde_json::Value {
    let this = self.clone();
    json!({
        "id": this.id.clone().unwrap().to_hex(),
        "user": this.user.to_hex(),
        "list": this.list.to_hex(),
        "url": this.url,
        "title": this.title,
        "description": this.description,
        "thumbnail": this.thumbnail,
        "position": this.position,
        "tags": this.tags,
        "created_at": date::to_rfc3339(this.created_at),
        "updated_at": date::to_rfc3339(this.updated_at),
        "completed_at": this.completed_at.map(date::to_rfc3339)
    })
  }

  pub fn get_url(&self) -> Url {
    let url = self.url.clone();
    let url = parse_url(url.as_str()).expect("Resource to have a valid URL");
    url
  }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceUpdate {
  pub list: Option<ObjectId>,
  pub url: Option<String>,
  pub title: Option<String>,
  pub description: Option<String>,
  pub tags: Option<Vec<String>>,
  pub updated_at: Option<DateTime>,
}

impl ResourceUpdate {
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
pub struct PrivateResource {
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub id: ObjectId,
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub user: ObjectId,
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub list: ObjectId,
  pub url: String,
  pub title: Option<String>,
  pub position: i32,
  pub description: Option<String>,
  pub thumbnail: Option<String>,
  pub tags: Vec<String>,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub created_at: DateTime,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub updated_at: DateTime,
  #[serde(serialize_with = "serialize_bson_datetime_option_as_iso_string")]
  pub completed_at: Option<DateTime>,
}

impl From<Resource> for PrivateResource {
  fn from(resource: Resource) -> Self {
    Self {
      id: resource.id.unwrap(),
      user: resource.user,
      list: resource.list,
      url: resource.url,
      title: resource.title,
      position: resource.position,
      description: resource.description,
      thumbnail: resource.thumbnail,
      tags: resource.tags,
      created_at: resource.created_at,
      updated_at: resource.updated_at,
      completed_at: resource.completed_at,
    }
  }
}
