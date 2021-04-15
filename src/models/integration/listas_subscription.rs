use actix::{Actor, Context as ActorContext, Handler, Message};
use actix::{ActorFuture, AsyncContext, ResponseActFuture};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JSON;
use wither::bson::doc;
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::models::resource::Resource;
use crate::models::Models;

// pub type ResponseActFuture<A, I> = Pin<Box<dyn ActorFuture<Output = I, Actor = A>>>;

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

impl Handler<SubscriptionPayload> for ListasSubscriptionActor {
  type Result = ResponseActFuture<Self, Result<(), Error>>;

  fn handle(&mut self, msg: SubscriptionPayload, ctx: &mut ActorContext<Self>) -> Self::Result {
    let resource_id = msg.resource_id;
    let models = self.models.clone();
    let task = async move {
      let resource = models
        .find_one::<Resource>(doc! { "_id": &resource_id }, None)
        .await?;

      Ok::<(), Error>(())
    };
    let task = actix::fut::wrap_future::<_, Self>(task);
    Box::pin(task)
  }
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct SubscriptionPayload {
  pub resource_id: ObjectId,
}

async fn send(models: &Models, resource_id: ObjectId) -> Result<(), Error> {
  let resource = models
    .find_one::<Resource>(doc! { "_id": &resource_id }, None)
    .await
    .unwrap();

  let resource = match resource {
    Some(resource) => resource,
    None => {
      error!("Resource not found, when handling subscription event");
      return Ok(());
    }
  };

  Ok(())
}
