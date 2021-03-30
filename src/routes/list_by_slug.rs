use actix_web::{web, HttpResponse};
use serde::Deserialize;
use wither::bson::doc;

use crate::auth::AuthenticationMetadata;
use crate::models::list::List;
use crate::models::user::User;
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
    .find_one::<User>(doc! { "slug": &params.user_slug })
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

  let lists = ctx.models.find::<List>(query, None).await?;

  let lists = lists
    .iter()
    .map(|list| list.to_json())
    .collect::<Vec<serde_json::Value>>();

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
    .find_one::<User>(doc! { "slug": user_slug })
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

  let list = ctx.models.find_one::<List>(query).await?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Returning list to the user");
  let res = HttpResponse::Ok().json(list.to_json());
  Ok(res)
}
