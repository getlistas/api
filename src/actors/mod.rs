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
      models: models.clone(),
      settings: settings.clone(),
      mailer: mailer.clone(),
    };

    let subscription_addr = subscription.start();

    Self {
      subscription: subscription_addr,
    }
  }
}
