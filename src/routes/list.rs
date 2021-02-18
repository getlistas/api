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

use crate::auth;
use crate::errors::Error;
use crate::lib::date;
use crate::lib::id::ID;
use crate::lib::util;
use crate::models::list;
use crate::models::list::List;
use crate::models::list::ListUpdate;
use crate::models::resource::Resource;
use crate::models::user::UserID;
use crate::Context;

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
  .map_err(Error::WitherError)?;

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
  let options = FindOptions::builder().sort(sort).build();
  let mut lists = List::find(&ctx.database.conn, doc! { "user": user.0 }, options)
    .await
    .map_err(Error::WitherError)?
    .try_collect::<Vec<List>>()
    .await
    .map_err(Error::WitherError)?;

  let mut populated_lists = vec![];
  for list in lists.iter_mut() {
    let conn = ctx.database.conn.clone();
    let task = async move { list.to_schema(&conn).await };
    populated_lists.push(task);
  }

  debug!("Querying list resources metadata");
  let lists = futures::stream::iter(populated_lists)
    .buffered(50)
    .collect::<Vec<Result<serde_json::Value, Error>>>()
    .await
    .into_iter()
    .collect::<Result<serde_json::Value, Error>>()?;

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
    .map_err(Error::WitherError)?;

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
  .map_err(Error::WitherError)?;

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
  let user_id = user.0.clone();

  let list = List::find_one(
    &ctx.database.conn,
    doc! {
        "_id": id.0,
        "is_public": true
    },
    None,
  )
  .await
  .map_err(Error::WitherError)?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  if list.user == user_id {
    debug!("User can not fork its own list, returning 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let now = date::now();
  let mut forked_list = List {
    id: None,
    user: user_id.clone(),
    title: list.title.clone(),
    description: list.description.clone(),
    // TODO: this option should come in the request body and we should default to
    // false.
    is_public: false,
    tags: list.tags.clone(),
    // TODO: We should maybe postfix a `forked` string to avoid collitions. Then
    // the user should be able to update this field.
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
    .map_err(Error::WitherError)?;

  let resources = Resource::find(
    &ctx.database.conn,
    doc! { "list": list.id.clone().unwrap() },
    None,
  )
  .await
  .map_err(Error::WitherError)?
  .try_collect::<Vec<Resource>>()
  .await
  .map_err(Error::WitherError)?;

  debug!("Creating forked resources");
  let forked_list_id = forked_list.id.clone().unwrap();
  let forked_resources = resources
    .into_iter()
    .map(move |resource| {
      let conn = ctx.database.conn.clone();
      let mut forked_resource = Resource {
        id: None,
        user: user_id.clone(),
        list: forked_list_id.clone(),
        position: resource.position,
        url: resource.url.clone(),
        title: resource.title.clone(),
        description: resource.description.clone(),
        thumbnail: resource.thumbnail.clone(),
        tags: resource.tags.clone(),
        created_at: now,
        updated_at: now,
        completed_at: None,
      };

      async move { forked_resource.save(&conn, None).await.map_err(Error::WitherError) }
    });

  debug!("Storing forked resources from forked list");
  futures::stream::iter(forked_resources)
    .buffer_unordered(50)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<(), Error>>()?;

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
  .map_err(Error::WitherError)?;

  if list.is_none() {
    debug!("List not found, returning 404 status code");
    return Ok(HttpResponse::NotFound().finish());
  }

  debug!("Removing resources associated to this list");
  Resource::collection(&ctx.database.conn)
    .delete_many(doc! { "list": &id.0 }, None)
    .await
    .map_err(Error::MongoError)?;

  debug!("Removing list");
  list
    .unwrap()
    .delete(&ctx.database.conn)
    .await
    .map_err(Error::WitherError)?;

  debug!("List removed, returning 204 status code");
  let res = HttpResponse::NoContent().finish();

  Ok(res)
}
