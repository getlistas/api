use crate::actors::Actors;
use crate::database::Database;
use crate::integrations::rss::RSS;
use crate::mailer::Mailer;
use crate::models::Models;
use crate::settings::Settings;

#[derive(Clone)]
pub struct Context {
  pub database: Database,
  pub mailer: Mailer,
  pub settings: Settings,
  pub rss: RSS,
  pub models: Models,
  pub actors: Actors,
}
