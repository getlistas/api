use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
#[macro_use]
extern crate log;

mod auth;
mod context;
mod database;
mod emails;
mod errors;
mod lib;
mod logger;
mod mailer;
mod models;
mod routes;
mod settings;

use context::Context;
use database::Database;
use logger::Logger;
use mailer::Mailer;
use settings::Settings;

#[actix_web::main]
async fn main() {
    let settings = match Settings::new() {
        Ok(value) => value,
        Err(err) => panic!("Failed to setup configuration. Error: {}", err),
    };

    match Logger::setup(&settings) {
        Ok(value) => value,
        Err(_) => panic!("Failed to setup logger"),
    };

    let database = match Database::new(&settings).await {
        Ok(value) => value,
        Err(_) => panic!("Failed to setup database connection"),
    };

    let mailer = match Mailer::new() {
        Ok(value) => value,
        Err(_) => panic!("Failed to setup mailer"),
    };

    let context = web::Data::new(Context {
        database: database.clone(),
        mailer: mailer.clone(),
        settings: settings.clone(),
    });

    let port = settings.server.port;

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(Cors::permissive())
            .app_data(web::Data::new(settings.clone()))
            .app_data(context.clone())
            .service(actix_files::Files::new("/static", "."))
            .configure(routes::user::create_router)
            .configure(routes::resource::create_router)
            .configure(routes::list::create_router)
            .configure(routes::discover::create_router)
            .configure(routes::resource_metadata::create_router)
            .service(web::scope("/").configure(routes::index::create_router))
    })
    .bind(("0.0.0.0", port))
    .expect("Failed to bind server to specified port")
    .run()
    .await
    .expect("Failed to start server");
}
