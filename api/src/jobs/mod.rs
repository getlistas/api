pub mod populate_resources;
pub mod create_resources;

use crate::models::Models;
use crate::rabbit_mq::RabbitMQ;
use lapin::options::BasicPublishOptions;
use lapin::BasicProperties;
use serde::Serialize;

#[derive(Clone)]
pub struct Jobs {
  rabbit_mq: RabbitMQ,
}

impl Jobs {
  pub async fn setup(rabbit_mq: RabbitMQ, models: Models) -> Self {
    populate_resources::setup(rabbit_mq.clone(), models.clone()).await;
    create_resources::setup(rabbit_mq.clone(), models.clone()).await;

    Self { rabbit_mq }
  }

  pub async fn queue<T>(&self, queue_name: &str, payload: T)
  where
    T: Serialize,
  {
    let channel = self.rabbit_mq.channel.clone();
    let payload: Vec<u8> = bincode::serialize(&payload).unwrap();

    let _confirm = channel
      .basic_publish(
        "",
        queue_name,
        BasicPublishOptions::default(),
        payload,
        BasicProperties::default(),
      )
      .await
      .unwrap()
      .await
      .unwrap();
  }
}
