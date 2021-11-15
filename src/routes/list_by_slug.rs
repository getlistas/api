use actix_web::{web, HttpResponse};
use serde::Deserialize;
use wither::bson::doc;
use wither::bson::Bson;

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

#[derive(Deserialize)]
struct Query {
  completed: Option<bool>,
  search_text: Option<String>,
  skip: Option<u32>,
  limit: Option<u32>,
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
  qs: web::Query<Query>,
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
      debug!(
        "User not found for slug {}, returning 404 status code to the user",
        user_slug
      );
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let user_id = user.id.unwrap();
  let is_authenticated = auth.is_authenticated;
  let is_self = is_authenticated && auth.user_id.clone().unwrap() == user_id;
  let mut find_list_query = doc! { "user": &user_id, "slug": &list_slug };
  if !is_self {
    find_list_query.insert("is_public", true);
  }

  let list = ctx.models.list.find_one(find_list_query, None).await?;
  let list = match list {
    Some(list) => list,
    None => {
      debug!(
        "List not found for slug {}, returning 404 status code to the user",
        list_slug
      );
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let mut pipeline = vec![];
  let mut filter = vec![];
  let mut must = vec![];

  filter.push(doc! {
    "equals": {
      "path": "user",
      "value": user_id
    }
  });

  filter.push(doc! {
    "equals": {
      "path": "list",
      "value": list.id.clone().unwrap()
    }
  });

  if let Some(ref search_text) = qs.search_text {
    must.push(doc! {
      "text": {
        "query": search_text,
        "path": ["title", "description", "tags"],
        "fuzzy": {
          "maxEdits": 2,
          "prefixLength": 3
        }
      }
    });
  }

  pipeline.push(doc! {
    "$search": {
      "index": "search",
      "compound": {
        "filter": filter,
        "must": must
      }
    }
  });

  if let Some(is_completed) = qs.completed {
    // The { item : null } query matches documents that either contain the
    // item field whose value is null or that do not contain the item field.
    let key = if is_completed { "$ne" } else { "$eq" };
    pipeline.push(doc! {
      "$match": {
        "completed_at": { key: Bson::Null }
      }
    });
  }

  // When querying using full text search, use the score order to sort data.
  if qs.search_text.is_none() {
    pipeline.push(doc! { "$sort": { "position": 1 } });
  }

  if let Some(skip) = qs.skip {
    pipeline.push(doc! { "$skip": skip });
  }

  if let Some(limit) = qs.limit {
    pipeline.push(doc! { "$limit": limit });
  }

  let resources = ctx
    .models
    .resource
    .aggregate::<PrivateResource>(pipeline)
    .await?;

  debug!("Returning resources");
  let res = HttpResponse::Ok().json(resources);
  Ok(res)
}
