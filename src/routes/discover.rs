use actix_web::{web, HttpResponse};
use futures::stream::TryStreamExt;
use wither::bson::doc;
use wither::mongodb::options::FindOptions;
use wither::Model;

use crate::errors::ApiError;
use crate::lib::pagination::Pagination;
use crate::models::list::List;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/discover").route(web::get().to(discover_lists)));
}

async fn discover_lists(ctx: web::Data<Context>, pagination: web::Query<Pagination>) -> Response {
    let find_options = FindOptions::builder()
        .limit(pagination.limit)
        .skip(pagination.skip)
        .build();

    let lists = List::find(&ctx.database.conn, doc! {}, find_options)
        .await
        .map_err(ApiError::WitherError)?
        .try_collect::<Vec<List>>()
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning lists to the client");
    let res = HttpResponse::Ok().json(lists);
    Ok(res)
}