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

use crate::auth;
use crate::errors::ApiError;
use crate::lib::id::ID;
use crate::models::resource::Resource;
use crate::models::resource::ResourceCreate;
use crate::models::resource::ResourceUpdate;
use crate::models::user::UserID;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;

#[derive(Deserialize)]
struct Query {
    list: Option<String>,
    completed: Option<bool>,
}

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
        web::resource("/resources")
            .route(web::get().to(get_resources))
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
            debug!("Resource not found, returning 404 status code to the client");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    debug!("Returning resource to the client");
    let res = HttpResponse::Ok().json(resource.to_json());
    Ok(res)
}

async fn get_resources(ctx: Ctx, user_id: UserID, qs: web::Query<Query>) -> Response {
    let mut query = doc! { "user": user_id.0 };
    let sort = doc! { "position": -1 };
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

    let resources = Resource::find(&ctx.database.conn, query, Some(options))
        .await
        .map_err(ApiError::WitherError)?
        .try_collect::<Vec<Resource>>()
        .await
        .map_err(ApiError::WitherError)?;

    let resources = resources
        .iter()
        .map(|resource| resource.to_json())
        .collect::<Vec<serde_json::Value>>();

    debug!("Returning resources to the client");
    let res = HttpResponse::Ok().json(resources);
    Ok(res)
}

async fn create_resource(ctx: Ctx, body: web::Json<ResourceCreate>, user_id: UserID) -> Response {
    let list_id = ObjectId::with_string(body.list.as_str()).map_err(ApiError::ParseObjectID)?;

    let last_resource = Resource::find_last(&ctx.database.conn, &user_id.0, &list_id)
        .await
        .map_err(ApiError::WitherError)?;

    let position = last_resource
        .map(|resource| resource.position + 1)
        .unwrap_or(0);

    let mut resource = Resource::new(body.into_inner(), user_id.0, list_id, position);

    resource
        .save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning created resource to the client");
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
            debug!("Resource not found, returning 404 status code to the client");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    debug!("Returning updated resource to the client");
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
            debug!("Resource removed, returning 204 status code to the client");
            HttpResponse::NoContent().finish()
        }
        None => {
            debug!("Resource not found, returning 404 status code to the client");
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
            debug!("Resource not found, returning 404 status code to the client");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    if resource.completed_at.is_some() {
        debug!("Resource was already completed, returnig 400 status code to the client");
        return Ok(HttpResponse::BadRequest().finish());
    }

    resource.completed_at = Some(chrono::Utc::now().into());
    resource
        .save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Resource marked as completed, returning 202 status code to the client");
    let res = HttpResponse::Accepted().finish();
    Ok(res)
}
