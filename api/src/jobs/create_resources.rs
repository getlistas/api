use futures::stream::StreamExt;
use lapin::message::DeliveryResult;
use lapin::options::BasicAckOptions;
use lapin::options::BasicConsumeOptions;
use lapin::options::BasicNackOptions;
use lapin::options::QueueDeclareOptions;
use lapin::types::FieldTable;
use serde::{Deserialize, Serialize};
use wither::bson::doc;

use crate::errors::Error;
use crate::lib::date;
use crate::lib::util::parse_url;
use crate::lib::util::to_object_id;
use crate::models::list::List;
use crate::models::resource::Resource;
use crate::models::Model;
use crate::models::Models;
use crate::rabbit_mq::RabbitMQ;

const QUEUE_NAME: &str = "create_resources";

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
      "",
      BasicConsumeOptions::default(),
      FieldTable::default(),
    )
    .await
    .unwrap();

  let models = models.clone();
  consumer
    .set_delegate(move |delivery: DeliveryResult| {
      // TODO: Add more data to this log.
      info!("Processing create_resources job");

      let models = models.clone();
      let delivery = delivery.expect("Error caught in consumer");

      async move {
        let delivery = match delivery {
          Some((_channel, delivery)) => delivery,
          None => return, // The consumer got canceled.
        };

        let payload = delivery.data.clone();
        let payload: JobPayload = bincode::deserialize(payload.as_ref()).unwrap();
        let result = create_resources(payload, models).await;

        match result {
          Ok(_) => delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("Failed to ack"),
          Err(err) => {
            error!("Failed to process the create_resources job. Error: {}", err);
            delivery
              .nack(BasicNackOptions::default())
              .await
              .expect("Failed to nack");
          }
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

  let resource_futures = urls.into_iter().enumerate().map(|(index, url)| {
    let position = position + (index + 1) as i32;
    let models = models.clone();
    let list = list.clone();

    async move {
      let result = create_resource(models.clone(), &list, url.clone(), position).await;
      // TODO: Improve this error handling, should we retry this URL?
      if let Err(err) = result {
        error!("Failed to create resource with URL {}. Error: {}", url, err);
      };
    }
  });

  futures::stream::iter(resource_futures)
    .buffer_unordered(50)
    .collect::<Vec<()>>()
    .await;

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
  let list_id = list.id.clone().unwrap();
  let resource_exists = models
    .resource
    .exists(doc! { "user": &list.user, "list": &list_id, "url": url.to_string() })
    .await?;

  if resource_exists {
    return Ok(());
  }

  let resource = Resource {
    id: None,
    url: url.to_string(),
    position,
    user: list.user.clone(),
    list: list_id,
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

  let resource = models.resource.build(resource).await?;
  let resource_id = resource.id.clone().unwrap();

  models.resource.populate(resource_id).await?;

  Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JobPayload {
  pub list: String,
  pub urls: Vec<String>,
}
