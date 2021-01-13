use actix_web::{http, HttpResponse};
use inflector::Inflector;
use rand::Rng;

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
