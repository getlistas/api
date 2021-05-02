use actix_web::{web, HttpResponse};
use serde::Deserialize;
use wither::bson::doc;

use crate::auth::AuthenticationMetadata;
use crate::models::Model as ModelTrait;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

#[derive(Deserialize)]
struct Params {
  user_slug: String,
  list_slug: Option<String>,
}

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("users/{user_slug}/lists").route(web::get().to(query_lists_by_slug)));

  cfg.service(
    web::resource("users/{user_slug}/lists/{list_slug}").route(web::get().to(find_list_by_slug)),
  );
}

async fn query_lists_by_slug(
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

async fn find_list_by_slug(
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
