use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::stream::StreamExt;
use serde::Deserialize;
use wither::bson::doc;
use wither::Model;

use crate::auth;
use crate::errors::Error;
use crate::lib::date;
use crate::lib::id::ID;
use crate::lib::util::parse_url;
use crate::lib::util::to_object_id;
use crate::models::integration;
use crate::models::integration::Integration;
use crate::models::integration::RSS;
use crate::models::list::List;
use crate::models::{resource::Resource, user::UserID};
use crate::Context;

#[derive(Deserialize)]
struct RSSCreate {
  list: String,
  url: String,
}

type Ctx = web::Data<Context>;
type Response = actix_web::Result<HttpResponse>;
type RSSCreateBody = web::Json<RSSCreate>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(
    web::resource("/integrations/rss")
      .route(web::post().to(create_rss_integration))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/integrations/rss/{id}")
      .route(web::delete().to(remove_rss_integration))
      .wrap(auth.clone()),
  );
}

async fn create_rss_integration(ctx: Ctx, body: RSSCreateBody, user_id: UserID) -> Response {
  let list_id = to_object_id(body.list.clone())?;
  let user_id = user_id.0;
  let url = parse_url(body.url.as_str())?;

  let list = ctx
    .models
    .find_one::<List>(doc! { "_id": &list_id, "user": &user_id })
    .await?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  if !ctx.rss.is_valid_url(&url).await? {
    debug!("Requested URL does not contains a valid RSS feed");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let now = date::now();
  let subscription = ctx.rss.subscribe(&url).await?;
  let integration = ctx
    .models
    .create(integration::Integration {
      id: None,
      user: user_id.clone(),
      list: list_id.clone(),
      created_at: now,
      updated_at: now,
      rss: Some(RSS {
        url: subscription.url,
        subscription_id: subscription.subscription_id,
        status: subscription.status,
        feed_type: subscription.feed_type,
        metadata: subscription.info,
      }),
    })
    .await?;

  let mut resources = ctx
    .rss
    .build_resources_from_feed(&url, &user_id, &list_id)
    .await?;

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

  debug!("Creating resources from RSS feed");
  futures::stream::iter(resources)
    .buffer_unordered(10)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<(), Error>>()?;

  debug!("Returning integration and 200 status code");
  let res = HttpResponse::Ok().json(integration);
  Ok(res)
}

async fn remove_rss_integration(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let user_id = user_id.0;
  let integration_id = id.0;

  let integration = ctx
    .models
    .find_one::<Integration>(doc! { "_id": &integration_id, "user": &user_id })
    .await?;

  let integration = match integration {
    Some(integration) => integration,
    None => {
      debug!("Integration not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let rss = match integration.rss {
    Some(rss) => rss,
    None => {
      debug!("Integration is not an RSS integration, returning 400 status code");
      return Ok(HttpResponse::BadRequest().finish());
    }
  };

  ctx.rss.unsuscribe(rss.subscription_id.as_str()).await?;

  debug!("Removing integration");
  ctx
    .models
    .delete_one::<Integration>(doc! { "_id": &integration_id })
    .await?;

  debug!("Integration removed, returning 204 status code");
  let res = HttpResponse::NoContent().finish();

  Ok(res)
}
