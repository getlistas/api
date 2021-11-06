use serde::{Deserialize, Serialize};
use url::Url;
use validator::Validate;
use wither::bson::doc;
use wither::bson::oid::ObjectId;
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

  // TODO improve fn name, maybe create and model one could be called insert.
  pub async fn build(&self, resource: Resource) -> Result<Resource, Error> {
    resource.validate().map_err(Error::ValidateModel)?;
    self.create(resource).await
  }

  pub async fn get_position(&self, query: Document) -> Result<Option<i32>, Error> {
    let resource = self.find_one(query, None).await?;

    match resource {
      Some(resource) => Ok(Some(resource.position)),
      None => Ok(None),
    }
  }

  pub async fn get_metadata(&self, url: &Url) -> Result<Option<ResourceMetadata>, Error> {
    let traer_response = self.traer.get_slim_content_from_url(url).await?;
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

  pub async fn populate(&self, resource_id: ObjectId) -> Result<(), Error> {
    let resource = self.find_by_id(&resource_id).await?;
    let resource = match resource {
      Some(resource) => resource,
      None => {
        error!(
          "Resource with ID {} not found when updating resource metadata",
          resource_id
        );
        return Ok(());
      }
    };

    let url = resource.get_url();
    let metadata = self.traer.get_content_from_url(&url).await?;
    let metadata = match metadata {
      None => return Ok(()),
      Some(metadata) => metadata,
    };

    let mut update = doc! {};

    if let Some(title) = metadata.title {
      update.insert("title", title);
    }
    if let Some(description) = metadata.description {
      update.insert("description", description);
    }
    if let Some(image) = metadata.image {
      update.insert("thumbnail", image);
    }
    if let Some(html) = metadata.html {
      update.insert("html", html);
    }
    if let Some(text) = metadata.text {
      update.insert("text", text);
    }
    if let Some(length) = metadata.length {
      update.insert("length", length);
    }
    if let Some(publisher) = metadata.publisher {
      update.insert("publisher", publisher);
    }
    if let Some(author) = metadata.author {
      update.insert("author", author);
    }

    // Metadata was available for the specified resource but for some reason
    // the Treaer API returned no attributes.
    let has_update = !update.is_empty();
    if !has_update {
      return Ok(());
    }

    self
      .update_one(doc! { "_id": resource_id }, doc! { "$set": update }, None)
      .await?;

    Ok(())
  }
}
