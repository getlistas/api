use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use wither::bson::{self, oid::ObjectId};
use wither::bson::{doc, Bson};

use crate::lib::pagination::Pagination;
use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;
use crate::lib::util::parse_query_string;
use crate::lib::util::to_object_id;
use crate::models::list;
use crate::models::Model as ModelTrait;
use crate::Context;

#[derive(Deserialize)]
pub struct Query {
  tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserResponse {
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  id: ObjectId,
  slug: String,
  name: String,
  avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListResponse {
  #[serde(serialize_with = "serialize_object_id_as_hex_string")]
  id: ObjectId,
  title: String,
  description: Option<String>,
  tags: Option<Vec<String>>,
  #[serde(serialize_with = "serialize_bson_datetime_as_iso_string")]
  created_at: bson::DateTime,
  slug: String,
  user: UserResponse,
}

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("/discover").route(web::get().to(discover_lists)));
}

async fn discover_lists(
  req: HttpRequest,
  ctx: web::Data<Context>,
  pagination: web::Query<Pagination>,
) -> Response {
  let skip = pagination.skip.unwrap_or(0);
  let limit = pagination.limit.unwrap_or(100);
  let query_string = parse_query_string::<Query>(&req.query_string())?;

  let mut query = doc! { "is_public": true };
  if let Some(tags) = query_string.tags {
    query.insert("tags", doc! { "$in": tags });
  }

  // Once we have more lists from more users in the database, we can start
  // showing them.
  query.insert(
    "_id",
    doc! {
      "$in": [
        Bson::ObjectId(to_object_id(String::from("60bfa87c009045b70098149f"))?), // The Missing Semester of Your CS Education (MIT)
        Bson::ObjectId(to_object_id(String::from("6045e7da002ef30000e83201"))?), // A Computer of One’s Own
      ]
    },
  );

  let pipeline = list::queries::create_discover_query(query, skip, limit);
  let res = ctx.models.list.aggregate::<ListResponse>(pipeline).await?;

  debug!("Returning lists to the client");
  let res = HttpResponse::Ok().json(res);
  Ok(res)
}
