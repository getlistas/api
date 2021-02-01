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

struct RSSPayload {}

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/rss").route(web::post().to(rss)));
}

async fn rss(ctx: web::Data<Context>, pagination: web::Query<Pagination>) -> Response {
    debug!("Returning lists to the client");
    let res = HttpResponse::Ok().finish();
    Ok(res)
}
