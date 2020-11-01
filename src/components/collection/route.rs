use actix_web::{web, HttpResponse};
use futures::stream::StreamExt;
use serde_json::json;
use wither::bson;
use wither::bson::doc;
use wither::mongodb;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::Model;

use crate::collection::model::Collection;
use crate::collection::model::CollectionCreate;
use crate::collection::model::CollectionUpdate;
use crate::errors::ApiError;
use crate::lib::id::ID;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("")
            .route(web::get().to(get_collections))
            .route(web::post().to(create_collection)),
    );
    cfg.service(
        web::resource("/{id}")
            .route(web::get().to(get_collection_by_id))
            .route(web::put().to(update_collection))
            .route(web::delete().to(remove_collection)),
    );
}

async fn get_collection_by_id(ctx: web::Data<Context>, id: ID) -> Response {
    let collection = Collection::find_one(&ctx.database.conn, doc! { "_id": id.0 }, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning collection to the client");
    let res = HttpResponse::Ok().json(collection);
    Ok(res)
}

async fn get_collections(ctx: web::Data<Context>) -> Response {
    let collections = Collection::find(&ctx.database.conn, None, None)
        .await
        .map_err(|err| ApiError::WitherError(err))?
        // TODO: Collect to a Result<Vec>
        .map(|post| post.unwrap())
        .collect::<Vec<Collection>>()
        .await;

    debug!("Returning collections to the client");
    let res = HttpResponse::Ok().json(collections);
    Ok(res)
}

async fn create_collection(ctx: web::Data<Context>, body: web::Json<CollectionCreate>) -> Response {
    let mut collection = Collection::new(body.into_inner());

    collection
        .save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning created collection to the client");
    let res = HttpResponse::Created().json(collection);
    Ok(res)
}

async fn update_collection(
    ctx: web::Data<Context>,
    id: ID,
    body: web::Json<CollectionUpdate>,
) -> Response {
    let mut body = body.into_inner();
    let body = CollectionUpdate::new(&mut body);
    let update = json!({ "$set": body });

    let update = bson::ser::to_document(&update).unwrap();
    let update_options = FindOneAndUpdateOptions::builder()
        .return_document(mongodb::options::ReturnDocument::After)
        .build();

    let collection = Collection::find_one_and_update(
        &ctx.database.conn,
        doc! { "_id": id.0 },
        update,
        update_options,
    )
    .await
    .map_err(ApiError::WitherError)?;

    debug!("Returning updated collection to the client");
    let res = HttpResponse::Ok().json(collection);
    Ok(res)
}

async fn remove_collection(ctx: web::Data<Context>, id: ID) -> Response {
    let collection =
        Collection::find_one_and_delete(&ctx.database.conn, doc! { "_id": id.0 }, None)
            .await
            .map_err(ApiError::WitherError)?;

    let res = match collection {
        Some(_) => {
            debug!("Collection removed, returning 204 status code to the client");
            HttpResponse::NoContent().finish()
        }
        None => {
            debug!("Collection not found, returning 404 status code to the client");
            HttpResponse::NotFound().finish()
        }
    };

    Ok(res)
}
