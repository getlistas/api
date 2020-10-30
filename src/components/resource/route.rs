use actix_web::{web, HttpResponse};
use futures::stream::StreamExt;
use serde_json::json;
use wither::bson;
use wither::bson::{doc, oid::ObjectId};
use wither::mongodb;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::Model;

use crate::errors::ApiError;
use crate::resource::model::Resource;
use crate::resource::model::ResourceUpdate;
use crate::Context;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("")
            .route(web::get().to(get_resources))
            .route(web::post().to(create_resource)),
    );
    cfg.service(
        web::resource("/{id}")
            .route(web::get().to(get_resource_by_id))
            .route(web::put().to(update_resource)),
    );
}

async fn get_resource_by_id(
    ctx: web::Data<Context>,
    id: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let id = ObjectId::with_string(id.as_str()).unwrap();
    let post = Resource::find_one(&ctx.database.conn, doc! { "_id": id }, None)
        .await
        .map_err(ApiError::WitherError)?;

    let res = HttpResponse::Ok().json(post);
    Ok(res)
}

async fn get_resources(ctx: web::Data<Context>) -> actix_web::Result<HttpResponse> {
    let posts = Resource::get(&ctx.database.conn)
        .await
        .map_err(|err| ApiError::WitherError(err))?
        // TODO: Collect to a Result<Vec>
        .map(|post| post.unwrap())
        .collect::<Vec<Resource>>()
        .await;

    debug!("Returning posts to the client");
    let res = HttpResponse::Ok().json(posts);
    Ok(res)
}

async fn create_resource(
    ctx: web::Data<Context>,
    body: web::Json<Resource>,
) -> actix_web::Result<HttpResponse> {
    let mut post = Resource {
        id: None,
        url: body.url.clone(),
        title: body.title.clone(),
        description: body.description.clone(),
    };

    post.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning created post to the client");
    let res = HttpResponse::Created().json(post);
    Ok(res)
}

async fn update_resource(
    ctx: web::Data<Context>,
    id: web::Path<String>,
    body: web::Json<ResourceUpdate>,
) -> actix_web::Result<HttpResponse> {
    let id = ObjectId::with_string(id.as_str()).unwrap();
    let body = body.into_inner();
    let update = json!({ "$set": body });
    let update = bson::ser::to_document(&update).unwrap();
    let update_options = FindOneAndUpdateOptions::builder()
        .return_document(mongodb::options::ReturnDocument::After)
        .build();

    let post = Resource::find_one_and_update(
        &ctx.database.conn,
        doc! { "_id": id },
        update,
        update_options,
    )
    .await
    .map_err(ApiError::WitherError)?;

    debug!("Returning updated post to the client");
    let res = HttpResponse::Ok().json(post);
    Ok(res)
}
