use serde::{Deserialize, Serialize};
use serde_json::Value as JSON;

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
