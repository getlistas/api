use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::stream::StreamExt;
use futures::stream::TryStreamExt;
use serde::Deserialize;
use serde_json::json;
use wither::bson;
use wither::bson::doc;
use wither::mongodb;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::mongodb::options::FindOptions;
use wither::Model;

use crate::errors::ApiError;
use crate::lib::date;
use crate::lib::id::ID;
use crate::lib::util;
use crate::models::list;
use crate::models::list::List;
use crate::models::list::ListUpdate;
use crate::models::resource::Resource;
use crate::models::user::UserID;
use crate::Context;
use crate::{auth, errors};

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;

#[derive(Deserialize)]
struct ListCreateBody {
  pub title: String,
  pub is_public: bool,
  pub description: Option<String>,
  pub tags: Option<Vec<String>>,
}

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(
    web::resource("/lists/{id}")
      .route(web::get().to(find_list_by_id))
      .route(web::delete().to(remove_list))
      .route(web::put().to(update_list))
      .route(web::post().to(fork_list))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/lists")
      .route(web::get().to(query_lists))
      .route(web::post().to(create_list))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/lists/{id}/fork")
      .route(web::post().to(fork_list))
      .wrap(auth.clone()),
  );
}

async fn find_list_by_id(ctx: web::Data<Context>, id: ID, user: UserID) -> Response {
  let list = List::find_one(
    &ctx.database.conn,
    doc! {
        "_id": id.0,
        "user": user.0
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

  debug!("Returning list");
  let list = list.to_schema(&ctx.database.conn).await?;
  let res = HttpResponse::Ok().json(list);
  Ok(res)
}

async fn query_lists(ctx: web::Data<Context>, user: UserID) -> Response {
  let sort = doc! { "created_at": 1 };
  let options = FindOptions::builder().sort(Some(sort)).build();
  let mut lists = List::find(&ctx.database.conn, doc! { "user": user.0 }, options)
    .await
    .map_err(ApiError::WitherError)?
    .try_collect::<Vec<List>>()
    .await
    .map_err(ApiError::WitherError)?;

  let mut populated_lists = vec![];
  for list in lists.iter_mut() {
    let conn = ctx.database.conn.clone();
    let task = async move { list.to_schema(&conn).await };
    populated_lists.push(task);
  }

  debug!("Querying list resources metadata");
  let lists = futures::stream::iter(populated_lists)
    .buffer_unordered(40)
    .collect::<Vec<Result<serde_json::Value, errors::ApiError>>>()
    .await
    .into_iter()
    .collect::<Result<serde_json::Value, ApiError>>()?;

  debug!("Returning lists");
  let res = HttpResponse::Ok().json(lists);
  Ok(res)
}

async fn create_list(ctx: Ctx, body: web::Json<ListCreateBody>, user: UserID) -> Response {
  let now = date::now();
  let tags = body.tags.clone().map(util::sanitize_tags).unwrap_or(vec![]);
  let slug = util::to_slug_case(body.title.clone());
  let mut list = List {
    id: None,
    user: user.0,
    title: body.title.clone(),
    description: body.description.clone(),
    is_public: body.is_public.clone(),
    tags,
    slug,
    fork: None,
    created_at: now,
    updated_at: now,
  };

  list
    .save(&ctx.database.conn, None)
    .await
    .map_err(ApiError::WitherError)?;

  debug!("Returning created list");
  let res = HttpResponse::Created().json(list.to_json());
  Ok(res)
}

async fn update_list(ctx: web::Data<Context>, id: ID, body: web::Json<ListUpdate>) -> Response {
  let mut body = body.into_inner();
  let body = ListUpdate::new(&mut body);
  let update = json!({ "$set": body });

  let update = bson::ser::to_document(&update).unwrap();
  let update_options = FindOneAndUpdateOptions::builder()
    .return_document(mongodb::options::ReturnDocument::After)
    .build();

  let list = List::find_one_and_update(
    &ctx.database.conn,
    doc! { "_id": id.0 },
    update,
    update_options,
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

  debug!("Returning updated list");
  let res = HttpResponse::Ok().json(list.to_json());
  Ok(res)
}

async fn fork_list(ctx: web::Data<Context>, id: ID, user: UserID) -> Response {
  let list = List::find_one(
    &ctx.database.conn,
    doc! {
        "_id": id.0,
        "is_public": true
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

  if list.user == user.0 {
    debug!("User can not fork its own list, returning 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let now = date::now();
  let mut forked_list = List {
    id: None,
    user: user.0,
    title: list.title.clone(),
    description: list.description.clone(),
    is_public: list.is_public.clone(),
    tags: list.tags.clone(),
    slug: list.slug.clone(),
    created_at: now,
    updated_at: now,

    fork: Some(list::Fork {
      from: list.id.clone().unwrap(),
      at: now,
    }),
  };

  forked_list
    .save(&ctx.database.conn, None)
    .await
    .map_err(ApiError::WitherError)?;

  let resources = Resource::find(
    &ctx.database.conn,
    doc! { "list": list.id.clone().unwrap() },
    None,
  )
  .await
  .map_err(ApiError::WitherError)?
  .try_collect::<Vec<Resource>>()
  .await
  .map_err(ApiError::WitherError)?;

  debug!("Creating forked resources");
  let mut forked_resources = resources
    .into_iter()
    .map(|resource| Resource {
      id: None,
      user: resource.user.clone(),
      list: forked_list.id.clone().unwrap(),
      position: resource.position.clone(),
      url: resource.url.clone(),
      title: resource.title.clone(),
      description: resource.description.clone(),
      thumbnail: resource.thumbnail.clone(),
      tags: resource.tags.clone(),
      created_at: now,
      updated_at: now,
      completed_at: None,
    })
    .collect::<Vec<Resource>>();

  let mut resource_futures = vec![];
  for resource in forked_resources.iter_mut() {
    let conn = ctx.database.conn.clone();
    let task = async move {
      resource
        .save(&conn, None)
        .await
        .map_err(ApiError::WitherError)
    };
    resource_futures.push(task);
  }

  debug!("Storing forked resources");
  futures::stream::iter(resource_futures)
    .buffer_unordered(20)
    .collect::<Vec<Result<(), errors::ApiError>>>()
    .await
    .into_iter()
    .collect::<Result<(), ApiError>>()?;

  debug!("Returning forked list");
  let res = HttpResponse::Ok().json(forked_list.to_json());
  Ok(res)
}

async fn remove_list(ctx: web::Data<Context>, id: ID, user: UserID) -> Response {
  let list = List::find_one(
    &ctx.database.conn,
    doc! {
        "_id": &id.0,
        "user": user.0
    },
    None,
  )
  .await
  .map_err(ApiError::WitherError)?;

  if list.is_none() {
    debug!("List not found, returning 404 status code");
    return Ok(HttpResponse::NotFound().finish());
  }

  debug!("Removing resources associated to this list");
  Resource::collection(&ctx.database.conn)
    .delete_many(doc! { "list": &id.0 }, None)
    .await
    .map_err(ApiError::MongoError)?;

  debug!("Removing list");
  list
    .unwrap()
    .delete(&ctx.database.conn)
    .await
    .map_err(ApiError::WitherError)?;

  debug!("List removed, returning 204 status code");
  let res = HttpResponse::NoContent().finish();

  Ok(res)
}
