use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::stream::StreamExt;
use serde::Deserialize;
use serde_json::json;
use wither::bson;
use wither::bson::doc;
use wither::mongodb;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::Model as WitherModelTrait;

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
use crate::models::Model as ModelTrait;
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

  cfg.service(
    web::resource("/lists/{id}/follow")
      .route(web::post().to(follow_list))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/lists/{id}/archive")
      .route(web::post().to(archive_list))
      .wrap(auth.clone()),
  );
}

async fn find_list_by_id(ctx: web::Data<Context>, id: ID, user: UserID) -> Response {
  let user_id = user.0;
  let list_id = id.0;

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": &list_id, "user": &user_id }, None)
    .await?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Returning list");
  let list = ctx.models.list.to_private_schema(&list).await?;
  let res = HttpResponse::Ok().json(list);
  Ok(res)
}

async fn query_lists(ctx: web::Data<Context>, user: UserID) -> Response {
  let user_id = user.0;

  let lists = ctx
    .models
    .list
    .get_private_lists(doc! { "user": &user_id })
    .await?;

  debug!("Returning lists");
  let res = HttpResponse::Ok().json(lists);
  Ok(res)
}

async fn create_list(ctx: Ctx, body: web::Json<ListCreateBody>, user: UserID) -> Response {
  let now = date::now();
  let tags = body.tags.clone().map(util::sanitize_tags).unwrap_or(vec![]);
  let slug = util::to_slug_case(body.title.clone());
  let list = List {
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
    archived_at: None,
  };

  let list = ctx.models.list.create(list).await?;
  let list = ctx.models.list.to_private_schema(&list).await?;

  debug!("Returning created list");
  let res = HttpResponse::Created().json(list);
  Ok(res)
}

async fn update_list(ctx: web::Data<Context>, id: ID, body: web::Json<ListUpdate>) -> Response {
  let list_id = id.0;
  let mut body = body.into_inner();
  let body = ListUpdate::new(&mut body);
  let update = json!({ "$set": body });

  let update = bson::ser::to_document(&update).unwrap();
  let update_options = FindOneAndUpdateOptions::builder()
    .return_document(mongodb::options::ReturnDocument::After)
    .build();

  let list = ctx
    .models
    .list
    .find_one_and_update(doc! { "_id": list_id }, update, Some(update_options))
    .await?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let list = ctx.models.list.to_private_schema(&list).await?;

  debug!("Returning updated list");
  let res = HttpResponse::Ok().json(list);
  Ok(res)
}

