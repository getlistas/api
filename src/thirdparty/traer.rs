use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

use crate::errors::Error;

#[derive(Clone)]
pub struct Traer {
  inner: Arc<TraerInner>,
}

#[derive(Clone)]
struct TraerInner {
  token: String,
  base_url: String,
  client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraerResponse<T> {
  pub success: bool,
  pub status: u16,
  pub data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraerReadResult {
  pub title: Option<String>,
  pub description: Option<String>,
  pub html: Option<String>,
  pub text: Option<String>,
  pub length: Option<i64>,
  pub author: Option<String>,
  pub publisher: Option<String>,
  pub image: Option<String>,
  pub logo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraerReadSomeResult {
  pub title: Option<String>,
  pub description: Option<String>,
  pub author: Option<String>,
  pub publisher: Option<String>,
  pub image: Option<String>,
  pub logo: Option<String>,
}

impl Traer {
  pub fn new(token: String) -> Self {
    let inner = Arc::new(TraerInner {
      base_url: "https://traer.vercel.app/api/v1".to_string(),
      token,
      client: reqwest::Client::new(),
    });

    Self { inner }
  }

  pub async fn get_content_from_url(&self, url: &Url) -> Result<Option<TraerReadResult>, Error> {
    let mut body = HashMap::new();
    body.insert("url".to_owned(), url.to_string());

    let res = self
      .inner
      .client
      .post(format!("{}/parse", self.inner.base_url).as_str())
      .json(&body)
      .header("Authentication", self.inner.token.clone())
      .send()
      .await?
      .json::<TraerResponse<TraerReadResult>>()
      .await
      .map_err(Error::Reqwest)?;

    let has_metadata = res.success;
    if !has_metadata {
      return Ok(None);
    }

    Ok(Some(res.data))
  }

  pub async fn get_slim_content_from_url(
    &self,
    url: &Url,
  ) -> Result<TraerResponse<TraerReadSomeResult>, Error> {
    let mut body = HashMap::new();
    body.insert("url".to_owned(), url.to_string());

    self
      .inner
      .client
      .post(format!("{}/parse", self.inner.base_url).as_str())
      .query(&[("slim", true)])
      .json(&body)
      .header("Authentication", self.inner.token.clone())
      .send()
      .await?
      .json::<TraerResponse<TraerReadSomeResult>>()
      .await
      .map_err(Error::Reqwest)
  }
}
