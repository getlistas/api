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
use crate::list::model::List;
use crate::list::model::ListCreate;
use crate::list::model::ListUpdate;
use crate::resource::model::Resource;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("")
            .route(web::get().to(get_lists))
            .route(web::post().to(create_list)),
    );
    cfg.service(
        web::resource("/{id}")
            .route(web::get().to(get_list_by_id))
            .route(web::put().to(update_list))
            .route(web::delete().to(remove_list)),
    );
}

async fn get_list_by_id(ctx: web::Data<Context>, id: ID) -> Response {
    let list = List::find_one(&ctx.database.conn, doc! { "_id": id.0 }, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning list to the client");
    let res = HttpResponse::Ok().json(list);
    Ok(res)
}

async fn get_lists(ctx: web::Data<Context>) -> Response {
    let lists = List::find(&ctx.database.conn, None, None)
        .await
        .map_err(|err| ApiError::WitherError(err))?
        .try_collect::<Vec<List>>()
        .await
        .map_err(|err| ApiError::WitherError(err))?;

    debug!("Returning lists to the client");
    let res = HttpResponse::Ok().json(lists);
    Ok(res)
}

async fn create_list(ctx: web::Data<Context>, body: web::Json<ListCreate>) -> Response {
    let mut list = List::new(body.into_inner());

    list.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning created list to the client");
    let res = HttpResponse::Created().json(list);
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

    debug!("Returning updated list to the client");
    let res = HttpResponse::Ok().json(list);
    Ok(res)
}

async fn remove_list(ctx: web::Data<Context>, id: ID) -> Response {
    let list = List::find_one(&ctx.database.conn, doc! { "_id": &id.0 }, None)
        .await
        .map_err(ApiError::WitherError)?;

    if list.is_none() {
        debug!("List not found, returning 404 status code to the client");
        return Ok(HttpResponse::NotFound().finish());
    }

    debug!("Removing resources associated to this list");
    Resource::collection(&ctx.database.conn)
        .delete_many(doc! { "list": &id.0 }, None)
        .await
        .map_err(ApiError::MongoError)?;

    debug!("Removing list");
    list.unwrap()
        .delete(&ctx.database.conn)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("List removed, returning 204 status code to the client");
    let res = HttpResponse::NoContent().finish();

    Ok(res)
}