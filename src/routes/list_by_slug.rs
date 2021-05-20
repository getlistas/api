use actix_web::{web, HttpResponse};
use serde::Deserialize;
use wither::bson::doc;

use crate::auth::AuthenticationMetadata;
use crate::models::resource::PrivateResource;
use crate::models::Model as ModelTrait;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

#[derive(Deserialize)]
struct Params {
  user_slug: String,
  list_slug: Option<String>,
}

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("users/{user_slug}/lists").route(web::get().to(query_lists)));
  cfg.service(web::resource("users/{user_slug}/lists/{list_slug}").route(web::get().to(find_list)));
  cfg.service(
    web::resource("users/{user_slug}/lists/{list_slug}/resources")
      .route(web::get().to(query_resources)),
  );
}

async fn query_lists(
  ctx: web::Data<Context>,
  params: web::Path<Params>,
  auth: AuthenticationMetadata,
) -> Response {
  let user = ctx
    .models
    .user
    .find_one(doc! { "slug": &params.user_slug }, None)
    .await?;

  let user = match user {
    Some(user) => user,
    None => {
      debug!("User not found for slug, returning 404 status code to the user");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let is_authenticated = auth.is_authenticated;
  let is_self = is_authenticated && auth.user_id.clone().unwrap() == user.id.clone().unwrap();
  let mut query = doc! { "user": user.id.unwrap() };
  if !is_self {
    query.insert("is_public", true);
  }

  // TODO: Review where we plan to use this endpoint, we might be exposing
  // too much information from the list.
  let lists = ctx.models.list.get_private_lists(query).await?;

  debug!("Returning list to the user");
  let res = HttpResponse::Ok().json(lists);
  Ok(res)
}

async fn find_list(
  ctx: web::Data<Context>,
  params: web::Path<Params>,
  auth: AuthenticationMetadata,
) -> Response {
  let list_slug = params.list_slug.clone().unwrap();
  let user_slug = &params.user_slug;
  let user = ctx
    .models
    .user
    .find_one(doc! { "slug": user_slug }, None)
    .await?;

  let user = match user {
    Some(user) => user,
    None => {
      debug!("User not found for slug, returning 404 status code to the user");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let is_authenticated = auth.is_authenticated;
  let is_self = is_authenticated && auth.user_id.clone().unwrap() == user.id.clone().unwrap();
  let mut query = doc! {
      "user": user.id.unwrap(),
      "slug": &list_slug
  };
  if !is_self {
    query.insert("is_public", true);
  }

  let list = ctx.models.list.find_one(query, None).await?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  // TODO: Review where we plan to use this endpoint, we might be exposing
  // too much information from the list.
  let list = ctx.models.list.to_private_schema(&list).await?;

  debug!("Returning list to the user");
  let res = HttpResponse::Ok().json(list);
  Ok(res)
}

async fn query_resources(
  ctx: web::Data<Context>,
  params: web::Path<Params>,
  auth: AuthenticationMetadata,
) -> Response {
  let user = ctx
    .models
    .user
    .find_one(doc! { "slug": &params.user_slug }, None)
    .await?;

  let user = match user {
    Some(user) => user,
    None => {
      debug!("User not found for slug, returning 404 status code to the user");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let user_id = user.id.unwrap();
  let is_authenticated = auth.is_authenticated;
  let is_self = is_authenticated && auth.user_id.clone().unwrap() == user_id;
  let mut find_list_query = doc! { "user": &user_id };
  if !is_self {
    find_list_query.insert("is_public", true);
  }

  let list = ctx.models.list.find_one(find_list_query, None).await?;
  let list = match list {
    Some(list) => list,
    None => {
      debug!("List not found for slug, returning 404 status code to the user");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  // TODO: Review what private resource content we are exposing.
  let resources = ctx
    .models
    .resource
    .find(doc! { "user": user_id, "list": list.id.unwrap() }, None)
    .await?
    .into_iter()
    .map(Into::into)
    .collect::<Vec<PrivateResource>>();

  debug!("Returning resources to the user");
  let res = HttpResponse::Ok().json(resources);
  Ok(res)
}
