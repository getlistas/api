use actix_web::web::block as to_future;
use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::lib::util::create_random_string;
use crate::{errors, lib::date};


pub struct RSS {
  pub url: String,
  pub external_subscription_id: String
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