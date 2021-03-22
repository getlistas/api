pub mod integration;
pub mod list;
pub mod resource;
pub mod user;

use futures::stream::TryStreamExt;
use serde::{de::DeserializeOwned, ser::Serialize};
use wither::bson;
use wither::bson::from_bson;
use wither::bson::Bson;
use wither::bson::Document;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::mongodb::options::FindOptions;
use wither::mongodb::results::DeleteResult;

use crate::database::Database;
use crate::errors::Error;

#[derive(Clone)]
pub struct Models {
  db: Database,
}

impl Models {
  pub fn new(db: Database) -> Self {
    Self { db }
  }

  pub async fn create<T>(&self, mut model: T) -> Result<T, Error>
  where
    T: wither::Model + Send,
  {
    model
      .save(&self.db.conn, None)
      .await
      .map_err(Error::WitherError)?;

    Ok(model)
  }

  pub async fn find_one<T>(&self, query: Document) -> Result<Option<T>, Error>
  where
    T: wither::Model + Send,
  {
    T::find_one(&self.db.conn, query, None)
      .await
      .map_err(Error::WitherError)
  }

  pub async fn find<T>(
    &self,
    query: Document,
    options: Option<FindOptions>,
  ) -> Result<Vec<T>, Error>
  where
    T: wither::Model + Send,
  {
    T::find(&self.db.conn, query, options)
      .await
      .map_err(Error::WitherError)?
      .try_collect::<Vec<T>>()
      .await
      .map_err(Error::WitherError)
  }

  pub async fn find_one_and_update<T>(
    &self,
    query: Document,
    update: Document,
    options: Option<FindOneAndUpdateOptions>,
  ) -> Result<Option<T>, Error>
  where
    T: wither::Model + Send,
  {
    T::find_one_and_update(&self.db.conn, query, update, options)
      .await
      .map_err(Error::WitherError)
  }

  pub async fn delete_many<T>(&self, query: Document) -> Result<DeleteResult, Error>
  where
    T: wither::Model + Send,
  {
    T::delete_many(&self.db.conn, query, None)
      .await
      .map_err(Error::WitherError)
  }

  pub async fn delete_one<T>(&self, query: Document) -> Result<DeleteResult, Error>
  where
    T: wither::Model + Send,
  {
    T::collection(&self.db.conn)
      .delete_one(query, None)
      .await
      .map_err(Error::MongoError)
  }

  pub async fn count<T>(&self, query: Document) -> Result<i64, Error>
  where
    T: wither::Model + Send,
  {
    T::collection(&self.db.conn)
      .count_documents(query, None)
      .await
      .map_err(Error::MongoError)
  }

  pub async fn aggregate<T, R>(&self, pipeline: Vec<Document>) -> Result<Vec<R>, Error>
  where
    T: wither::Model + Send,
    R: Serialize + DeserializeOwned,
  {
    let documents = T::collection(&self.db.conn)
      .aggregate(pipeline, None)
      .await
      .map_err(Error::MongoError)?
      .try_collect::<Vec<Document>>()
      .await
      .map_err(Error::MongoError)?;

    let documents = documents
      .into_iter()
      .map(|document| from_bson::<R>(Bson::Document(document)))
      .collect::<Result<Vec<R>, bson::de::Error>>()
      .map_err(Error::SerializeMongoResponse)?;

    Ok(documents)
  }
}
