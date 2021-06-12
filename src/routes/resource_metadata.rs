use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::{Deserialize, Serialize};
use wither::bson::doc;

use crate::auth;
use crate::auth::UserID;
use crate::lib::util;
use crate::models::resource::PrivateResource;
use crate::models::Model as ModelTrait;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;

#[derive(Deserialize)]
struct Body {
  url: String,
}

#[derive(Debug, Serialize)]
pub struct ResourceMetadataResponse {
  can_resolve: bool,
  resource: Option<PrivateResource>,
  title: Option<String>,
  description: Option<String>,
  thumbnail: Option<String>,
}

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(
    web::resource("/resource-metadata")
      .route(web::post().to(get_resource_metadata))
      .wrap(auth),
  );
}

async fn get_resource_metadata(ctx: Ctx, body: web::Json<Body>, user: UserID) -> Response {
  let user_id = user.0;
  let url = util::parse_url(body.url.as_str())?;

  let resource_metadata = ctx.models.resource.get_metadata(&url).await;

  let resource = ctx
    .models
    .resource
    .find_one(doc! { "user": &user_id, "url": url.as_str() }, None)
    .await?;

  let resource_metadata = match resource_metadata {
    Ok(Some(metadata)) => metadata,
    Err(_) | Ok(None) => {
      error!("Can not get resource metadata, returning empty metadata to the client");
      let metadata = ResourceMetadataResponse {
        resource: resource.map(Into::into),
        can_resolve: false,
        title: None,
        description: None,
        thumbnail: None,
      };

      return Ok(HttpResponse::Ok().json(metadata));
    }
  };

  let metadata = ResourceMetadataResponse {
    resource: resource.map(Into::into),
    can_resolve: true,
    title: resource_metadata.title,
    description: resource_metadata.description,
    thumbnail: resource_metadata.thumbnail,
  };

  debug!("Returning resource metadata to the client");
  let res = HttpResponse::Ok().json(metadata);
  Ok(res)
}
