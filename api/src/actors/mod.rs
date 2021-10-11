pub mod subscription;

use actix::{Actor, Addr};

use crate::mailer::Mailer;
use crate::models::Models;
use crate::settings::Settings;

#[derive(Clone)]
pub struct Actors {
  pub subscription: Addr<subscription::Actor>,
}

impl Actors {
  pub fn new(models: Models, settings: Settings, mailer: Mailer) -> Self {
    let subscription = subscription::Actor {
      models,
      settings,
      mailer,
    };

    Self {
      subscription: subscription.start(),
    }
  }
}
