use wither::bson::Document;

use crate::database;
use crate::errors::Error;
use crate::models;
use crate::models::resource::Resource;
use crate::models::Model as ModelTrait;

#[derive(Clone)]
pub struct Model {
  pub database: database::Database,
}

impl models::Model<Resource> for Model {
  fn get_database(&self) -> &database::Database {
    &self.database
  }
}

impl Model {
  pub fn new(database: database::Database) -> Self {
    Self { database }
  }

  pub async fn get_position(&self, query: Document) -> Result<Option<i32>, Error> {
    let resource = self.find_one(query, None).await?;

    match resource {
      Some(resource) => Ok(Some(resource.position)),
      None => Ok(None),
    }
  }
}
