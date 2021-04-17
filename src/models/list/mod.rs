pub mod queries;

use actix_web::web;
use futures::future::try_join4;
use futures::future::try_join_all;
use futures::future::{try_join, try_join3};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::convert::From;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId, Bson};
use wither::mongodb::options::FindOneOptions;
use wither::mongodb::options::FindOptions;
use wither::Model;

use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_bson_datetime_option_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;
use crate::models::integration::Integration;
use crate::models::resource::PrivateResource;
use crate::models::user::PublicUser;
use crate::models::user::User;
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
  pub async fn find_populated(
    models: &Models,
    user_id: &ObjectId,
  ) -> Result<Vec<PopulatedListWithResourceMetadata>, Error> {
    let sort = doc! { "created_at": 1 };
    let options = FindOptions::builder().sort(sort).build();
    let mut lists = models
      .find::<List>(doc! { "user": &user_id }, Some(options))
      .await?;

    let lists = lists
      .iter_mut()
      .map(move |list| async move { list.to_schema(&models).await });

    debug!("Querying list resources metadata");
    let lists = try_join_all(lists).await?;

    debug!("Returning list to the user");
    Ok(lists)
  }

  pub async fn get_resource_metadata(
    models: &Models,
    list_id: &ObjectId,
  ) -> Result<ResourceMetadata, Error> {
    let (count, uncompleted_count, last_completed_resource) = try_join3(
      models.count::<Resource>(doc! { "list": &list_id }),
      models.count::<Resource>(doc! { "list": &list_id, "completed_at": Bson::Null }),
      Resource::find_last_completed(models, &list_id),
    )
    .await?;

    Ok(ResourceMetadata {
      count,
      completed_count: count - uncompleted_count,
      last_completed_at: last_completed_resource.and_then(|resource| resource.completed_at),
    })
  }

  pub async fn get_populated_fork(&self, models: &Models) -> Result<Option<PopulatedFork>, Error> {
    if self.fork.is_none() {
      return Ok(None);
    }

    let fork = self.fork.as_ref().unwrap();
    let user_id = &fork.user;
    let list_id = &fork.list;

    let (user, list) = try_join(
      models.find_by_id::<User>(user_id),
      models.find_by_id::<List>(list_id),
    )
    .await?;

    let populated_fork = PopulatedFork {
      list: list.map(Into::into),
      user: user.map(Into::into),
    };

    Ok(Some(populated_fork))
  }

  pub async fn to_schema(
    &self,
    models: &Models,
  ) -> Result<PopulatedListWithResourceMetadata, Error> {
    let list_id = self.id.clone().unwrap();

    let (metadata, next_resource, last_completed_resource, populated_fork) = try_join4(
      Self::get_resource_metadata(models, &list_id),
      Resource::find_next(models, &list_id),
      Resource::find_last_completed(models, &list_id),
      self.get_populated_fork(models),
    )
    .await?;

    Ok(PopulatedListWithResourceMetadata {
      id: self.id.clone().unwrap(),
      user: self.user.clone(),
      title: self.title.clone(),
      slug: self.slug.clone(),
      description: self.description.clone(),
      tags: self.tags.clone(),
      is_public: self.is_public,
      created_at: self.created_at,
      updated_at: self.updated_at,
      archived_at: self.archived_at,
      fork: populated_fork,
      resource_metadata: ResourceMetadataWithNextResource {
        count: metadata.count,
        completed_count: metadata.completed_count,
        last_completed_at: last_completed_resource.and_then(|resource| resource.completed_at),
        next: next_resource.map(Into::into),
      },
    })
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
      .iter()
      .map(move |integration| async move { integration.remove(&ctx).await });

    futures::stream::iter(remove_integration_futures)
      .buffer_unordered(50)
      .collect::<Vec<Result<(), Error>>>()
      .await
      .into_iter()
      .collect::<Result<(), Error>>()
  }

  pub async fn get_next_resource_position(
    models: &Models,
    list_id: &ObjectId,
  ) -> Result<i32, Error> {
    let sort = doc! { "position": -1 };
    let options = FindOneOptions::builder().sort(sort).build();
    let last_resource = models
      .find_one::<Resource>(doc! { "list": list_id }, Some(options))
      .await?;

    let position = last_resource
      .map(|resource| resource.position + 1)
      .unwrap_or(0);

    Ok(position)
  }
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
          .unwrap_or(vec![]),
      );
    }

    update.updated_at = Some(date::now());
    update
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PopulatedList {
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  #[serde(alias = "_id")]
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
  pub fork: Option<PopulatedFork>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PopulatedFork {
  pub list: Option<PublicList>,
  pub user: Option<PublicUser>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceMetadata {
  pub count: i64,
  pub completed_count: i64,
  pub last_completed_at: Option<DateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceMetadataWithNextResource {
  pub count: i64,
  pub completed_count: i64,
  #[serde(serialize_with = "serialize_bson_datetime_option_as_iso_string")]
  pub last_completed_at: Option<DateTime>,
  pub next: Option<PrivateResource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PopulatedListWithResourceMetadata {
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
  pub fork: Option<PopulatedFork>,
  pub resource_metadata: ResourceMetadataWithNextResource,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrivateList {
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub id: ObjectId,
  pub user: ObjectId,
  pub title: String,
  pub slug: String,
  pub description: Option<String>,
  pub tags: Vec<String>,
  pub is_public: bool,
  pub fork: Option<PrivateFork>,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub created_at: DateTime,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub updated_at: DateTime,
  #[serde(serialize_with = "serialize_bson_datetime_option_as_iso_string")]
  pub archived_at: Option<DateTime>,
}

impl From<List> for PrivateList {
  fn from(list: List) -> Self {
    Self {
      id: list.id.unwrap(),
      user: list.user,
      title: list.title,
      slug: list.slug,
      description: list.description,
      tags: list.tags,
      is_public: list.is_public,
      fork: list.fork.map(Into::into),
      created_at: list.created_at,
      updated_at: list.updated_at,
      archived_at: list.archived_at,
    }
  }
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
  pub fork: Option<PrivateFork>,
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
pub struct PrivateFork {
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub list: ObjectId,
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  pub user: ObjectId,
}

impl From<Fork> for PrivateFork {
  fn from(fork: Fork) -> Self {
    Self {
      user: fork.user,
      list: fork.list,
    }
  }
}
