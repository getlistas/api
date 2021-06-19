pub mod resource;
pub mod subscription;

use actix::{Actor, Addr};

use crate::mailer::Mailer;
use crate::models::Models;
use crate::settings::Settings;
use resource::ResourceActor;

#[derive(Clone)]
pub struct Actors {
  pub subscription: Addr<subscription::Actor>,
  pub resource: Addr<ResourceActor>,
}

impl Actors {
  pub fn new(models: Models, settings: Settings, mailer: Mailer) -> Self {
    let subscription = subscription::Actor {
      models: models.clone(),
      settings,
      mailer,
    };

    let resource = ResourceActor { models };

    Self {
      subscription: subscription.start(),
      resource: resource.start(),
    }
  }
}
