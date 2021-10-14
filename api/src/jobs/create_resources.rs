use lapin::message::DeliveryResult;
use lapin::options::BasicAckOptions;
use lapin::options::BasicConsumeOptions;
use lapin::options::QueueDeclareOptions;
use lapin::types::FieldTable;
use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::lib::util::to_object_id;
use crate::models::Models;
use crate::rabbit_mq::RabbitMQ;

const QUEUE_NAME: &str = "create-resources";

pub async fn setup(rabbit_mq: RabbitMQ, models: Models) {
  let channel = rabbit_mq.channel;
  let _queue = channel
    .queue_declare(
      QUEUE_NAME,
      QueueDeclareOptions::default(),
      FieldTable::default(),
    )
    .await
    .unwrap();

  let consumer = channel
    .basic_consume(
      QUEUE_NAME,
      "api",
      BasicConsumeOptions::default(),
      FieldTable::default(),
    )
    .await
    .unwrap();

  let models = models.clone();
  consumer
    .set_delegate(move |delivery: DeliveryResult| {
      // TODO: Add more data to this log.
      info!("Processing create-resources job");

      let models = models.clone();
      let delivery = delivery.expect("Error caught in in consumer");

      async move {
        if let Some((_channel, delivery)) = delivery {
          let payload = delivery.data.clone();
          let payload: JobPayload = bincode::deserialize(payload.as_ref()).unwrap();

          populate_resources(payload, models)
            .await
            .expect("Populate resources succesfully");

          delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("Failed to ack");
        }
      }
    })
    .unwrap();
}

async fn populate_resources(payload: JobPayload, models: Models) -> Result<(), Error> {
  let list_id = payload.list;
  let list_id = to_object_id(list_id)?;
  let urls = payload.urls;

  // TODO: Process this in parallel.
  for url in urls {
    create_resource(&list_id, url, models.clone())
      .await
      .unwrap();
  }

  Ok(())
}

async fn create_resource(list_id: &ObjectId, url: String, models: Models) -> Result<(), Error> {
  info!(
    "Creating resource from url {:?} to list {:?}",
    &url, &list_id
  );

  // let id = to_object_id(resource_id).expect("Job to receive a valid resource ID");
  // models.resource.populate(id).await.unwrap();

  Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JobPayload {
  pub list: String,
  pub urls: Vec<String>,
}
