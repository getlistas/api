use config::{Config, ConfigError};
use serde::Deserialize;
use std::{env, fmt};

#[derive(Debug, Clone, Deserialize)]
pub struct Rss {
  pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sendgrid {
  pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthSettings {
  pub client_id: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Server {
  pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Logger {
  pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Database {
  pub uri: String,
  pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Auth {
  pub secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuth {
  pub google: OAuthSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mailer {
  pub from: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Traer {
  pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RabbitMQ {
  pub uri: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
  pub environment: String,
  pub server: Server,
  pub logger: Logger,
  pub database: Database,
  pub auth: Auth,
  pub oauth: OAuth,
  pub mailer: Mailer,
  pub base_url: String,
  pub client_url: String,
  pub rss: Rss,
  pub sendgrid: Sendgrid,
  pub traer: Traer,
  pub rabbit_mq: RabbitMQ,
}

impl Settings {
  pub fn new() -> Result<Self, ConfigError> {
    let mut settings = Config::new();
    let env = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

    settings.merge(config::File::with_name("config/default"))?;
    settings.merge(config::File::with_name(&format!("config/{}", env)).required(false))?;
    settings.merge(config::File::with_name("config/local").required(false))?;

    // Merge config from the environment variables.
    // Eg: `LOGGER__LEVEL=info ./target/listas` would set the `logger.level`
    // value.
    settings.merge(config::Environment::new().separator("__"))?;

    // Some cloud services like Heroku expose a randomly assigned port in
    // the PORT env var and there is no way to change the env var name.
    if env::var("PORT").is_ok() {
      let port = env::var("PORT").unwrap();
      settings
        .set("server.port", port)
        .expect("Failed to set PORT env var to settings");
    }

    settings.try_into()
  }
}

impl fmt::Display for Server {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "http://localhost:{}", &self.port)
  }
}
