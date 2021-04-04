use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::skip_serializing_none;
use validator::Validate;
use wither::bson::{doc, oid::ObjectId, Bson};
use wither::bson::{DateTime, Document};
use wither::mongodb::options::FindOneOptions;
use wither::mongodb::Database;
use wither::Model;

use crate::errors::Error;
use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_bson_datetime_option_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;
use crate::lib::{date, util};
use crate::models::Models;

#[derive(Debug, Model, Validate, Serialize, Deserialize)]
pub struct Resource {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub user: ObjectId,
  pub list: ObjectId,
  #[validate(url)]
  pub url: String,
  pub title: String,
  pub position: i32,
  pub description: Option<String>,
  pub thumbnail: Option<String>,
  pub tags: Vec<String>,
  pub created_at: DateTime,
  pub updated_at: DateTime,
  pub completed_at: Option<DateTime>,
}

impl Resource {
  pub async fn find_last(
    conn: &Database,
    user_id: &ObjectId,
    list_id: &ObjectId,
  ) -> Result<Option<Self>, Error> {
    let query = doc! { "user": user_id, "list": list_id };
    let sort = doc! { "position": -1 };
    let options = FindOneOptions::builder().sort(Some(sort)).build();
    Self::find_one(conn, query, Some(options))
      .await
      .map_err(Error::WitherError)
  }

  pub async fn find_next(models: &Models, list_id: &ObjectId) -> Result<Option<Self>, Error> {
    let query = doc! { "list": list_id, "completed_at": Bson::Null };
    let sort = doc! { "position": 1 };
    let options = FindOneOptions::builder().sort(sort).build();

    models.find_one::<Resource>(query, Some(options)).await
  }

  pub async fn find_last_completed(
    models: &Models,
    list_id: &ObjectId,
  ) -> Result<Option<Self>, Error> {
    let query = doc! {
        "list": list_id,
        "completed_at": { "$exists": true, "$ne": Bson::Null }
    };
    let sort = doc! { "completed_at": -1 };
    let options = FindOneOptions::builder().sort(sort).build();

    models.find_one(query, Some(options)).await
  }

  pub async fn get_position(conn: &Database, query: Document) -> Result<Option<i32>, Error> {
    let resource = Self::find_one(conn, query, None)
      .await
      .map_err(Error::WitherError)?;

    match resource {
      Some(resource) => Ok(Some(resource.position)),
      None => Ok(None),
    }
  }

  pub async fn find_by_url(
    conn: &Database,
    user_id: &ObjectId,
    url: String,
  ) -> Result<Option<Self>, Error> {
    let query = doc! { "user": user_id, "url": url };

    Self::find_one(conn, query, None)
      .await
      .map_err(Error::WitherError)
  }

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
        "completed_at": this.completed_at.map(|date| date::to_rfc3339(date))
    })
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
          .unwrap_or(vec![]),
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
  pub title: String,
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
