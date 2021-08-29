use serde::{Deserialize, Serialize};
use url::Url;
use wither::bson::Document;

use crate::database;
use crate::errors::Error;
use crate::models;
use crate::models::resource::Resource;
use crate::models::Model as ModelTrait;
use crate::thirdparty::traer::Traer;

#[derive(Clone)]
pub struct Model {
  pub database: database::Database,
  pub traer: Traer,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ResourceMetadata {
  pub title: Option<String>,
  pub description: Option<String>,
  pub thumbnail: Option<String>,
}

impl models::Model<Resource> for Model {
  fn get_database(&self) -> &database::Database {
    &self.database
  }
}

impl Model {
  pub fn new(database: database::Database, traer: Traer) -> Self {
    Self { database, traer }
  }

  pub async fn get_position(&self, query: Document) -> Result<Option<i32>, Error> {
    let resource = self.find_one(query, None).await?;

    match resource {
      Some(resource) => Ok(Some(resource.position)),
      None => Ok(None),
    }
  }

  pub async fn get_metadata(&self, url: &Url) -> Result<Option<ResourceMetadata>, Error> {
    let traer_response = self.traer.get_some_content_from_url(&url).await?;
    if !traer_response.success {
      return Ok(None);
    }

    let metadata = ResourceMetadata {
      title: traer_response.data.title,
      description: traer_response.data.description,
      thumbnail: traer_response.data.image,
    };

    Ok(Some(metadata))
  }
}
