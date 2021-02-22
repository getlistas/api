use actix_web::{http, HttpResponse};
use itertools::Itertools;
use rand::Rng;
use url;
use wither::bson::oid::ObjectId;
use slug::slugify;

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
  let mut url = url::Url::parse(url).map_err(|_| Error::ParseURL())?;

  // Remove the URL trailing slash
  url
    .path_segments_mut()
    .map_err(|_| Error::ParseURL())?
    .pop_if_empty();

  Ok(url)
}

pub fn sanitize_tags(tags: Vec<String>) -> Vec<String> {
  tags
    .into_iter()
    .map(|tag| tag.to_lowercase().trim().to_owned())
    .filter(|tag| tag.len() >= 1)
    .unique()
    .collect()
}
