use wither::bson::doc;
use wither::bson::oid::ObjectId;

use crate::database;
use crate::errors::Error;
use crate::models;
use crate::models::integration::Integration;
use crate::models::Model as ModelTrait;
use crate::{models::integration::Kind, thirdparty::rss::Rss};

#[derive(Clone)]
pub struct Model {
  database: database::Database,
  rss: Rss,
}

impl models::Model<Integration> for Model {
  fn get_database(&self) -> &database::Database {
    &self.database
  }
}

impl Model {
  pub fn new(database: database::Database, rss: Rss) -> Self {
    Self { database, rss }
  }

  pub async fn remove(&self, integration_id: &ObjectId) -> Result<(), Error> {
    let integration = self.find_by_id(integration_id).await?;
    let integration = match integration {
      Some(integration) => integration,
      None => {
        error!(
          "Failed to remove Integration, Integration with ID {} not found",
          integration_id
        );
        return Ok(());
      }
    };

    if let Kind::Rss = integration.kind {
      self
        .rss
        .unsuscribe(integration.rss.as_ref().unwrap().subscription_id.as_str())
        .await?;
    };

    self
      .delete_one(doc! { "_id": integration.id.clone().unwrap() })
      .await?;

    Ok(())
  }
}
