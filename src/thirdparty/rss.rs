use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use url::Url;
use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::lib::date;
use crate::lib::resource_metadata;
use crate::lib::util::parse_url;
use crate::models::resource::Resource;

// https://rssapi.net/docs
#[derive(Clone)]
pub struct Rss {
  pub application_key: String,
  pub base_url: String,
  pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Webhook {
  pub webhook_reason: String,
  pub subscription_id: String,
  pub info: Option<String>,
  pub new_entries_count: usize,
  pub new_entries: Vec<Entry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
  pub title: String,
  pub link: String,
  pub description: String,
  pub time: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetResponse {
  entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
pub struct SubscribeResponse {
  pub status: String,
  pub subscription_id: String,
  pub feed_type: String,
  pub webhook_url: String,
  pub url: String,
  pub info: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UnsuscribeResponse {}

#[derive(Serialize, Deserialize)]
pub struct ValidateResponse {
  valid_feed: bool,
  feed_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response<T> {
  ok: bool,
  // RSS API returns a human readable error message when ok is false. It is safe
  // to unwrap error and result based on the ok value.
  error: Option<String>,
  result: Option<T>,
}

impl Rss {
  pub fn new(application_key: String) -> Self {
    Self {
      base_url: format!("https://api.rssapi.net/v1/{}", application_key),
      application_key,
      client: reqwest::Client::new(),
    }
  }

  pub async fn get_entries(&self, url: &Url) -> Result<Vec<Entry>, Error> {
    let limit_qs = ("limit", "10");
    let sort_qs = ("sort", "asc");
    let url_qs = ("url", url.as_str());

    let res = self
      .client
      .get(format!("{}/get", self.base_url).as_str())
      .query(&[url_qs, limit_qs, sort_qs])
      .send()
      .await?
      .json::<Response<GetResponse>>()
      .await
      .map_err(Error::ReqwestError)?;

    match res.ok {
      true => Ok(res.result.unwrap().entries),
      false => Err(Error::RSSIntegration(res.error.unwrap())),
    }
  }

  pub async fn subscribe(&self, url: &Url) -> Result<SubscribeResponse, Error> {
    let url_qs = ("url", url.as_str());

    let res = self
      .client
      .get(format!("{}/subscribe", self.base_url).as_str())
      .query(&[url_qs])
      .send()
      .await?
      .json::<Response<SubscribeResponse>>()
      .await
      .map_err(Error::ReqwestError)?;

    match res.ok {
      true => Ok(res.result.unwrap()),
      false => return Err(Error::RSSIntegration(res.error.unwrap())),
    }
  }

  pub async fn unsuscribe(&self, subscription_id: &str) -> Result<(), Error> {
    let res = self
      .client
      .get(format!("{}/removeSubscription", self.base_url).as_str())
      .query(&[("id", subscription_id)])
      .send()
      .await?
      .json::<Response<UnsuscribeResponse>>()
      .await
      .map_err(Error::ReqwestError)?;

    match res.ok {
      true => Ok(()),
      false => return Err(Error::RSSIntegration(res.error.unwrap())),
    }
  }

  pub async fn is_valid_url(&self, url: &Url) -> Result<bool, Error> {
    let res = self
      .client
      .get(format!("{}/validate", self.base_url).as_str())
      .query(&[("url", url.as_str())])
      .send()
      .await?
      .json::<Response<ValidateResponse>>()
      .await
      .map_err(Error::ReqwestError)?;

    match res.ok {
      true => Ok(res.result.unwrap().valid_feed),
      false => return Err(Error::RSSIntegration(res.error.unwrap())),
    }
  }

  pub async fn create_resource_payload_from_entry(
    entry: Entry,
    user: &ObjectId,
    list: &ObjectId,
  ) -> Result<Resource, Error> {
    let now = date::now();
    let url = parse_url(entry.link.as_str())?;
    let metadata = resource_metadata::get_website_metadata(&url).await?;

    let description = if entry.description.is_empty() {
      metadata.description
    } else {
      Some(entry.description.clone())
    };

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
      description,
      thumbnail: metadata.thumbnail,
      created_at: now,
      updated_at: now,
      completed_at: None,
    };

    Ok(resource)
  }

  pub async fn create_resources_payload_from_feed(
    &self,
    url: &Url,
    user: &ObjectId,
    list: &ObjectId,
  ) -> Result<Vec<Resource>, Error> {
    let entries = self.get_entries(&url).await?;
    let resource_futures = entries
      .into_iter()
      .map(|entry| Self::create_resource_payload_from_entry(entry, &user, &list));

    try_join_all(resource_futures).await
  }
}
