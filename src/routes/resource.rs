use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::Deserialize;
use serde_json::json;
use validator::Validate;
use wither::bson;
use wither::bson::{doc, Bson};
use wither::mongodb;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::mongodb::options::FindOptions;
use wither::Model as WitherModelTrait;

use crate::actors::subscription;
use crate::auth::UserID;
use crate::lib::id::ID;
use crate::lib::util::to_object_id;
use crate::models::resource::PrivateResource;
use crate::models::resource::Resource;
use crate::models::resource::ResourceUpdate;
use crate::models::Model as ModelTrait;
use crate::Context;
use crate::{auth, lib::date};
use crate::{errors::Error, lib::util};

#[derive(Deserialize)]
struct Query {
  list: Option<String>,
  is_completed: Option<bool>,
}

#[derive(Deserialize)]
pub struct ResourceCreate {
  pub list: String,
  pub url: String,
  pub title: String,
  pub description: Option<String>,
  pub thumbnail: Option<String>,
  pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct PositionUpdate {
  pub list: String,
  pub previus: Option<String>,
}

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;
type ResourceCreateBody = web::Json<ResourceCreate>;
type PositionUpdateBody = web::Json<PositionUpdate>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(
    web::resource("/resources/{id}")
      .route(web::get().to(get_resource_by_id))
      .route(web::put().to(update_resource))
      .route(web::delete().to(remove_resource))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/resources/{id}/complete")
      .route(web::post().to(complete_resource))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/resources/{id}/position")
      .route(web::put().to(update_position))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/resources")
      .route(web::get().to(query_resources))
      .route(web::post().to(create_resource))
      .wrap(auth),
  );
}

async fn get_resource_by_id(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let resource = ctx
    .models
    .resource
    .find_one(doc! { "_id": &resource_id, "user": &user_id }, None)
    .await?;

  let resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Returning resource");
  let res = HttpResponse::Ok().json(resource.to_json());
  Ok(res)
}

async fn query_resources(ctx: Ctx, user_id: UserID, qs: web::Query<Query>) -> Response {
  let user_id = user_id.0;
  let mut query = doc! { "user": user_id };

  if let Some(list_id) = qs.list.clone() {
    let list_id = util::to_object_id(list_id)?;
    query.insert("list", list_id);
  }

  if let Some(is_complete) = qs.is_completed {
    // The { item : null } query matches documents that either contain the
    // item field whose value is null or that do not contain the item field.
    let key = if is_complete { "$ne" } else { "$eq" };
    query.insert("completed_at", doc! { key: Bson::Null });
  }

  let sort = match qs.is_completed {
    Some(true) => doc! { "completed_at": -1 },
    _ => doc! { "created_at": -1 },
  };

  let options = FindOptions::builder().sort(Some(sort)).build();

  let resources = ctx
    .models
    .resource
    .find(query, Some(options))
    .await?
    .into_iter()
    .map(Into::into)
    .collect::<Vec<PrivateResource>>();

  debug!("Returning resources");
  let res = HttpResponse::Ok().json(resources);
  Ok(res)
}

async fn create_resource(ctx: Ctx, body: ResourceCreateBody, user_id: UserID) -> Response {
  let list_id = to_object_id(body.list.clone())?;
  let user_id = user_id.0;
  let url = util::parse_url(body.url.clone().as_str())?;
  let tags = body
    .tags
    .clone()
    .map(util::sanitize_tags)
    .unwrap_or_default();

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": &list_id,"user": &user_id }, None)
    .await?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("Failed creating Resource, asociated List not found");
      return Ok(HttpResponse::BadRequest().finish());
    }
  };

  if list.user != user_id {
    debug!("Failed creating Resource, Can not create resource in a not owned List");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let position = ctx
    .models
    .list
    .get_position_for_new_resource(&list_id)
    .await?;

  let resource = Resource {
    id: None,
    position,
    tags,
    user: user_id,
    list: list_id,
    url: url.to_string(),
    title: body.title.clone(),
    description: body.description.clone(),
    thumbnail: body.thumbnail.clone(),
    created_at: date::now(),
    updated_at: date::now(),
    completed_at: None,
  };

  match resource.validate() {
    Ok(_) => (),
    Err(_err) => {
      debug!("Failed creating Resource, payload is not valid. Returning 400 status code");
      return Ok(HttpResponse::BadRequest().finish());
    }
  };

  let resource = ctx.models.resource.create(resource).await?;
  let resource_id = resource.id.clone().unwrap();

  ctx
    .actors
    .subscription
    .try_send(subscription::on_resource_created::ResourceCreated { resource_id })
    .map_err(|err| error!("Failed to send message to subscription actor, {}", err))?;

  debug!("Returning created resource");
  let resource: PrivateResource = resource.into();
  let res = HttpResponse::Created().json(resource);
  Ok(res)
}

