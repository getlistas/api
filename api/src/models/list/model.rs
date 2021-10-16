use futures::future::try_join_all;
use futures::try_join;
use serde::{Deserialize, Serialize};
use wither::bson::{self, doc, oid::ObjectId, Bson};
use wither::bson::{DateTime, Document};
use wither::mongodb::options::FindOneOptions;
use wither::mongodb::options::FindOptions;

use crate::models;
use crate::models::integration;
use crate::models::like;
use crate::models::list::List;
use crate::models::list::ListResourceMetadata;
use crate::models::list::PrivateList;
use crate::models::resource;
use crate::models::resource::Resource;
use crate::models::user;
use crate::models::Model as ModelTrait;
use crate::thirdparty::traer::Traer;
use crate::{database, thirdparty::rss::Rss};
use crate::{errors::Error, lib::date};

#[derive(Clone)]
pub struct Model {
  database: database::Database,
  user: user::model::Model,
  resource: resource::model::Model,
  integration: integration::model::Model,
  like: like::model::Model,
}

impl models::Model<List> for Model {
  fn get_database(&self) -> &database::Database {
    &self.database
  }
}

impl Model {
  pub fn new(database: database::Database, rss: Rss, traer: Traer) -> Self {
    let resource = resource::model::Model::new(database.clone(), traer);
    let user = user::model::Model::new(database.clone());
    let integration = integration::model::Model::new(database.clone(), rss);
    let like = like::model::Model::new(database.clone());

    Self {
      database,
      user,
      resource,
      integration,
      like,
    }
  }

  pub async fn to_private_schema(&self, list: &List) -> Result<PrivateList, Error> {
    let list_id = list.id.clone().expect("Failed to unwrap List ID");
    let user_id = list.user.clone();

    let (resource_metadata, forks_count, subscriptions_count, likes_count) = try_join!(
      self.get_resource_metadata(&user_id, &list_id),
      self.get_forks_count(&list_id),
      self.get_subscriptions_count(&list_id),
      self.get_likes_count(&list_id),
    )?;

    let private_list = PrivateList {
      id: list_id,
      user: list.user.clone(),
      title: list.title.clone(),
      slug: list.slug.clone(),
      description: list.description.clone(),
      tags: list.tags.clone(),
      is_public: list.is_public,
      created_at: list.created_at,
      updated_at: list.updated_at,
      last_activity_at: list.last_activity_at,
      archived_at: list.archived_at,
      fork: list.fork.clone().map(Into::into),
      forks_count,
      subscriptions_count,
      likes_count,
      resource_metadata,
    };

    Ok(private_list)
  }

  pub async fn get_private_lists(&self, query: Document) -> Result<Vec<PrivateList>, Error> {
    let sort = doc! { "created_at": 1 };
    let options = FindOptions::builder().sort(sort).build();
    let lists = self.find(query, Some(options)).await?;

    let lists = lists.iter().map(|list| self.to_private_schema(list));

    debug!("Querying list resources metadata");
    let lists = try_join_all(lists).await?;

    debug!("Returning private lists to the user");
    Ok(lists)
  }

  pub async fn get_resource_metadata(
    &self,
    user_id: &ObjectId,
    list_id: &ObjectId,
  ) -> Result<ListResourceMetadata, Error> {
    let (count, uncompleted_count, last_completed_resource, next_resource) = try_join!(
      self
        .resource
        .count(doc! { "user": &user_id, "list": &list_id }),
      self
        .resource
        .count(doc! { "user": &user_id, "list": &list_id, "completed_at": Bson::Null }),
      self.get_last_completed_resource(user_id, list_id),
      self.get_next_resource(user_id, list_id)
    )?;

    Ok(ListResourceMetadata {
      count,
      completed_count: count - uncompleted_count,
      last_completed_at: last_completed_resource.and_then(|resource| resource.completed_at),
      next: next_resource.map(Into::into),
    })
  }

  pub async fn archive(&self, list_id: &ObjectId) -> Result<(), Error> {
    self
      .resource
      .delete_many(doc! { "list": list_id, "completed_at": Bson::Null })
      .await?;

    self.remove_integrations(list_id).await?;

    let update = doc! {
      "$set": {
        "archived_at": Bson::DateTime(date::now().into())
      }
    };

    self
      .find_one_and_update(doc! { "_id": list_id }, update, None)
      .await?;

    Ok(())
  }

  pub async fn remove(&self, list_id: &ObjectId) -> Result<(), Error> {
    self.resource.delete_many(doc! { "list": list_id }).await?;
    self.remove_integrations(list_id).await?;
    self.delete_one(doc! { "_id": list_id }).await?;

    Ok(())
  }

  pub async fn remove_integrations(&self, list_id: &ObjectId) -> Result<(), Error> {
    let integrations = self
      .integration
      .find(doc! { "list": list_id }, None)
      .await?;

    let remove_integration_futures = integrations
      .iter()
      .map(move |integration| self.integration.remove(integration.id.as_ref().unwrap()));

    try_join_all(remove_integration_futures).await?;
    Ok(())
  }

  // TODO: Rename this method to "get_next_resource_position"
  pub async fn get_position_for_new_resource(&self, list_id: &ObjectId) -> Result<i32, Error> {
    let sort = doc! { "position": -1 };
    let options = FindOneOptions::builder().sort(sort).build();
    let last_resource = self
      .resource
      .find_one(doc! { "list": list_id }, Some(options))
      .await?;

    let position = last_resource
      .map(|resource| resource.position + 1)
      .unwrap_or(0);

    Ok(position)
  }

  pub async fn get_last_completed_resource(
    &self,
    user_id: &ObjectId,
    list_id: &ObjectId,
  ) -> Result<Option<Resource>, Error> {
    let query = doc! {
        "user": user_id,
        "list": list_id,
        "completed_at": { "$exists": true, "$ne": Bson::Null }
    };
    let sort = doc! { "completed_at": -1 };
    let options = FindOneOptions::builder().sort(sort).build();

    self.resource.find_one(query, Some(options)).await
  }

  pub async fn get_next_resource(
    &self,
    user_id: &ObjectId,
    list_id: &ObjectId,
  ) -> Result<Option<Resource>, Error> {
    let query = doc! { "user": user_id, "list": list_id, "completed_at": Bson::Null };
    let sort = doc! { "position": 1 };
    let options = FindOneOptions::builder().sort(sort).build();

    self.resource.find_one(query, Some(options)).await
  }

  pub async fn get_forks_count(&self, list_id: &ObjectId) -> Result<i64, Error> {
    self.count(doc! { "fork.list": list_id }).await
  }

  pub async fn get_subscriptions_count(&self, list_id: &ObjectId) -> Result<i64, Error> {
    self
      .integration
      .count(doc! { "listas_subscription.list": list_id })
      .await
  }

  pub async fn get_likes_count(&self, list_id: &ObjectId) -> Result<i64, Error> {
    self.like.count(doc! { "list": list_id }).await
  }

  pub async fn update_last_activity_at(&self, list_id: &ObjectId) -> Result<(), Error> {
    let update = doc! {
      "$set": {
        "last_activity_at": bson::to_bson(&date::now()).unwrap()
      }
    };

    self
      .update_one(doc! { "_id": list_id }, update, None)
      .await?;

    Ok(())
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceMetadata {
  pub count: i64,
  pub completed_count: i64,
  pub last_completed_at: Option<DateTime>,
}
