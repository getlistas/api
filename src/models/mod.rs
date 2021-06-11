pub mod integration;
pub mod like;
pub mod list;
pub mod resource;
pub mod user;

use async_trait::async_trait;
use futures::stream::TryStreamExt;
use serde::{de::DeserializeOwned, ser::Serialize};
use wither::bson::doc;
use wither::bson::from_bson;
use wither::bson::Bson;
use wither::bson::Document;
use wither::bson::{self, oid::ObjectId};
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::mongodb::options::FindOneOptions;
use wither::mongodb::options::FindOptions;
use wither::mongodb::options::UpdateOptions;
use wither::mongodb::results::DeleteResult;
use wither::mongodb::results::UpdateResult;

use crate::database::Database;
use crate::errors::Error;
use crate::thirdparty::rss::Rss;

#[derive(Clone)]
pub struct Models {
  pub user: user::model::Model,
  pub list: list::model::Model,
  pub resource: resource::model::Model,
  pub integration: integration::model::Model,
  pub like: like::model::Model,
}

impl Models {
  pub fn new(database: Database, rss: Rss) -> Self {
    let list = list::model::Model::new(database.clone(), rss.clone());
    let resource = resource::model::Model::new(database.clone());
    let user = user::model::Model::new(database.clone());
    let integration = integration::model::Model::new(database.clone(), rss);
    let like = like::model::Model::new(database);

    Self {
      user,
      list,
      resource,
      integration,
      like,
    }
  }

  pub async fn sync_indexes(&self) -> Result<(), Error> {
    self.user.sync_indexes().await?;
    self.list.sync_indexes().await?;
    self.resource.sync_indexes().await?;
    self.like.sync_indexes().await?;
    self.integration.sync_indexes().await?;

    Ok(())
  }
}

#[async_trait]
pub trait Model<T: wither::Model + Send> {
  fn get_database(&self) -> &Database;

  async fn create(&self, mut model: T) -> Result<T, Error>
  where
    T: 'async_trait + wither::Model + Send,
  {
    let db = self.get_database();
    model
      .save(&db.conn, None)
      .await
      .map_err(Error::WitherError)?;

    Ok(model)
  }

  async fn find_by_id(&self, id: &ObjectId) -> Result<Option<T>, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::find_one(&db.conn, doc! { "_id": id }, None)
      .await
      .map_err(Error::WitherError)
  }

  async fn find_one(
    &self,
    query: Document,
    options: Option<FindOneOptions>,
  ) -> Result<Option<T>, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::find_one(&db.conn, query, options)
      .await
      .map_err(Error::WitherError)
  }

  async fn find(&self, query: Document, options: Option<FindOptions>) -> Result<Vec<T>, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::find(&db.conn, query, options)
      .await
      .map_err(Error::WitherError)?
      .try_collect::<Vec<T>>()
      .await
      .map_err(Error::WitherError)
  }

  async fn find_one_and_update(
    &self,
    query: Document,
    update: Document,
    options: Option<FindOneAndUpdateOptions>,
  ) -> Result<Option<T>, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::find_one_and_update(&db.conn, query, update, options)
      .await
      .map_err(Error::WitherError)
  }

  async fn update_one(
    &self,
    query: Document,
    update: Document,
    options: Option<UpdateOptions>,
  ) -> Result<UpdateResult, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::collection(&db.conn)
      .update_one(query, update, options)
      .await
      .map_err(Error::MongoError)
  }

  async fn update_many(
    &self,
    query: Document,
    update: Document,
    options: Option<UpdateOptions>,
  ) -> Result<UpdateResult, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::collection(&db.conn)
      .update_many(query, update, options)
      .await
      .map_err(Error::MongoError)
  }

  async fn delete_many(&self, query: Document) -> Result<DeleteResult, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::delete_many(&db.conn, query, None)
      .await
      .map_err(Error::WitherError)
  }

  async fn delete_one(&self, query: Document) -> Result<DeleteResult, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::collection(&db.conn)
      .delete_one(query, None)
      .await
      .map_err(Error::MongoError)
  }

  async fn count(&self, query: Document) -> Result<i64, Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::collection(&db.conn)
      .count_documents(query, None)
      .await
      .map_err(Error::MongoError)
  }

  async fn aggregate<A>(&self, pipeline: Vec<Document>) -> Result<Vec<A>, Error>
  where
    T: wither::Model + Send,
    A: Serialize + DeserializeOwned,
  {
    let db = self.get_database();
    let documents = T::collection(&db.conn)
      .aggregate(pipeline, None)
      .await
      .map_err(Error::MongoError)?
      .try_collect::<Vec<Document>>()
      .await
      .map_err(Error::MongoError)?;

    let documents = documents
      .into_iter()
      .map(|document| from_bson::<A>(Bson::Document(document)))
      .collect::<Result<Vec<A>, bson::de::Error>>()
      .map_err(Error::SerializeMongoResponse)?;

    Ok(documents)
  }

  async fn sync_indexes(&self) -> Result<(), Error>
  where
    T: wither::Model + Send,
  {
    let db = self.get_database();
    T::sync(&db.conn).await.map_err(Error::WitherError)?;

    Ok(())
  }
}
