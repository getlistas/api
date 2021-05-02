use serde::{Deserialize, Serialize};
use wither::bson::doc;
use wither::bson::oid::ObjectId;

use crate::lib::serde::serialize_object_id_as_hex_string;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListasSubscription {
  pub list: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateListasSubscription {
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub list: ObjectId,
}

impl From<ListasSubscription> for PrivateListasSubscription {
  fn from(listas_subscription: ListasSubscription) -> Self {
    Self {
      list: listas_subscription.list.clone(),
    }
  }
}
