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

#[derive(Serialize, Deserialize)]
pub struct TraerReadResponse {
  pub can_resolve_url: bool,
  pub result: TraerReadResultResponse,
}

#[derive(Serialize, Deserialize)]
pub struct TraerReadResultResponse {
  pub title: Option<String>,
  pub description: Option<String>,
  pub html: Option<String>,
  pub text: Option<String>,
  pub length: Option<i64>,
  pub website_name: Option<String>,
  pub image: Option<String>,
  pub logo: Option<String>,
}

impl Traer {
  pub fn new(token: String) -> Self {
    Self {
      base_url: format!("https://traer.herokuapp.com"),
      token,
      client: reqwest::Client::new(),
    }
  }

  pub async fn get_url_content(&self, url: &Url) -> Result<TraerReadResponse, Error> {
    let mut body = HashMap::new();
    body.insert("url".to_owned(), url.to_string());

    self
      .client
      .post(format!("{}/read", self.base_url).as_str())
      .json(&body)
      .header("Authentication", self.token.clone())
      .send()
      .await?
      .json::<TraerReadResponse>()
      .await
      .map_err(Error::ReqwestError)
  }
}
