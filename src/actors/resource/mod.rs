use actix::{Actor, Context, Handler, Message, ResponseActFuture};
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::models::Models;

#[derive(Clone)]
pub struct ResourceActor {
  pub models: Models,
}

impl Actor for ResourceActor {
  type Context = Context<Self>;

  fn started(&mut self, _ctx: &mut Context<Self>) {
    info!("Resource actor started");
  }

  fn stopped(&mut self, _ctx: &mut Context<Self>) {
    info!("Resource actor stopped");
  }
}

#[derive(Debug, Clone, Message)]
#[rtype(result = "Result<(), Error>")]
struct EnreachResourceMessage {
  pub resource_id: ObjectId,
}

impl Handler<EnreachResourceMessage> for ResourceActor {
  type Result = ResponseActFuture<Self, Result<(), Error>>;

  fn handle(
    &mut self,
    msg: EnreachResourceMessage,
    _ctx: &mut actix::Context<Self>,
  ) -> Self::Result {
    debug!(
      "Handling enreach resource message event with payload {:?}",
      &msg
    );

    let models = self.models.clone();
    let task = enreach_resource(msg.resource_id.clone());
    let task = actix::fut::wrap_future::<_, Self>(task);

    Box::pin(task)
  }
}

async fn enreach_resource(resource_id: ObjectId) -> Result<(), Error> {
  Ok(())
}
