use actix_files::NamedFile;
use actix_web::web;
use std::path::PathBuf;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("").route(web::get().to(index)));
}

async fn index() -> actix_web::Result<NamedFile> {
  let path = PathBuf::from("static/index.html");
  let html = NamedFile::open(path)?;

  debug!("Returning index.html to the client");
  Ok(html)
}
