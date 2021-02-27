use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JSON;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model as DatabaseModel;

use crate::lib::date;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSS {
  pub url: String,
  pub subscription_id: String,
  pub status: String,
  pub feed_type: String,
  pub metadata: Option<String>,
}

impl RSS {
  pub fn to_response_schema(&self) -> JSON {
    serde_json::to_value(self).unwrap()
  }
}

#[derive(Debug, Clone, DatabaseModel, Serialize, Deserialize)]
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
  pub fn to_response_schema(&self) -> JSON {
    let this = self.clone();
    json!({
        "id": this.id.clone().unwrap().to_hex(),
        "user": this.user.to_hex(),
        "list": this.list.to_hex(),
        "rss": this.rss.map(|rss| rss.to_response_schema()),
        "created_at": date::to_rfc3339(this.created_at),
        "updated_at": date::to_rfc3339(this.updated_at),
    })
  }
}
