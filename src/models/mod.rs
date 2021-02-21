pub mod integration;
pub mod list;
pub mod resource;
pub mod user;
use wither::bson::Document;

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
    T: wither::Model + std::marker::Send,
  {
    model
      .save(&self.db.conn, None)
      .await
      .map_err(Error::WitherError)?;

    Ok(model)
  }

  pub async fn find_one<T>(&self, query: Document) -> Result<Option<T>, Error>
  where
    T: wither::Model + std::marker::Send,
  {
    T::find_one(&self.db.conn, query, None)
      .await
      .map_err(Error::WitherError)
  }
}
