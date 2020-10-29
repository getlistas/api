use actix_web::{web, HttpResponse};
use futures::stream::StreamExt;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::errors::ApiError;
use crate::resource::model::Resource;
use crate::Context;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(get_resources)));
    cfg.service(web::resource("/{id}").route(web::get().to(get_resource_by_id)));
}

async fn get_resource_by_id(
    ctx: web::Data<Context>,
    id: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let id = ObjectId::with_string(id.as_str()).unwrap();
    let query = doc! { "_id": id };
    let post = Resource::find_one(&ctx.database.conn, query, None)
        .await
        .map_err(|err| ApiError::WitherError(err))?;

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
