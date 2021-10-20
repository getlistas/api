use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::{Deserialize, Serialize};
use validator::Validate;
use wither::bson::doc;

use crate::auth;
use crate::auth::UserID;
use crate::jobs::create_resources::JobPayload;
use crate::lib::util::to_object_id;
use crate::models::Model as ModelTrait;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(
    web::resource("/import-resources")
      .route(web::post().to(import_resources))
      .wrap(auth),
  );
}

#[derive(Serialize, Deserialize)]
struct RequestBody {
  list: String,
  payload: String,
}

async fn import_resources(ctx: Ctx, body: web::Json<RequestBody>, user: UserID) -> Response {
  let user_id = user.0;
  let list_id = to_object_id(body.list.clone())?;
  let payload = body.payload.clone();

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": list_id, "user": user_id }, None)
    .await?;

  let list = match list {
    Some(list) => list,
    None => return Ok(HttpResponse::NotFound().finish()),
  };

  let urls = parse_import_payload(payload);
  let payload = JobPayload {
    list: list.id.unwrap().to_string(),
    urls,
  };

  // TODO: Queue un batch.

  ctx.jobs.queue("create_resources", payload).await;

  debug!("Returning resource metadata to the client");
  let res = HttpResponse::Ok().finish();
  Ok(res)
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
struct ImportItem {
  #[validate(url)]
  url: String,
}

fn parse_import_payload(payload: String) -> Vec<String> {
  payload
    .trim()
    .lines()
    .filter(|line| !line.is_empty())
    .filter_map(|line| {
      let url = line.trim().split(' ').next()?;
      let import_item = ImportItem {
        url: url.to_string(),
      };

      match import_item.validate() {
        Ok(_) => Some(import_item.url),
        Err(_) => None,
      }
    })
    .collect::<Vec<String>>()
}
