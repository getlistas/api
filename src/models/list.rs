use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JSON;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId, Bson};
use wither::mongodb::Database;
use wither::Model;

use crate::{errors::Error, lib::date};
use crate::{lib::util, models::resource::Resource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fork {
  pub from: ObjectId,
  pub at: DateTime,
}

impl Fork {
  pub fn to_json(&self) -> JSON {
    let this = self.clone();
    json!({
        "from": this.from.clone().to_hex(),
        "at": date::to_rfc3339(this.at)
    })
  }
}

#[derive(Debug, Model, Serialize, Deserialize)]
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

impl List {
  pub fn to_json(&self) -> JSON {
    let this = self.clone();
    json!({
        "id": this.id.clone().unwrap().to_hex(),
        "user": this.user.to_hex(),
        "title": this.title,
        "description": this.description,
        "tags": this.tags,
        "slug": this.slug,
        "is_public": this.is_public,
        "fork": this.fork.clone().map(|fork| fork.to_json()),
        "created_at": date::to_rfc3339(this.created_at),
        "updated_at": date::to_rfc3339(this.updated_at)
    })
  }

  pub async fn to_schema(&self, conn: &Database) -> Result<JSON, Error> {
    let id = self.id.clone().unwrap();
    let user_id = self.user.clone();
    let mut res = self.to_json();

    let resources_count: i64 = Resource::collection(conn)
      .count_documents(doc! { "list": &id }, None)
      .await
      .map_err(Error::MongoError)?;

    let uncompleted_resources_count: i64 = Resource::collection(conn)
      .count_documents(
        doc! {
            "list": &id,
            "completed_at": Bson::Null
        },
        None,
      )
      .await
      .map_err(Error::MongoError)?;

    let next_resource = Resource::find_next(conn, &user_id, &id).await?;
    let last_completed_resource = Resource::find_last_completed(conn, &user_id, &id).await?;
    let last_completed_at = last_completed_resource
      .map(|resource| resource.completed_at)
      .and_then(|completed_at| completed_at.map(date::to_rfc3339));

    res["resource_metadata"] = json!({
        "count": resources_count,
        "completed_count": resources_count - uncompleted_resources_count,
        "last_completed_at": last_completed_at,
        "next": next_resource.map(|resource| resource.to_json())
    });

    Ok(res)
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListUpdate {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tags: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub is_public: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
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
          .unwrap_or(vec![]),
      );
    }

    update.updated_at = Some(date::now());
    update
  }
}
