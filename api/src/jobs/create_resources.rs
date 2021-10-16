use lapin::message::DeliveryResult;
use lapin::options::BasicAckOptions;
use lapin::options::BasicConsumeOptions;
use lapin::options::QueueDeclareOptions;
use lapin::types::FieldTable;
use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::lib::date;
use crate::lib::util::parse_url;
use crate::lib::util::to_object_id;
use crate::models::list::List;
use crate::models::resource::Resource;
use crate::models::Model;
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

          create_resources(payload, models)
            .await
            .expect("Create resources succesfully");

          delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("Failed to ack");
        }
      }
    })
    .unwrap();
}

async fn create_resources(payload: JobPayload, models: Models) -> Result<(), Error> {
  let list_id = payload.list;
  let list_id = to_object_id(list_id)?;
  let urls = payload.urls;

  let list = match models.list.find_by_id(&list_id).await? {
    Some(list) => list,
    None => {
      error!("List {:?} not found", &list_id);
      return Ok(());
    }
  };

  let position = models.list.get_position_for_new_resource(&list_id).await?;

  // TODO: Process this in parallel.
  for (index, url) in urls.into_iter().enumerate() {
    let position = position + (index + 1) as i32;
    create_resource(models.clone(), &list, url, position).await?
  }

  Ok(())
}

async fn create_resource(
  models: Models,
  list: &List,
  url: String,
  position: i32,
) -> Result<(), Error> {
  debug!(
    "Creating resource from url {:?} to list {:?}",
    &url, &list.id
  );

  let url = parse_url(&url)?;
  let resource = Resource {
    id: None,
    url: url.to_string(),
    position: position,
    user: list.user.clone(),
    list: list.id.clone().unwrap(),
    created_at: date::now(),
    updated_at: date::now(),
    title: None,
    description: None,
    thumbnail: None,
    tags: vec![],
    html: None,
    text: None,
    author: None,
    length: None,
    publisher: None,
    completed_at: None,
  };

  let resource_id = resource.id.clone().unwrap();
  models.resource.build(resource).await?;
  models.resource.populate(resource_id).await?;

  Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JobPayload {
  pub list: String,
  pub urls: Vec<String>,
}