async fn fork_list(ctx: web::Data<Context>, id: ID, user: UserID) -> Response {
  let list_id = id.0;
  let user_id = user.0;

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": &list_id, "is_public": true }, None)
    .await?;

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
  let forked_list = List {
    id: None,
    user: user_id.clone(),
    title: list.title.clone(),
    description: list.description.clone(),
    is_public: false,
    tags: list.tags.clone(),
    // TODO: We should maybe postfix a `forked` string to avoid collitions. Then
    // the user should be able to update this field.
    slug: list.slug.clone(),
    created_at: now,
    updated_at: now,
    archived_at: None,
    fork: Some(list::Fork {
      list: list.id.clone().unwrap(),
      user: list.user.clone(),
    }),
  };

  let forked_list = ctx.models.list.create(forked_list).await?;

  let resources = ctx
    .models
    .resource
    .find(doc! { "list": list.id.clone().unwrap() }, None)
    .await?;

  debug!("Creating forked resources");
  let forked_list_id = forked_list.id.clone().unwrap();
  let models = ctx.models.clone();
  let forked_resources = resources.into_iter().map(move |resource| {
    let models = models.clone();
    let forked_resource = Resource {
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

    async move {
      models.resource.create(forked_resource).await?;
      Ok::<(), Error>(())
    }
  });

  debug!("Storing forked resources from forked list");
  futures::stream::iter(forked_resources)
    .buffer_unordered(50)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<(), Error>>()?;

  let forked_list = ctx.models.list.to_private_schema(&forked_list).await?;

  debug!("Returning forked list");
  let res = HttpResponse::Ok().json(forked_list);
  Ok(res)
}

async fn follow_list(ctx: web::Data<Context>, id: ID, user: UserID) -> Response {
  let parent_list_id = &id.0;
  let user_id = &user.0;

  let parent_list = ctx
    .models
    .list
    .find_one(doc! { "_id": parent_list_id, "is_public": true }, None)
    .await?;

  let parent_list = match parent_list {
    Some(parent_list) => parent_list,
    None => {
      debug!("List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  if parent_list.user == *user_id {
    debug!("User can not follow its own list, returning 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let now = date::now();
  let list = List {
    id: None,
    user: user_id.clone(),
    title: parent_list.title.clone(),
    description: parent_list.description.clone(),
    is_public: false,
    tags: parent_list.tags.clone(),
    // TODO: We should maybe postfix a `follow` string to avoid collitions. Then
    // the user should be able to update this field.
    slug: parent_list.slug.clone(),
    created_at: now,
    updated_at: now,
    archived_at: None,
    fork: None,
  };

  let list = ctx.models.list.create(list).await?;

  let parent_list_resources = ctx
    .models
    .resource
    .find(doc! { "list": parent_list_id }, None)
    .await?;

  debug!("Creating resources from parent list");
  let list_id = list.id.clone().unwrap();
  let resources = parent_list_resources
    .into_iter()
    .map(move |parent_resource| {
      let conn = ctx.database.conn.clone();
      let mut resource = Resource {
        id: None,
        user: user_id.clone(),
        list: list_id.clone(),
        position: parent_resource.position,
        url: parent_resource.url.clone(),
        title: parent_resource.title.clone(),
        description: parent_resource.description.clone(),
        thumbnail: parent_resource.thumbnail.clone(),
        tags: parent_resource.tags.clone(),
        created_at: now,
        updated_at: now,
        completed_at: None,
      };

      // TODO: Use model.create method
      async move { resource.save(&conn, None).await.map_err(Error::WitherError) }
    });

  debug!("Storing resources from followed list");
  futures::stream::iter(resources)
    .buffer_unordered(50)
    .collect::<Vec<Result<(), Error>>>()
    .await
    .into_iter()
    .collect::<Result<(), Error>>()?;

  debug!("Returning followed list");
  let res = HttpResponse::Ok().finish();
  Ok(res)
}

// TODO: Update route to use ctx.models.list.delete_one instead of first finding
// the list.
async fn remove_list(ctx: web::Data<Context>, id: ID, user: UserID) -> Response {
  let user_id = user.0;
  let list_id = id.0;

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": &list_id, "user": &user_id }, None)
    .await?;

  if list.is_none() {
    debug!("List not found, returning 404 status code");
    return Ok(HttpResponse::NotFound().finish());
  }

  debug!("Removing list");
  ctx.models.list.remove(&list_id).await?;

  debug!("List removed, returning 204 status code");
  let res = HttpResponse::NoContent().finish();

  Ok(res)
}

async fn archive_list(ctx: web::Data<Context>, id: ID, user: UserID) -> Response {
  let user_id = user.0;
  let list_id = id.0;

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": &list_id, "user": &user_id }, None)
    .await?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("List not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  if list.archived_at.is_some() {
    debug!("List was already archived, returning 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let completed_resources_count = ctx
    .models
    .resource
    .count(doc! { "list": &list_id, "completed_at": { "$exists": true } })
    .await?;

  if completed_resources_count == 0 {
    debug!("Can not archive list with no completed resources, returning 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  debug!("Archiving list");
  ctx.models.list.archive(&list_id).await?;

  debug!("List archived, returning 204 status code");
  let res = HttpResponse::NoContent().finish();

  Ok(res)
}
