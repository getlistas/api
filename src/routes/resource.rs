use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::stream::TryStreamExt;
use serde::Deserialize;
use serde_json::json;
use wither::bson;
use wither::bson::{doc, oid::ObjectId, Bson};
use wither::mongodb;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::mongodb::options::FindOptions;
use wither::Model;

use crate::errors::ApiError;
use crate::lib::id::ID;
use crate::lib::util::to_object_id;
use crate::models::resource::Resource;
use crate::models::resource::ResourceUpdate;
use crate::models::user::UserID;
use crate::Context;
use crate::{auth, lib::date};

#[derive(Deserialize)]
struct Query {
    list: Option<String>,
    completed: Option<bool>,
    sort: Option<String>,
}

#[derive(Deserialize)]
pub struct ResourceCreate {
    pub list: String,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
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
            .wrap(auth.clone()),
    );
}

async fn get_resource_by_id(ctx: Ctx, id: ID, user_id: UserID) -> Response {
    let resource = Resource::find_one(
        &ctx.database.conn,
        doc! {
            "_id": id.0,
            "user": user_id.0,
        },
        None,
    )
    .await
    .map_err(ApiError::WitherError)?;

    let resource = match resource {
        Some(resource) => resource,
        None => {
            debug!("Resource not found, returning 404");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    debug!("Returning resource");
    let res = HttpResponse::Ok().json(resource.to_json());
    Ok(res)
}

async fn query_resources(ctx: Ctx, user_id: UserID, qs: web::Query<Query>) -> Response {
    let sort_option = qs.sort.clone().unwrap_or("position_asc".into());
    let mut query = doc! { "user": user_id.0 };

    let sort = match sort_option.as_str() {
        "position_asc" => doc! { "position": 1 },
        "position_des" => doc! { "position": -1 },
        "date_asc" => doc! { "completed_at": 1, "created_at": 1 },
        "date_des" => doc! { "completed_at": -1, "created_at": -1 },
        _ => doc! { "position": 1 },
    };

    let options = FindOptions::builder().sort(Some(sort)).build();

    if qs.list.is_some() {
        let list_id = ObjectId::with_string(qs.list.clone().unwrap().as_str())
            .map_err(ApiError::ParseObjectID)?;
        query.insert("list", list_id);
    }

    if qs.completed.is_some() {
        let completed = qs.completed.unwrap();
        // The { item : null } query matches documents that either contain the
        // item field whose value is null or that do not contain the item field.
        let key = if completed { "$ne" } else { "$eq" };
        query.insert("completed_at", doc! { key: Bson::Null });
    }

    let resources = Resource::find(&ctx.database.conn, query, options)
        .await
        .map_err(ApiError::WitherError)?
        .try_collect::<Vec<Resource>>()
        .await
        .map_err(ApiError::WitherError)?;

    let resources = resources
        .iter()
        .map(|resource| resource.to_json())
        .collect::<Vec<serde_json::Value>>();

    debug!("Returning resources");
    let res = HttpResponse::Ok().json(resources);
    Ok(res)
}

async fn create_resource(ctx: Ctx, body: ResourceCreateBody, user_id: UserID) -> Response {
    let list_id = to_object_id(body.list.clone().into())?;
    let user_id = user_id.0;

    let last_resource = Resource::find_last(&ctx.database.conn, &user_id, &list_id).await?;

    let position = last_resource
        .map(|resource| resource.position + 1)
        .unwrap_or(0);

    let mut resource = Resource {
        id: None,
        position,
        user: user_id,
        list: list_id,
        url: body.url.clone(),
        title: body.title.clone(),
        description: body.description.clone(),
        thumbnail: body.thumbnail.clone(),
        created_at: date::now(),
        updated_at: date::now(),
        completed_at: None,
    };

    resource
        .save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning created resource");
    let res = HttpResponse::Created().json(resource.to_json());
    Ok(res)
}

async fn update_resource(
    ctx: Ctx,
    id: ID,
    body: web::Json<ResourceUpdate>,
    user_id: UserID,
) -> Response {
    let mut body = body.into_inner();
    let body = ResourceUpdate::new(&mut body);
    let update = json!({ "$set": body });

    let update = bson::ser::to_document(&update).unwrap();
    let update_options = FindOneAndUpdateOptions::builder()
        .return_document(mongodb::options::ReturnDocument::After)
        .build();

    let resource = Resource::find_one_and_update(
        &ctx.database.conn,
        doc! {
            "_id": id.0,
            "user": user_id.0
        },
        update,
        update_options,
    )
    .await
    .map_err(ApiError::WitherError)?;

    let resource = match resource {
        Some(resource) => resource,
        None => {
            debug!("Resource not found, returning 404");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    debug!("Returning updated resource");
    let res = HttpResponse::Ok().json(resource.to_json());
    Ok(res)
}

async fn remove_resource(ctx: Ctx, id: ID, user_id: UserID) -> Response {
    let resource = Resource::find_one_and_delete(
        &ctx.database.conn,
        doc! {
            "_id": id.0,
            "user": user_id.0
        },
        None,
    )
    .await
    .map_err(ApiError::WitherError)?;

    let res = match resource {
        Some(_) => {
            debug!("Resource removed, returning 204");
            HttpResponse::NoContent().finish()
        }
        None => {
            debug!("Resource not found, returning 404");
            HttpResponse::NotFound().finish()
        }
    };

    Ok(res)
}

async fn complete_resource(ctx: Ctx, id: ID, user_id: UserID) -> Response {
    let resource = Resource::find_one(
        &ctx.database.conn,
        doc! {
            "_id": id.0,
            "user": user_id.0
        },
        None,
    )
    .await
    .map_err(ApiError::WitherError)?;

    let mut resource = match resource {
        Some(resource) => resource,
        None => {
            debug!("Resource not found, returning 404");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    if resource.completed_at.is_some() {
        debug!("Resource was already completed, returnig 400");
        return Ok(HttpResponse::BadRequest().finish());
    }

    resource.completed_at = Some(chrono::Utc::now().into());
    resource
        .save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Resource marked as completed, returning 202");
    let res = HttpResponse::Accepted().finish();
    Ok(res)
}

async fn update_position(ctx: Ctx, id: ID, user_id: UserID, body: PositionUpdateBody) -> Response {
    let id = id.0;
    let list_id = to_object_id(body.list.clone())?;
    let user_id = user_id.0;

    let resource = Resource::find_one(
        &ctx.database.conn,
        doc! { "_id": &id, "user": &user_id, "list": &list_id },
        None,
    )
    .await
    .map_err(ApiError::WitherError)?;

    let mut resource = match resource {
        Some(resource) => resource,
        None => {
            debug!("Resource not found, returning 404 status code");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    let position = match body.previus.clone() {
        Some(previus) => {
            let previus_id = to_object_id(previus)?;
            let query = doc! {
                "_id": &previus_id,
                "user": &user_id,
                "list": &list_id,
            };
            let position = match Resource::get_position(&ctx.database.conn, query).await? {
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

    Resource::collection(&ctx.database.conn)
        .update_many(
            doc! {
                "_id": doc! { "$ne": &id },
                "user": &user_id,
                "list": &list_id,
                "position": doc! { "$gte": &position },
            },
            doc! {
                "$inc": doc! { "position": 1 }
            },
            None,
        )
        .await
        .map_err(ApiError::MongoError)?;

    resource.position = position;
    resource.updated_at = date::now();
    resource
        .save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Resource position updated, returning 202 status code");
    let res = HttpResponse::Accepted().finish();
    Ok(res)
}
