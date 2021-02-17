use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId, Document};
use wither::Model as DatabaseModel;

use crate::{database::Database, errors::Error};

#[derive(Debug, Serialize, Deserialize)]
pub struct RSS {
  pub url: String,
  pub subscription_id: String,
  pub status: String,
  pub feed_type: String,
  pub metadata: Option<String>,
}

#[derive(Debug, DatabaseModel, Serialize, Deserialize)]
pub struct Model {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub user: ObjectId,
  pub list: ObjectId,

  pub rss: Option<RSS>,

  pub created_at: DateTime,
  pub updated_at: DateTime,
}

#[derive(Clone)]
pub struct Integration {
  database: Database,
}

impl Integration {
  pub fn new(database: Database) -> Self {
    Self { database }
  }

  pub async fn create(&self, mut model: Model) -> Result<Model, Error> {
    model
      .save(&self.database.conn, None)
      .await
      .map_err(Error::WitherError)?;

    Ok(model)
  }

  pub async fn find_one(&self, query: Document) -> Result<Option<Model>, Error> {
    Model::find_one(&self.database.conn, query, None)
      .await
      .map_err(Error::WitherError)
  }
}
