use actix_web::{web, HttpResponse};
use serde::Serialize;

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("").route(web::get().to(index)));
}

#[derive(Debug, Serialize)]
struct Status {
  status: String,
  version: String,
}

async fn index() -> Response {
  let status = String::from("OK");
  let version = String::from("0.1.0");

  let res = HttpResponse::Ok().json(Status { status, version });
  Ok(res)
}
