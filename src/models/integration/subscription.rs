use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JSON;
use wither::bson::doc;
use wither::bson::oid::ObjectId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListasSubscription {
  pub list: ObjectId,
}

impl ListasSubscription {
  pub fn to_response_schema(&self) -> JSON {
    json!({ "list": self.list.to_hex() })
  }
}
