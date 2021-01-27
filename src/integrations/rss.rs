use url::Url;
use serde::{Serialize, Deserialize};

use crate::errors::ApiError as Error;

// https://rssapi.net/docs
pub struct RSS {
  pub application_key: String,
  pub base_url: String,
  pub client: reqwest::Client
}


#[derive(Serialize, Deserialize)]
pub struct GetResponse {

}
#[derive(Serialize, Deserialize)]
pub struct SubscribeResponse {

}

#[derive(Serialize, Deserialize)]
pub struct UnsuscribeResponse {

}

#[derive(Serialize, Deserialize)]
pub struct ValidateResponse {

}

impl RSS {
  pub fn new (application_key: String) -> Self {
    Self {
      base_url: format!("https://api.rssapi.net/v1/{}", application_key),
      application_key,
      client: reqwest::Client::new()
    }
  }

  pub async fn get(&self, url: Url) -> Result<GetResponse, Error> {
    let url_qs = ("url[]", url.as_str());

    let res = self.client
      .get(format!("{}/get", self.base_url).as_str())
      .query(&[url_qs])
      .send()
      .await?
      .json::<GetResponse>()
      .await
      .map_err(Error::SubscribeToRSS)?;

      Ok(res)
  }

  pub async fn subscribe(&self, url: Url) -> Result<SubscribeResponse, Error> {
    let url_qs = ("url", url.as_str());

    let res = self.client
      .get(format!("{}/subscribe", self.base_url).as_str())
      .query(&[url_qs])
      .send()
      .await?
      .json::<SubscribeResponse>()
      .await
      .map_err(Error::SubscribeToRSS)?;

      Ok(res)
  }

  pub async fn unsuscribe(&self, id: String) -> Result<UnsuscribeResponse, Error> {
    let res = self.client
      .get(format!("{}/removeSubscription", self.base_url).as_str())
      .query(&[("id", id.as_str())])
      .send()
      .await?
      .json::<UnsuscribeResponse>()
      .await
      .map_err(Error::SubscribeToRSS)?;

      Ok(res)
  }

  pub async fn validate (&self, url: Url) -> Result<ValidateResponse, Error> {
    let res = self.client
      .get(format!("{}/validate", self.base_url).as_str())
      .query(&[("url", url.as_str())])
      .send()
      .await?
      .json::<ValidateResponse>()
      .await
      .map_err(Error::SubscribeToRSS)?;

      Ok(res)
  }
}