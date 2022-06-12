use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::sync::Arc;
use url::Url;
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::lib::date;
use crate::lib::util::parse_url;
use crate::models::resource::Resource;

static APPLICATION: &str = "62a3dfc09b9f7bee4fd5fa66";
static ENDPOINT: &str = "62a553394a314dde29ceee6f";

#[derive(Clone)]
pub struct Rss {
  inner: Arc<RssInner>,
}

#[derive(Clone)]
struct RssInner {
  pub base_url: String,
  pub token: String,
  pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Webhook {
  pub application: String,
  pub subscription: String,
  pub endpoint: String,
  pub entries: Vec<Entry>,
  pub metadata: Json,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
  pub url: Option<String>,
  pub title: Option<String>,
  pub description: Option<String>,
  pub published_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SubscribeResponse {
  pub id: String,
  pub application: String,
  pub url: String,
  pub feed: String,
  pub endpoint: String,
  pub metadata: Option<Json>,
}

#[derive(Serialize, Deserialize)]
pub struct UnsuscribeResponse {}

#[derive(Serialize, Deserialize)]
pub struct ValidateResponse {
  feed_type: String,
}

impl Rss {
  pub fn new(token: String) -> Self {
    let inner = Arc::new(RssInner {
      base_url: "https://therssproject.herokuapp.com".to_string(),
      client: reqwest::Client::new(),
      token,
    });

    Self { inner }
  }

  pub async fn subscribe(&self, url: &Url) -> Result<SubscribeResponse, Error> {
    let base_url = &self.inner.base_url;
    let base_url = format!("{}/applications/{}/subscriptions", base_url, APPLICATION);

    let payload = CreateSubscriptionPayload {
      url: url.to_string(),
      endpoint: ENDPOINT.to_string(),
      metadata: Json::Null,
    };

    self
      .inner
      .client
      .post(&base_url)
      .header("Authorization", &self.inner.token)
      .json(&payload)
      .send()
      .await?
      .json::<SubscribeResponse>()
      .await
      .map_err(|err| Error::RSSIntegration(err.to_string()))
  }

  pub async fn unsuscribe(&self, subscription_id: &str) -> Result<(), Error> {
    let base_url = &self.inner.base_url;
    let base_url = format!(
      "{}/applications/{}/subscriptions/{}",
      base_url, APPLICATION, subscription_id
    );

    self
      .inner
      .client
      .delete(&base_url)
      .header("Authorization", &self.inner.token)
      .send()
      .await?
      .json::<UnsuscribeResponse>()
      .await
      .map_err(|err| Error::RSSIntegration(err.to_string()))?;

    Ok(())
  }

  pub async fn is_valid_url(&self, url: &Url) -> Result<bool, Error> {
    let base_url = &self.inner.base_url;
    let base_url = format!("{}/applications/{}/subscriptions", base_url, APPLICATION);

    let res = self
      .inner
      .client
      .get(&base_url)
      .header("Authorization", &self.inner.token)
      .query(&[("url", url.as_str())])
      .send()
      .await?
      .json::<ValidateResponse>()
      .await;

    match res {
      Ok(_) => Ok(true),
      Err(_) => Ok(false),
    }
  }
}

#[derive(Deserialize, Serialize)]
struct CreateSubscriptionPayload {
  url: String,
  endpoint: String,
  metadata: Json,
}

pub async fn create_resource_payload_from_entry(
  entry: Entry,
  user: &ObjectId,
  list: &ObjectId,
) -> Result<Resource, Error> {
  let now = date::now();
  let url = parse_url(entry.url.unwrap().as_str())?;

  let resource = Resource {
    id: None,
    user: user.clone(),
    list: list.clone(),
    // The position will be computed before inserting the resource into the
    // database.
    position: 0,
    tags: vec!["rss".to_owned()],
    url: url.to_string(),
    title: entry.title.clone(),
    description: entry.description,
    created_at: now,
    updated_at: now,
    thumbnail: None,
    completed_at: None,
    html: None,
    text: None,
    author: None,
    length: None,
    publisher: None,
    populated_at: None,
  };

  Ok(resource)
}
