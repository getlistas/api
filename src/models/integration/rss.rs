use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rss {
  pub url: String,
  pub subscription_id: String,
  // pub feed_type: String,
  pub metadata: Option<Json>,
}
