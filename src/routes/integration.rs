use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::stream::StreamExt;
use serde::Deserialize;
use std::str::FromStr;
use wither::bson::doc;

use crate::auth;
use crate::auth::UserID;
use crate::errors::Error;
use crate::lib::date;
use crate::lib::id::ID;
use crate::lib::util::parse_url;
use crate::lib::util::to_object_id;
use crate::models::integration;
use crate::models::integration::rss::Rss;
use crate::models::integration::PrivateIntegration;
use crate::models::resource::Resource;
use crate::models::Model as ModelTrait;
use crate::Context;

#[derive(Deserialize)]
struct RSSPayload {
  list: String,
  url: String,
}

#[derive(Deserialize)]
struct SubscriptionPayload {
  subscribe_from: String,
  subscribe_to: String,
}
#[derive(Deserialize)]
struct Query {
  list: Option<String>,
  kind: Option<String>,
}

type Ctx = web::Data<Context>;
type Response = actix_web::Result<HttpResponse>;
type RSSCreateBody = web::Json<RSSPayload>;
type SubscriptionCreateBody = web::Json<SubscriptionPayload>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(
    web::resource("/integrations")
      .route(web::get().to(query_integrations))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/integrations/rss")
      .route(web::post().to(create_rss_integration))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/integrations/listas-subscription")
      .route(web::post().to(create_subscription_integration))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/integrations/{id}")
      .route(web::delete().to(remove_integration))
      .wrap(auth),
  );
}

async fn query_integrations(ctx: Ctx, user: UserID, qs: web::Query<Query>) -> Response {
  let user_id = user.0;
  let mut query = doc! { "user": &user_id };

  if qs.list.is_some() {
    let list_id = to_object_id(qs.list.clone().unwrap())?;
    query.insert("list", list_id);
  }

  if qs.kind.is_some() {
    let kind = qs.kind.as_ref().unwrap();
    query.insert("kind", kind);
  }

  let integrations = ctx.models.integration.find(query, None).await?;
  let integrations = integrations
    .into_iter()
    .map(Into::into)
    .collect::<Vec<PrivateIntegration>>();

  debug!("Returning integrations");
  let res = HttpResponse::Ok().json(integrations);
  Ok(res)
}