async fn update_resource(
  ctx: Ctx,
  id: ID,
  body: web::Json<ResourceUpdate>,
  user_id: UserID,
) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let mut body = body.into_inner();
  let body = ResourceUpdate::new(&mut body);
  let update = json!({ "$set": body });

  let update = bson::ser::to_document(&update).unwrap();
  let options = FindOneAndUpdateOptions::builder()
    .return_document(mongodb::options::ReturnDocument::After)
    .build();

  let resource = ctx
    .models
    .resource
    .find_one_and_update(
      doc! { "_id": &resource_id, "user": &user_id },
      update,
      Some(options),
    )
    .await?;

  let resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Returning updated resource");
  let res = HttpResponse::Ok().json(resource.to_json());
  Ok(res)
}

async fn remove_resource(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let result = ctx
    .models
    .resource
    .delete_one(doc! { "_id": resource_id, "user": user_id })
    .await?;

  let res = match result.deleted_count {
    0 => {
      debug!("Resource not found, returning 404 status code");
      HttpResponse::NotFound().finish()
    }
    _ => {
      debug!("Resource removed, returning 204 status code");
      HttpResponse::NoContent().finish()
    }
  };

  Ok(res)
}

async fn complete_resource(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let resource = ctx
    .models
    .resource
    .find_one(doc! { "_id": resource_id, "user": user_id }, None)
    .await?;

  let mut resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  if resource.completed_at.is_some() {
    debug!("Resource was already completed, returnig 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  // TODO: Use an atomic update
  resource.completed_at = Some(date::now());
  resource
    .save(&ctx.database.conn, None)
    .await
    .map_err(Error::WitherError)?;

  debug!("Resource marked as completed, returning 202 status code");
  let res = HttpResponse::Accepted().finish();
  Ok(res)
}

async fn update_position(ctx: Ctx, id: ID, user_id: UserID, body: PositionUpdateBody) -> Response {
  let resource_id = id.0;
  let list_id = to_object_id(body.list.clone())?;
  let user_id = user_id.0;

  let resource = ctx
    .models
    .resource
    .find_one(
      doc! { "_id": &resource_id, "user": &user_id, "list": &list_id },
      None,
    )
    .await?;

  if resource.is_none() {
    debug!("Resource not found, returning 404 status code");
    return Ok(HttpResponse::NotFound().finish());
  }

  let position = match body.previus.clone() {
    Some(previus) => {
      let previus_id = to_object_id(previus)?;
      let query = doc! {
          "_id": &previus_id,
          "user": &user_id,
          "list": &list_id,
      };
      let position = match ctx.models.resource.get_position(query).await? {
        Some(position) => position,
        None => {
          debug!("Resource not found, returning 404 status code");
          return Ok(HttpResponse::NotFound().finish());
        }
      };

      position + 1
    }
    None => 0,
  };

  ctx
    .models
    .resource
    .update_many(
      doc! {
          "_id": doc! { "$ne": &resource_id },
          "user": &user_id,
          "list": &list_id,
          "position": doc! { "$gte": &position },
      },
      doc! {
          "$inc": doc! { "position": 1 }
      },
      None,
    )
    .await?;


  ctx
    .models
    .resource
    .update_one(
      doc! { "_id": &resource_id },
      doc! {
        "$set": {
          "position": position,
          "updated_at": bson::to_bson(&date::now()).unwrap()
        }
      },
      None,
    )
    .await?;

  debug!("Resource position updated, returning 202 status code");
  let res = HttpResponse::Accepted().finish();
  Ok(res)
}
