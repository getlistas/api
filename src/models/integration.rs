use actix_web::web;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JSON;
use strum::EnumString;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::errors::Error;
use crate::lib::date;
use crate::Context;

type Ctx = web::Data<Context>;

#[derive(Debug, Clone, Serialize, Deserialize, EnumString)]
pub enum Kind {
  #[serde(rename = "rss")]
  #[strum(serialize = "rss")]
  RSS,
  #[serde(rename = "follow")]
  #[strum(serialize = "follow")]
  Follow,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Follow {
  pub list: ObjectId,
}

impl Follow {
  pub fn to_response_schema(&self) -> JSON {
    json!({ "list": self.list.to_hex() })
  }
}

#[derive(Debug, Clone, Model, Serialize, Deserialize)]
pub struct Integration {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<ObjectId>,
  pub user: ObjectId,
  pub list: ObjectId,
  pub kind: Kind,
  // TODO: Remove once the front end is not using this field anymore.
  pub service: Kind,
  pub created_at: DateTime,
  pub updated_at: DateTime,

  pub rss: Option<RSS>,
  pub follow: Option<Follow>,
}

impl Integration {
  pub async fn remove(&self, ctx: &Ctx) -> Result<(), Error> {
    match self.kind {
      Kind::RSS => {
        ctx
          .rss
          .unsuscribe(self.rss.as_ref().unwrap().subscription_id.as_str())
          .await?;
      }
      _ => {}
    };

    ctx
      .models
      .delete_one::<Integration>(doc! { "_id": self.id.clone().unwrap() })
      .await?;

    Ok(())
  }

  pub fn to_response_schema(&self) -> JSON {
    let this = self.clone();
    json!({
        "id": this.id.clone().unwrap().to_hex(),
        "user": this.user.to_hex(),
        "list": this.list.to_hex(),
        "kind": this.kind,
        // TODO: Remove once the front end is not using this field anymore.
        "service": this.kind,
        "rss": this.rss.map(|rss| rss.to_response_schema()),
        "follow": this.follow.map(|follow| follow.to_response_schema()),
        "created_at": date::to_rfc3339(this.created_at),
        "updated_at": date::to_rfc3339(this.updated_at),
    })
  }
}
