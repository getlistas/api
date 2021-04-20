use actix::ResponseActFuture;
use futures::stream::StreamExt;
use wither::bson::doc;
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::lib::date;
use crate::models::integration::Integration;
use crate::models::list::List;
use crate::models::resource::Resource;
use crate::models::Models;

#[derive(Clone)]
pub struct Actor {
  pub models: Models,
}

impl actix::Actor for Actor {
  type Context = actix::Context<Self>;

  fn started(&mut self, _ctx: &mut actix::Context<Self>) {
    info!("Subscription actor started");
  }

  fn stopped(&mut self, _ctx: &mut actix::Context<Self>) {
    info!("Subscription actor stopped");
  }
}

impl actix::Handler<Message> for Actor {
  type Result = ResponseActFuture<Self, Result<(), Error>>;

  fn handle(&mut self, msg: Message, _ctx: &mut actix::Context<Self>) -> Self::Result {
    debug!("Handling subscription actor event with payload {:?}", &msg);
    let models = self.models.clone();
    let task = send(models.clone(), msg.resource_id.clone());
    let task = actix::fut::wrap_future::<_, Self>(task);
    Box::pin(task)
  }
}

#[derive(Debug, actix::Message)]
#[rtype(result = "Result<(), Error>")]
pub struct Message {
  pub resource_id: ObjectId,
}

async fn send(models: Models, resource_id: ObjectId) -> Result<(), Error> {
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

  let list_id = resource.list.clone();

  let integrations = models
    .find::<Integration>(
      doc! { "kind": "listas-subscription", "listas_subscription.list": &list_id },
      None,
    )
    .await?;

  debug!(
    "Creating {} resources from subscription integration",
    integrations.len()
  );
  let now = date::now();
  let resource_futures = integrations.iter().map(|integration| {
    let resource = resource.clone();
    let models = models.clone();
    async move {
      let position = List::get_next_resource_position(&models, &integration.list).await?;
      models
        .create::<Resource>(Resource {
          id: None,
          list: integration.list.clone(),
          user: integration.user.clone(),
          title: resource.title.clone(),
          url: resource.url.clone(),
          description: resource.description.clone(),
          thumbnail: resource.thumbnail.clone(),
          tags: resource.tags.clone(),
          created_at: now,
          updated_at: now,
          completed_at: None,
          position,
        })
        .await?;

      Ok::<(), Error>(())
    }
  });

  futures::stream::iter(resource_futures)
    .buffer_unordered(50)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<(), Error>>()?;

  Ok(())
}
