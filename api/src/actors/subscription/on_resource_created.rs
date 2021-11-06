use actix::Message;
use actix::ResponseActFuture;
use futures::stream::StreamExt;
use wither::bson::doc;
use wither::bson::oid::ObjectId;

use crate::actors::subscription::Actor as SubscriptionActor;
use crate::errors::Error;
use crate::lib::date;
use crate::models::resource::Resource;
use crate::models::Model as ModelTrait;
use crate::models::Models;

impl actix::Handler<ResourceCreated> for SubscriptionActor {
  type Result = ResponseActFuture<Self, Result<(), Error>>;

  fn handle(&mut self, msg: ResourceCreated, _ctx: &mut actix::Context<Self>) -> Self::Result {
    debug!(
      "Handling subscription actor resource created event with payload {:?}",
      &msg
    );
    let models = self.models.clone();
    let task = on_resource_created(models, msg.resource_id);
    let task = actix::fut::wrap_future::<_, Self>(task);

    Box::pin(task)
  }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct ResourceCreated {
  pub resource_id: ObjectId,
}

async fn on_resource_created(models: Models, resource_id: ObjectId) -> Result<(), Error> {
  let resource = models
    .resource
    .find_one(doc! { "_id": &resource_id }, None)
    .await?;

  let resource = match resource {
    Some(resource) => resource,
    None => {
      error!("Resource not found, when handling subscription event");
      return Ok(());
    }
  };

  let list_id = resource.list.clone();

  let integrations = models
    .integration
    .find(
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
      let position = models
        .list
        .get_next_resource_position(&integration.list)
        .await?;

      models
        .resource
        .create(Resource {
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
          html: None,
          text: None,
          author: None,
          length: None,
          publisher: None,
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
