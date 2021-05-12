use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::{Deserialize, Serialize};

use crate::models::user::UserID;
use crate::Context;
use crate::{auth, lib::util};
use crate::{
  lib::resource_metadata,
  models::{resource::PrivateResource, Model},
};
use wither::bson::doc;

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;

#[derive(Deserialize)]
struct Body {
  url: String,
}

#[derive(Debug, Serialize)]
pub struct ResourceMetadata {
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

  let website_metadata = resource_metadata::get_website_metadata(&url).await;

  let resource = ctx
    .models
    .resource
    .find_one(doc! { "user": &user_id, "url": url.as_str() }, None)
    .await?;

  let website_metadata = match website_metadata {
    Ok(metadata) => metadata,
    Err(_) => {
      debug!("Can not resolve url, returning metadata to the client");
      let metadata = ResourceMetadata {
        resource: resource.map(Into::into),
        can_resolve: false,
        title: None,
        description: None,
        thumbnail: None,
      };

      return Ok(HttpResponse::Ok().json(metadata));
    }
  };

  let metadata = ResourceMetadata {
    resource: resource.map(Into::into),
    can_resolve: true,
    title: website_metadata.title,
    description: website_metadata.description,
    thumbnail: website_metadata.thumbnail,
  };

  debug!("Returning metadata to the client");
  let res = HttpResponse::Ok().json(metadata);
  Ok(res)
}
