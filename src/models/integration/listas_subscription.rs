use actix::{Actor, Context as ActorContext};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JSON;
use wither::{bson::oid::ObjectId, ModelCursor};

use crate::errors::Error;
use crate::models::resource::Resource;
use crate::models::Models;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListasSubscription {
  pub list: ObjectId,
}

impl ListasSubscription {
  pub fn to_response_schema(&self) -> JSON {
    json!({ "list": self.list.to_hex() })
  }
}

pub fn on_resource_creation(models: Models, resource: Resource) -> Result<(), Error> {
  Ok(())
}

pub struct ListasSubscriptionActor {
  models: Models,
}

impl ListasSubscriptionActor {
  pub fn new(models: Models) -> Self {
    Self { models }
  }
}

impl Actor for ListasSubscriptionActor {
  type Context = ActorContext<Self>;

  fn started(&mut self, ctx: &mut ActorContext<Self>) {
    println!("Actor is alive");
  }

  fn stopped(&mut self, ctx: &mut ActorContext<Self>) {
    println!("Actor is stopped");
  }
}
