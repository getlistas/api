use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use wither::bson::doc;
use wither::bson::{self, oid::ObjectId};

use crate::lib::pagination::Pagination;
use crate::lib::serde::serialize_bson_datetime_as_iso_string;
use crate::lib::serde::serialize_object_id_as_hex_string;
use crate::lib::util::parse_query_string;
use crate::models::list::List;
use crate::Context;

#[derive(Deserialize)]
pub struct Query {
  tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserResponse {
  #[serde(serialize_with = "serialize_object_id_as_hex_string ")]
  id: ObjectId,
  slug: String,
  name: String,
  avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListResponse {
  #[serde(serialize_with = "serialize_object_id_as_hex_string ")]
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
  let query = parse_query_string::<Query>(&req.query_string())?;

  let mut list_match = doc! { "is_public": true };
  if let Some(tags) = query.tags {
    list_match.insert("tags", doc! { "$in": tags });
  }

  let pipeline = vec![
    doc! { "$match": list_match },
    doc! {
      "$lookup": {
        "from":"resources",
        "as": "resources",
        "let": {
          "list": "$_id"
        },
        "pipeline": vec![
          doc! {
            "$match": {
              "$expr": {
                "$eq": [ "$list",  "$$list" ]
              }
            }
          },
          doc! {
            "$sort": {
              "created_at": -1
            }
          },
          doc! { "$limit": 1 }
        ]
      }
    },
    doc! {
      "$match": {
        "resources": { "$ne": [] }
      }
    },
    doc! {
      "$sort": {
        "created_at": -1
      }
    },
    doc! { "$skip":  skip },
    doc! { "$limit": limit },
    doc! {
      "$lookup": {
        "from":"users",
        "localField": "user",
        "foreignField": "_id",
        "as": "user",
      }
    },
    doc! { "$unwind": "$user" },
    doc! {
      "$project": {
        "_id": false,
        "id": "$_id",
        "title": "$title",
        "description": "$description",
        "tags": "$tags",
        "created_at": "$created_at",
        "slug": "$slug",
        "user": {
          "id": "$user._id",
          "slug": "$user.slug",
          "name": "$user.name",
          "avatar": "$user.avatar",
        }
      }
    },
  ];

  let res = ctx.models.aggregate::<List, ListResponse>(pipeline).await?;

  debug!("Returning lists to the client");
  let res = HttpResponse::Ok().json(res);
  Ok(res)
}
