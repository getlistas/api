pub mod integration;
pub mod list;
pub mod resource;
pub mod user;
use futures::stream::TryStreamExt;
use wither::bson::Document;
use wither::mongodb::options::DeleteOptions;
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
    options: FindOneAndUpdateOptions,
  ) -> Result<Option<T>, Error>
  where
    T: wither::Model + Send,
  {
    T::find_one_and_update(&self.db.conn, query, update, options)
      .await
      .map_err(Error::WitherError)
  }

  pub async fn delete_many<T>(
    &self,
    query: Document,
    options: Option<DeleteOptions>,
  ) -> Result<DeleteResult, Error>
  where
    T: wither::Model + Send,
  {
    T::delete_many(&self.db.conn, query, options)
      .await
      .map_err(Error::WitherError)
  }

  pub async fn delete_one<T>(
    &self,
    query: Document,
    options: Option<DeleteOptions>
  ) -> Result<DeleteResult, Error>
  where
    T: wither::Model + Send,
  {
    T::collection(&self.db.conn).delete_one(query, options)
      .await
      .map_err(Error::MongoError)
  }
}
