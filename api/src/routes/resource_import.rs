use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::{Deserialize, Serialize};
use validator::Validate;
use wither::bson::doc;

use crate::models::Model as ModelTrait;
use crate::lib::util::to_object_id;
use crate::auth;
use crate::auth::UserID;
use crate::models::resource::PrivateResource;
use crate::Context;
use crate::jobs::create_resources::CreateResources;

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
    web::resource("/import-resources")
      .route(web::post().to(import_from_onetab))
      .wrap(auth.clone()),
  );
}

#[derive(Serialize, Deserialize)]
struct RequestBody {
  list: String,
  payload: String,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
struct ImportResourceItem {
  #[validate(url)]
  url: String,
}

async fn import_from_onetab(ctx: Ctx, body: web::Json<RequestBody>, user: UserID) -> Response {
  let user_id = user.0;
  let list_id = to_object_id(body.list.clone())?;
  let payload = body.payload.clone();

  let list = ctx.models.list.find_by_id(&list_id).await?;
  let list = match list {
    Some(list) => list,
    None => return Ok(HttpResponse::NotFound().finish()),
  };
  
  let urls = parse_import_payload(payload);
  let payload = CreateResources {
    list: body.list.clone(),
    resources: urls
  };
  // TODO: Queue un batch.
  ctx.jobs.queue("create-resources", payload).await;

  debug!("Returning resource metadata to the client");
  let res = HttpResponse::Ok().finish();
  Ok(res)
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
struct ImportIten {
  #[validate(url)]
  url: String,
}

fn parse_import_payload(payload: String) -> Vec<String> {
  payload
    .trim()
    .lines()
    .filter(|line| !line.is_empty())
    .filter_map(|line| {
      let url = line
        .trim()
        .split(' ')
        .nth(0)?;

      let import_item = ImportIten { url: url.to_string() };
      match import_item.validate() {
        Ok(_) => Some(import_item.url),
        Err(_) => None,
      }
    })
    .collect::<Vec<String>>()
}
