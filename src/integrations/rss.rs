use serde::{Deserialize, Serialize};
use url::Url;

use crate::errors::ApiError as Error;

// https://rssapi.net/docs
#[derive(Clone)]
pub struct RSS {
    pub application_key: String,
    pub base_url: String,
    pub client: reqwest::Client,
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub title: String,
    pub link: String,
    pub description: String,
    pub time: String,
}

#[derive(Serialize, Deserialize)]
pub struct Entries {
    entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
pub struct GetResponse {
    ok: bool,
    error: Option<String>,
    result: Entries,
}
#[derive(Serialize, Deserialize)]
pub struct SubscribeResponse {}

#[derive(Serialize, Deserialize)]
pub struct UnsuscribeResponse {}

#[derive(Serialize, Deserialize)]
pub struct ValidateResponse {}

impl RSS {
    pub fn new(application_key: String) -> Self {
        Self {
            base_url: format!("https://api.rssapi.net/v1/{}", application_key),
            application_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_entries(&self, url: &Url) -> Result<Vec<Entry>, Error> {
        let url_qs = ("url[]", url.as_str());
        let limit_qs = ("limit", "10");
        let sort_qs = ("sort", "asc");

        let res = self
            .client
            .get(format!("{}/get", self.base_url).as_str())
            .query(&[url_qs, limit_qs, sort_qs])
            .send()
            .await?
            .json::<GetResponse>()
            .await
            .map_err(Error::SubscribeToRSS)?;

        Ok(res.result.entries)
    }

    pub async fn subscribe(&self, url: Url) -> Result<SubscribeResponse, Error> {
        let url_qs = ("url", url.as_str());

        let res = self
            .client
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
        let res = self
            .client
            .get(format!("{}/removeSubscription", self.base_url).as_str())
            .query(&[("id", id.as_str())])
            .send()
            .await?
            .json::<UnsuscribeResponse>()
            .await
            .map_err(Error::SubscribeToRSS)?;

        Ok(res)
    }

    pub async fn validate(&self, url: Url) -> Result<ValidateResponse, Error> {
        let res = self
            .client
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
