use crate::actors::Actors;
use crate::database::Database;
use crate::mailer::Mailer;
use crate::models::Models;
use crate::settings::Settings;
use crate::thirdparty::rss::Rss;

#[derive(Clone)]
pub struct Context {
  pub database: Database,
  pub mailer: Mailer,
  pub settings: Settings,
  pub rss: Rss,
  pub actors: Actors,
  pub models: Models,
}
