use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

#[derive(Debug, Serialize, Deserialize)]
pub struct RSS {
  pub url: String,
  pub subscription_id: String,
  pub status: String,
  pub feed_type: String,
  pub metadata: Option<String>,
}

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct Integration {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub user: ObjectId,
  pub list: ObjectId,

  pub rss: Option<RSS>,

  pub created_at: DateTime,
  pub updated_at: DateTime,
}

impl Integration {
  pub fn is_rss(&self) -> bool {
    self.rss.is_some()
  }
}
