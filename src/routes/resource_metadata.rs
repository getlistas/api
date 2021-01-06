use actix_web::{web, HttpResponse};
use serde::Deserialize;
use url::Url;

use crate::errors::ApiError;
use crate::lib::resource_metadata;

type Response = actix_web::Result<HttpResponse>;

#[derive(Deserialize)]
struct Body {
    url: String,
}

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/resource-metadata").route(web::post().to(get_resource_metadata)));
}

async fn get_resource_metadata(body: web::Json<Body>) -> Response {
    let url = Url::parse(body.url.as_str()).map_err(|_| ApiError::ParseRequestBody())?;
    let metadata = resource_metadata::get_website_ogp_metadata(&url).await;

    let metadata = match metadata {
        Ok(metadata) => metadata,
        Err(_) => {
            debug!("Can not resolve URL, returning 404 status code to the client");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    debug!("Returning metadata to the client");
    let res = HttpResponse::Ok().json(metadata);
    Ok(res)
}
