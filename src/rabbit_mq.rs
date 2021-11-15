use lapin::Channel;
use lapin::Connection;
use lapin::ConnectionProperties;
use lapin::Error as LapinError;
use std::sync::Arc;
use tokio_amqp::*;

use crate::settings::Settings;

#[derive(Clone)]
pub struct RabbitMQ {
  pub channel: Arc<Channel>,
}

impl RabbitMQ {
  pub async fn new(settings: &Settings) -> Result<Self, LapinError> {
    let uri = settings.rabbit_mq.uri.as_str();
    let conn = Connection::connect(uri, ConnectionProperties::default().with_tokio()).await?;
    let channel = conn.create_channel().await?;
    let channel = Arc::new(channel);

    Ok(Self { channel })
  }
}
