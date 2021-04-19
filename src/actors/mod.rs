pub mod subscription;

use actix::{Actor, Addr};

use crate::models::Models;

#[derive(Clone)]
pub struct Actors {
  pub subscription: Addr<subscription::Actor>,
}

impl Actors {
  pub fn new(models: Models) -> Self {
    let subscription = subscription::Actor {
      models: models.clone(),
    };

    let subscription_addr = subscription.start();

    Self {
      subscription: subscription_addr,
    }
  }
}
