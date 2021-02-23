use actix_web::{web, HttpResponse};
use futures::StreamExt;
use wither::bson::doc;
use wither::Model;

use crate::errors::Error;
use crate::integrations::rss;
use crate::models::integration::Integration;
use crate::models::list::List;
use crate::models::resource::Resource;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;
type WebhookBody = web::Json<rss::Webhook>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("/rssapi").route(web::post().to(webhook)));
}

async fn webhook(ctx: web::Data<Context>, body: WebhookBody) -> Response {
  debug!("Processing RSS webhook from rssapi");

  if !body.new_entries.len() > 0 {
    debug!("RSS webhook does not contain new entries, returning 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let subscription_id = body.subscription_id.clone();
  let integration = ctx
    .models
    .find_one::<Integration>(doc! { "rss.subscription_id": &subscription_id })
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
    .find_one::<List>(doc! { "_id": &list_id, "user": &user_id })
    .await?;

  let list = match list {
    Some(list) => list,
    None => {
      error!("List not found, removing integration, unsubscribing and returning 404 status code");
      ctx
        .models
        .delete_one::<Integration>(doc! { "_id": integration.id.unwrap() })
        .await?;
      ctx.rss.unsuscribe(subscription_id.as_str()).await?;
      return Ok(HttpResponse::Ok().finish());
    }
  };

  let mut entries = body.new_entries.clone();
  let resources = entries
    .iter_mut()
    .map(|entry| rss::RSS::create_resource_from_entry(entry, &user_id, &list_id));

  let mut resources = futures::stream::iter(resources)
    .buffered(50)
    .collect::<Vec<Result<Resource, Error>>>()
    .await
    .into_iter()
    .collect::<Result<Vec<Resource>, Error>>()?;

  let last_resource = Resource::find_last(&ctx.database.conn, &user_id, &list_id).await?;
  let position = last_resource
    .map(|resource| resource.position + 1)
    .unwrap_or(0);

  let resources = resources
    .iter_mut()
    .enumerate()
    .map(move |(index, resource)| {
      let conn = ctx.database.conn.clone();
      resource.position = position + (index as i32);

      async move { resource.save(&conn, None).await.map_err(Error::WitherError) }
    });

  debug!("Creating resources from RSS webhook");
  futures::stream::iter(resources)
    .buffer_unordered(50)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<(), Error>>()?;

  debug!("Returning 200 status code");
  let res = HttpResponse::Ok().finish();
  Ok(res)
}
