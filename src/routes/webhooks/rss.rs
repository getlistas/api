use actix_web::{web, HttpResponse};
use futures::StreamExt;
use wither::bson::doc;
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::integrations::rss;
use crate::integrations::rss::Entry as RssEntry;
use crate::models::Model as ModelTrait;
use crate::models::Models;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;
type WebhookBody = web::Json<rss::Webhook>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("").route(web::post().to(webhook)));
}

async fn webhook(ctx: web::Data<Context>, body: WebhookBody) -> Response {
  debug!("Processing RSS webhook from rssapi");

  if body.new_entries.is_empty() {
    debug!("RSS webhook does not contain new entries, returning 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let subscription_id = body.subscription_id.clone();
  let integration = ctx
    .models
    .integration
    .find_one(doc! { "rss.subscription_id": &subscription_id }, None)
    .await?;

  let integration = match integration {
    Some(integration) => integration,
    None => {
      error!("Integration not found, unsubscribing and returning 404 status code");
      ctx.rss.unsuscribe(subscription_id.as_str()).await?;
      return Ok(HttpResponse::Ok().finish());
    }
  };

  let user_id = integration.user.clone();
  let list_id = integration.list.clone();

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": &list_id, "user": &user_id }, None)
    .await?;

  if list.is_none() {
    error!("List not found, removing integration, unsubscribing and returning 404 status code");
    ctx
      .models
      .integration
      .delete_one(doc! { "_id": integration.id.unwrap() })
      .await?;
    ctx.rss.unsuscribe(subscription_id.as_str()).await?;
    return Ok(HttpResponse::Ok().finish());
  }

  let next_resource_position = ctx
    .models
    .list
    .get_position_for_new_resource(&list_id)
    .await?;

  futures::stream::iter(body.new_entries.clone())
    .enumerate()
    .map(|(index, entry)| {
      create_resource_from_rss_entry(
        &ctx.models,
        entry,
        &user_id,
        &list_id,
        next_resource_position + (index as i32),
      )
    })
    .buffered(10)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<Vec<()>, Error>>()?;

  debug!("Returning 200 status code");
  let res = HttpResponse::Ok().finish();
  Ok(res)
}

pub async fn create_resource_from_rss_entry(
  models: &Models,
  entry: RssEntry,
  user_id: &ObjectId,
  list_id: &ObjectId,
  position: i32,
) -> Result<(), Error> {
  let mut resource = rss::Rss::create_resource_payload_from_entry(entry, &user_id, &list_id).await?;
  resource.position = position;
  models.resource.create(resource).await?;
  Ok(())
}
