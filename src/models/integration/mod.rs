pub mod model;
pub mod rss;
pub mod subscription;

use serde::{Deserialize, Serialize};
use strum::EnumString;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;
use crate::models::integration::rss::RSS;
use crate::models::integration::subscription::ListasSubscription;
use crate::models::integration::subscription::PrivateListasSubscription;

#[derive(Debug, Clone, Serialize, Deserialize, EnumString)]
pub enum Kind {
  #[serde(rename = "rss")]
  #[strum(serialize = "rss")]
  RSS,
  #[serde(rename = "listas-subscription")]
  #[strum(serialize = "listas-subscription")]
  ListasSubscription,
}

#[derive(Debug, Clone, Model, Serialize, Deserialize)]
pub struct Integration {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub user: ObjectId,
  pub list: ObjectId,
  pub kind: Kind,
  pub created_at: DateTime,
  pub updated_at: DateTime,

  pub rss: Option<RSS>,
  pub listas_subscription: Option<ListasSubscription>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateIntegration {
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub id: ObjectId,
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub user: ObjectId,
  #[serde(alias = "_id", serialize_with = "serialize_object_id_as_hex_string")]
  pub list: ObjectId,
  pub kind: Kind,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub created_at: DateTime,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  pub updated_at: DateTime,
  pub rss: Option<RSS>,
  pub listas_subscription: Option<PrivateListasSubscription>,
}

impl From<Integration> for PrivateIntegration {
  fn from(integration: Integration) -> Self {
    Self {
      id: integration.id.clone().unwrap(),
      user: integration.user.clone(),
      list: integration.list.clone(),
      created_at: integration.created_at.clone(),
      updated_at: integration.updated_at.clone(),
      kind: integration.kind.clone(),
      rss: integration.rss.clone(),
      listas_subscription: integration.listas_subscription.clone().map(Into::into),
    }
  }
}