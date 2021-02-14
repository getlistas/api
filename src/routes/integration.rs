use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::stream::StreamExt;
use itertools::Update;
use serde::Deserialize;
use wither::bson::doc;
use wither::Model;
use wither::WitherError;

use crate::auth;
use crate::errors::ApiError;
use crate::lib::util::parse_url;
use crate::lib::util::to_object_id;
use crate::lib::{date, resource_metadata};
use crate::models::integration::{Integration, RSS};
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
}

async fn create_rss_integration(ctx: Ctx, body: RSSCreateBody, user_id: UserID) -> Response {
  let list_id = to_object_id(body.list.clone())?;
  let user_id = user_id.0;
  let url = parse_url(body.url.as_str())?;

  let list = List::find_one(
    &ctx.database.conn,
    doc! {
        "_id": &list_id,
        "user": &user_id,
    },
    None,
  )
  .await
  .map_err(ApiError::WitherError)?;

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
  let mut integration = Integration {
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
  };

  integration
    .save(&ctx.database.conn, None)
    .await
    .map_err(ApiError::WitherError)?;

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

      async move {
        resource
          .save(&conn, None)
          .await
          .map_err(ApiError::WitherError)
      }
    });

  debug!("Creating resources from RSS feed");
  futures::stream::iter(resources)
    .buffer_unordered(10)
    .collect::<Vec<Result<(), ApiError>>>()
    .await
    .into_iter()
    .collect::<Result<(), ApiError>>()?;

  debug!("Returning 200 status code");
  let res = HttpResponse::Ok().finish();
  Ok(res)
}
