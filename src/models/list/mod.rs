pub mod queries;

use actix_web::web;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JSON;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId, Bson};
use wither::mongodb::Database;
use wither::Model;

use crate::models::integration::Integration;
use crate::models::list::queries::create_find_populated_query;
use crate::models::user::PublicUser;
use crate::models::Models;
use crate::Context;
use crate::{errors::Error, lib::date};
use crate::{lib::util, models::resource::Resource};

type Ctx = web::Data<Context>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fork {
  pub list: ObjectId,
  pub user: ObjectId,
}

impl Fork {
  pub fn to_json(&self) -> JSON {
    let this = self.clone();
    json!({
        "list": this.list.clone().to_hex(),
        "user": this.user.clone().to_hex(),
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
        "updated_at": date::to_rfc3339(this.updated_at),
        "archived_at": this.archived_at.map(date::to_rfc3339)
    })
  }

  pub async fn find_populated(
    models: &Models,
    user_id: &ObjectId,
  ) -> Result<Vec<PopulatedList>, Error> {
    let query = doc! { "user": user_id };
    let pipeline = create_find_populated_query(query);
    let lists = models.aggregate::<List, PopulatedList>(pipeline).await?;

    Ok(lists)
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

  pub async fn archive(&self, ctx: &Ctx) -> Result<(), Error> {
    let id = self.id.as_ref().unwrap();

    ctx
      .models
      .delete_many::<Resource>(doc! { "list": id, "completed_at": Bson::Null })
      .await?;

    self.remove_integrations(&ctx).await?;

    let update = doc! {
      "$set": {
        "archived_at": Bson::DateTime(date::now().into())
      }
    };

    ctx
      .models
      .find_one_and_update::<List>(doc! { "_id": id }, update, None)
      .await?;

    Ok(())
  }

  pub async fn remove(&self, ctx: &Ctx) -> Result<(), Error> {
    let id = self.id.as_ref().unwrap();

    ctx
      .models
      .delete_many::<Resource>(doc! { "list": id })
      .await?;

    self.remove_integrations(&ctx).await?;

    ctx.models.delete_one::<List>(doc! { "_id": id }).await?;

    Ok(())
  }

  pub async fn remove_integrations(&self, ctx: &Ctx) -> Result<(), Error> {
    let id = self.id.as_ref().unwrap();

    let integrations = ctx
      .models
      .find::<Integration>(doc! { "list": id }, None)
      .await?;

    let remove_integration_futures = integrations
      .into_iter()
      .map(move |integration| async move { integration.remove(&ctx).await });

    futures::stream::iter(remove_integration_futures)
      .buffer_unordered(50)
      .collect::<Vec<Result<(), Error>>>()
      .await
      .into_iter()
      .collect::<Result<(), Error>>()
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PopulatedList {
  pub id: ObjectId,
  pub user: ObjectId,
  pub title: String,
  pub slug: String,
  pub description: Option<String>,
  pub tags: Vec<String>,
  pub is_public: bool,
  pub created_at: DateTime,
  pub updated_at: DateTime,
  pub archived_at: Option<DateTime>,
  pub fork: Option<PopulatedFork>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PopulatedFork {
  // pub user: PublicUser,
  pub list: Option<ListFromPopulatedFork>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListFromPopulatedFork {
  pub id: Option<ObjectId>,
  pub title: Option<String>,
  pub slug: Option<String>,
}
