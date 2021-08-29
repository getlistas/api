use actix::{Actor, Context, Handler, Message, ResponseActFuture};
use wither::bson::doc;
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::lib::util::parse_url;
use crate::models::Model as ModelTrait;
use crate::models::Models;
use crate::thirdparty::traer::Traer;

#[derive(Clone)]
pub struct ResourceActor {
  pub models: Models,
  pub traer: Traer,
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
pub struct EnreachResourceMessage {
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
    let traer = self.traer.clone();
    let task = enreach_resource(models, traer, msg.resource_id);
    let task = actix::fut::wrap_future::<_, Self>(task);

    Box::pin(task)
  }
}

async fn enreach_resource(
  models: Models,
  traer: Traer,
  resource_id: ObjectId,
) -> Result<(), Error> {
  let resource = models.resource.find_by_id(&resource_id).await?;
  let resource = match resource {
    Some(resource) => resource,
    None => {
      warn!("Resource was not found when updating resource metadatas");
      return Ok(());
    }
  };

  let url = parse_url(resource.url.as_str())?;
  let traer_response = traer.get_content_from_url(&url).await?;
  let metadata = traer_response.data;

  let mut update = doc! {};

  if let Some(html) = metadata.html {
    update.insert("html", html);
  }
  if let Some(text) = metadata.text {
    update.insert("text", text);
  }
  if let Some(length) = metadata.length {
    update.insert("length", length);
  }
  if let Some(publisher) = metadata.publisher {
    update.insert("publisher", publisher);
  }
  if let Some(author) = metadata.author {
    update.insert("author", author);
  }

  let has_update = !update.is_empty();
  if !has_update {
    return Ok(());
  }

  let update = doc! { "$set": update };

  models
    .resource
    .update_one(doc! { "_id": resource_id }, update, None)
    .await?;

  Ok(())
}
