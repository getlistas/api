pub mod on_list_removed;
pub mod on_resource_created;

use actix::Actor as ActixActor;
use actix::Context as ActixContext;

use crate::mailer::Mailer;
use crate::models::Models;
use crate::settings::Settings;

#[derive(Clone)]
pub struct Actor {
  pub models: Models,
  pub settings: Settings,
  pub mailer: Mailer,
}

impl ActixActor for Actor {
  type Context = ActixContext<Self>;

  fn started(&mut self, _ctx: &mut ActixContext<Self>) {
    info!("Subscription actor started");
  }

  fn stopped(&mut self, _ctx: &mut ActixContext<Self>) {
    info!("Subscription actor stopped");
  }
}
