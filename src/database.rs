use wither::mongodb;

use crate::settings::Settings;

#[derive(Clone)]
pub struct Database {
  pub conn: mongodb::Database,
}

impl Database {
  pub async fn new(settings: &Settings) -> Result<Self, mongodb::error::Error> {
    let db_uri = settings.database.uri.as_str();
    let db_name = settings.database.name.as_str();

    let mut client_options = mongodb::options::ClientOptions::parse(db_uri).await?;

    // Mongo Atlas current tier support 500 concurrent connections. Default value
    // is 100.
    client_options.max_pool_size = Some(150);

    let connection = mongodb::Client::with_options(client_options)?.database(db_name);

    Ok(Self { conn: connection })
  }
}
