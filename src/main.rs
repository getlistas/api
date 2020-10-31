use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
#[macro_use]
extern crate log;

mod components;
mod context;
mod database;
mod errors;
mod logger;
mod settings;

use components::resource;

use context::Context;
use database::Database;
use logger::Logger;
use settings::Settings;

#[actix_web::main]
async fn main() {
    let settings = match Settings::new() {
        Ok(value) => value,
        Err(err) => panic!("Failed to setup configuration. Error: {}", err),
    };

    match Logger::new(&settings) {
        Ok(value) => value,
        Err(_) => panic!("Failed to setup logger"),
    };

    let database = match Database::new(&settings).await {
        Ok(value) => value,
        Err(_) => panic!("Failed to setup database connection"),
    };

    let context = web::Data::new(Context {
        database: database.clone(),
    });

    let port = settings.server.port;

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .app_data(settings.clone())
            .app_data(context.clone())
            .wrap(Cors::default().supports_credentials())
            .service(web::scope("/resources").configure(resource::route::create_router))
    })
    .bind(("0.0.0.0", port))
    .expect("Failed to bind server to specified port")
    .run()
    .await
    .expect("Failed to start server");
}
