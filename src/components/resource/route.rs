use actix_web::{web, HttpResponse};
use futures::stream::TryStreamExt;
use serde_json::json;
use wither::bson;
use wither::bson::doc;
use wither::mongodb;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::Model;

use crate::errors::ApiError;
use crate::lib::id::ID;
use crate::resource::model::Resource;
use crate::resource::model::ResourceCreate;
use crate::resource::model::ResourceUpdate;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("")
            .route(web::get().to(get_resources))
            .route(web::post().to(create_resource)),
    );
    cfg.service(
        web::resource("/{id}")
            .route(web::get().to(get_resource_by_id))
            .route(web::put().to(update_resource))
            .route(web::delete().to(remove_resource)),
    );
}

async fn get_resource_by_id(ctx: web::Data<Context>, id: ID) -> Response {
    let resource = Resource::find_one(&ctx.database.conn, doc! { "_id": id.0 }, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning resource to the client");
    let res = HttpResponse::Ok().json(resource);
    Ok(res)
}

async fn get_resources(ctx: web::Data<Context>) -> Response {
    let resources = Resource::find(&ctx.database.conn, None, None)
        .await
        .map_err(ApiError::WitherError)?
        .try_collect::<Vec<Resource>>()
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning resources to the client");
    let res = HttpResponse::Ok().json(resources);
    Ok(res)
}

async fn create_resource(ctx: web::Data<Context>, body: web::Json<ResourceCreate>) -> Response {
    let mut resource = Resource::new(body.into_inner());

    resource
        .save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning created resource to the client");
    let res = HttpResponse::Created().json(resource);
    Ok(res)
}

async fn update_resource(
    ctx: web::Data<Context>,
    id: ID,
    body: web::Json<ResourceUpdate>,
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
        doc! { "_id": id.0 },
        update,
        update_options,
    )
    .await
    .map_err(ApiError::WitherError)?;

    debug!("Returning updated resource to the client");
    let res = HttpResponse::Ok().json(resource);
    Ok(res)
}

async fn remove_resource(ctx: web::Data<Context>, id: ID) -> Response {
    let resource = Resource::find_one_and_delete(&ctx.database.conn, doc! { "_id": id.0 }, None)
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
