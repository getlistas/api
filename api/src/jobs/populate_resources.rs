use lapin::message::DeliveryResult;
use lapin::options::BasicAckOptions;
use lapin::options::BasicConsumeOptions;
use lapin::options::QueueDeclareOptions;
use lapin::types::FieldTable;

use crate::errors::Error;
use crate::lib::util::to_object_id;
use crate::models::Models;
use crate::rabbit_mq::RabbitMQ;

const QUEUE_NAME: &str = "populate_resources";

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
      let models = models.clone();
      let delivery = delivery.expect("Error caught in in consumer");
      async move {
        if let Some((_channel, delivery)) = delivery {
          let payload = delivery.data.clone();

          info!("Received message: {:?}", &payload);

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

async fn populate_resources(payload: Vec<u8>, models: Models) -> Result<(), Error> {
  let ids: Vec<String> = bincode::deserialize(payload.as_ref()).unwrap();
  // TODO: Handle resources in parallel
  info!("Populating resources {:?}", &ids);

  for id in ids {
    populate_resource(id, models.clone()).await.unwrap();
  }

  Ok(())
}

async fn populate_resource(resource_id: String, models: Models) -> Result<(), Error> {
  info!("Populating resource: {}", &resource_id);
  let id = to_object_id(resource_id).expect("Job to receive a valid resource ID");
  models.resource.populate(id).await.unwrap();

  Ok(())
}
