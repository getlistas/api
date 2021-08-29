use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::errors::Error;

#[derive(Clone)]
pub struct Traer {
  pub token: String,
  pub base_url: String,
  pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraerResponse<T> {
  pub can_resolve_url: bool,
  pub result: T,
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
    Self {
      base_url: "https://traer.vercel.app/api/v1".to_string(),
      token,
      client: reqwest::Client::new(),
    }
  }

  pub async fn get_content_from_url(
    &self,
    url: &Url,
  ) -> Result<TraerResponse<TraerReadResult>, Error> {
    let mut body = HashMap::new();
    body.insert("url".to_owned(), url.to_string());

    self
      .client
      .post(format!("{}/parse", self.base_url).as_str())
      .json(&body)
      .header("Authentication", self.token.clone())
      .send()
      .await?
      .json::<TraerResponse<TraerReadResult>>()
      .await
      .map_err(Error::ReqwestError)
  }

  pub async fn get_some_content_from_url(
    &self,
    url: &Url,
  ) -> Result<TraerResponse<TraerReadSomeResult>, Error> {
    let mut body = HashMap::new();
    body.insert("url".to_owned(), url.to_string());

    self
      .client
      .post(format!("{}/read/some", self.base_url).as_str())
      .json(&body)
      .header("Authentication", self.token.clone())
      .send()
      .await?
      .json::<TraerResponse<TraerReadSomeResult>>()
      .await
      .map_err(Error::ReqwestError)
  }
}
