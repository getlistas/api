use actix_web::{http, HttpResponse};
use itertools::Itertools;
use rand::Rng;
use serde::de::Deserialize;
use slug::slugify;
use url;
use wither::bson::oid::ObjectId;

use crate::errors::Error;

type Response = actix_web::Result<HttpResponse>;

pub fn redirect_to(url: &str) -> Response {
  Ok(
    HttpResponse::Found()
      .header(http::header::LOCATION, url)
      .finish()
      .into_body(),
  )
}

pub fn create_random_string(size: usize) -> String {
  rand::thread_rng()
    .sample_iter(&rand::distributions::Alphanumeric)
    .take(size)
    .map(char::from)
    .collect()
}

// The slug will consist of a-z, 0-9, and '-'. Furthermore, a slug will never
// contain more than one '-' in a row and will never start or end with '-'.
pub fn to_slug_case(string: String) -> String {
  slugify(string)
}

pub fn to_object_id(id: String) -> Result<ObjectId, Error> {
  ObjectId::with_string(id.as_str()).map_err(Error::ParseObjectID)
}

pub fn parse_url(url: &str) -> Result<url::Url, Error> {
  let mut url = url.to_owned();
  // Removes the URL trailing slashes
  while url.ends_with('/') {
    url.pop();
  }

  let url = url::Url::parse(url.as_str()).map_err(|_| Error::ParseURL())?;
  Ok(url)
}

pub fn parse_query_string<'a, T>(query_string: &'a str) -> Result<T, Error>
where
  T: Deserialize<'a>,
{
  serde_qs::from_str::<T>(&query_string).map_err(Error::ParseQueryString)
}

pub fn sanitize_tags(tags: Vec<String>) -> Vec<String> {
  tags
    .into_iter()
    .map(|tag| tag.to_lowercase().trim().to_owned())
    .filter(|tag| tag.len() >= 1)
    .unique()
    .collect()
}
