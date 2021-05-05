use actix::Message;
use actix::ResponseActFuture;
use futures::stream::StreamExt;
use wither::bson::doc;
use wither::bson::oid::ObjectId;

use crate::actors::subscription::Actor as SubscriptionActor;
use crate::emails;
use crate::errors::Error;
use crate::mailer::Mailer;
use crate::models::Model as ModelTrait;
use crate::models::Models;
use crate::settings::Settings;

impl actix::Handler<ListRemoved> for SubscriptionActor {
  type Result = ResponseActFuture<Self, Result<(), Error>>;

  fn handle(&mut self, msg: ListRemoved, _ctx: &mut actix::Context<Self>) -> Self::Result {
    debug!(
      "Handling subscription actor list removed event with payload {:?}",
      &msg
    );
    let models = self.models.clone();
    let settings = self.settings.clone();
    let mailer = self.mailer.clone();
    let task = on_list_removed(settings, models, mailer, msg);
    let task = actix::fut::wrap_future::<_, Self>(task);

    Box::pin(task)
  }
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub struct ListRemoved {
  pub id: ObjectId,
  pub title: String,
}

async fn on_list_removed(
  settings: Settings,
  models: Models,
  mailer: Mailer,
  removed_list: ListRemoved,
) -> Result<(), Error> {
  let integrations = models
    .integration
    .find(
      doc! { "kind": "listas-subscription", "listas_subscription.list": &removed_list.id },
      None,
    )
    .await?;

  let send_email_from = settings.mailer.from.as_str();

  debug!("Removing {} integrations", integrations.len());

  let list_futures = integrations.iter().map(|integration| {
    let models = models.clone();
    let mailer = mailer.clone();
    let removed_list = removed_list.clone();
    async move {
      let list = models.list.find_by_id(&integration.list).await?;
      let list = match list {
        Some(list) => list,
        None => {
          debug!("List not found when removing subscription integration");
          return Ok(());
        }
      };

      let user = models.user.find_by_id(&integration.user).await?;
      let user = match user {
        Some(user) => user,
        None => {
          error!("User not found when removing subscription integration");
          return Ok(());
        }
      };

      let integration_id = integration
        .id
        .as_ref()
        .expect("Failed to unwrap Integration ID");

      models.integration.remove(integration_id).await?;

      let subscription_removed_email =
        emails::create_subscription_removed_email(send_email_from, &user, &list, &removed_list)?;

      mailer.send(subscription_removed_email).await?;

      Ok::<(), Error>(())
    }
  });

  futures::stream::iter(list_futures)
    .buffer_unordered(50)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<(), Error>>()?;

  Ok(())
}
