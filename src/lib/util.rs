use actix_web::{http, HttpResponse};
use inflector::Inflector;
use itertools::Itertools;
use rand::Rng;
use url;
use wither::bson::oid::ObjectId;

use crate::errors::ApiError;

type Response = actix_web::Result<HttpResponse>;

pub fn redirect_to(url: &str) -> Response {
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, url)
        .finish()
        .into_body())
}

pub fn create_random_string(size: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(size)
        .map(char::from)
        .collect()
}

pub fn to_slug_case(string: String) -> String {
    string.to_kebab_case()
}

pub fn to_object_id(id: String) -> Result<ObjectId, ApiError> {
    ObjectId::with_string(id.as_str()).map_err(ApiError::ParseObjectID)
}

pub fn parse_url(url: &str) -> Result<url::Url, ApiError> {
    let mut url = url::Url::parse(url).map_err(|_| ApiError::ParseURL())?;

    url.path_segments_mut()
        .map_err(|_| ApiError::ParseURL())?
        .pop_if_empty();

    Ok(url)
}

pub fn sanitize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.to_lowercase().trim().to_owned())
        .filter(|tag| tag.len() >= 1)
        .unique()
        .collect()
}