async fn create_rss_integration(ctx: Ctx, body: RSSCreateBody, user_id: UserID) -> Response {
  let list_id = to_object_id(body.list.clone())?;
  let user_id = user_id.0;
  let url = parse_url(body.url.as_str())?;

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": &list_id, "user": &user_id }, None)
    .await?;

  if list.is_none() {
    debug!("List not found, returning 404 status code");
    return Ok(HttpResponse::NotFound().finish());
  }

  if !ctx.rss.is_valid_url(&url).await? {
    debug!("Requested URL does not contains a valid RSS feed");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let now = date::now();
  let subscription = ctx.rss.subscribe(&url).await?;
  let integration = ctx
    .models
    .integration
    .create(integration::Integration {
      id: None,
      user: user_id.clone(),
      list: list_id.clone(),
      created_at: now,
      updated_at: now,
      kind: integration::Kind::from_str("rss").unwrap(),
      listas_subscription: None,
      rss: Some(Rss {
        url: subscription.url,
        subscription_id: subscription.subscription_id,
        status: subscription.status,
        feed_type: subscription.feed_type,
        metadata: subscription.info,
      }),
    })
    .await?;

  // TODO: Move this to an actor / background job
  let resources = ctx
    .rss
    .create_resources_payload_from_feed(&url, &user_id, &list_id)
    .await?;

  let last_resource = ctx
    .models
    .list
    .get_last_completed_resource(&list_id)
    .await?;

  let position = last_resource
    .map(|resource| resource.position + 1)
    .unwrap_or(0);

  let resources = resources
    .into_iter()
    .enumerate()
    .map(|(index, mut resource)| {
      resource.position = position + (index as i32);
      async {
        let resource = enrich_resource(&ctx, resource).await?;
        ctx.models.resource.create(resource).await?;
        Ok(())
      }
    });

  debug!("Creating resources from RSS feed");
  futures::stream::iter(resources)
    .buffer_unordered(10)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<(), Error>>()?;

  ctx
    .models
    .list
    .update_last_activity_at(&list_id)
    .await
    .map_err(|err| {
      error!(
        "Failed to update last activity for list {}. Error {}",
        &list_id, err
      )
    })?;

  debug!("Returning integration and 200 status code");
  let integration: PrivateIntegration = integration.into();
  let res = HttpResponse::Ok().json(integration);
  Ok(res)
}

async fn create_subscription_integration(
  ctx: Ctx,
  body: SubscriptionCreateBody,
  user_id: UserID,
) -> Response {
  let user_id = user_id.0;
  let follower_list_id = to_object_id(body.subscribe_from.clone())?;
  let following_list_id = to_object_id(body.subscribe_to.clone())?;

  let follower_list = ctx
    .models
    .list
    .find_one(doc! { "_id": &follower_list_id, "user": &user_id }, None)
    .await?;

  let follower_list = match follower_list {
    Some(list) => list,
    None => {
      debug!("Follower List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let following_list = ctx
    .models
    .list
    .find_one(doc! { "_id": &following_list_id, "is_public": true }, None)
    .await?;

  let following_list = match following_list {
    Some(list) => list,
    None => {
      debug!("Following List not found or private, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  if follower_list.user == following_list.user {
    debug!("User can not subscribe to its own list");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let is_already_subscribed_to_list = ctx
    .models
    .integration
    .find_one(
      doc! { "user": &user_id, "listas-subscription.list": &following_list_id },
      None,
    )
    .await?
    .is_some();

  if is_already_subscribed_to_list {
    debug!("User can not subscribe twice to the same list");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let now = date::now();
  let integration = ctx
    .models
    .integration
    .create(integration::Integration {
      id: None,
      user: user_id.clone(),
      list: follower_list_id.clone(),
      created_at: now,
      updated_at: now,
      kind: integration::Kind::from_str("listas-subscription").unwrap(),
      rss: None,
      listas_subscription: Some(integration::subscription::ListasSubscription {
        list: following_list_id.clone(),
      }),
    })
    .await?;

  ctx
    .models
    .list
    .update_last_activity_at(&follower_list_id)
    .await
    .map_err(|err| {
      error!(
        "Failed to update last activity for list {}. Error {}",
        &follower_list_id, err
      )
    })?;

  debug!("Returning integration and 200 status code");
  let integration: PrivateIntegration = integration.into();
  let res = HttpResponse::Ok().json(integration);
  Ok(res)
}

async fn remove_integration(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let user_id = user_id.0;
  let integration_id = id.0;

  let integration = ctx
    .models
    .integration
    .find_one(doc! { "_id": &integration_id, "user": &user_id }, None)
    .await?;

  let integration = match integration {
    Some(integration) => integration,
    None => {
      debug!("Integration not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Removing integration");
  ctx
    .models
    .integration
    .remove(integration.id.as_ref().unwrap())
    .await?;

  ctx
    .models
    .list
    .update_last_activity_at(&integration.list)
    .await
    .map_err(|err| {
      error!(
        "Failed to update last activity for list {}. Error {}",
        &integration.list, err
      )
    })?;

  debug!("Integration removed, returning 204 status code");
  let res = HttpResponse::NoContent().finish();

  Ok(res)
}

async fn enrich_resource(ctx: &Ctx, mut resource: Resource) -> Result<Resource, Error> {
  let url = parse_url(resource.url.as_str())?;
  let metadata = ctx.models.resource.get_metadata(&url).await?;

  let metadata = match metadata {
    Some(metadata) => metadata,
    None => return Ok(resource),
  };

  if let Some(title) = metadata.title {
    resource.title = title
  }

  if let Some(description) = metadata.description {
    resource.description = Some(description)
  }

  if let Some(thumbnail) = metadata.thumbnail {
    resource.thumbnail = Some(thumbnail)
  }

  Ok(resource)
}
