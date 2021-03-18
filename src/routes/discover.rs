use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use wither::bson::doc;
use wither::bson::Document;

use crate::lib::pagination::Pagination;
use crate::models::list::List;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoverListResponse {}

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("/discover").route(web::get().to(discover_lists)));
}

async fn discover_lists(ctx: web::Data<Context>, pagination: web::Query<Pagination>) -> Response {
  let skip = pagination.skip.unwrap_or(0);
  let limit = pagination.limit.unwrap_or(100);
  let pipeline = vec![
    doc! {
      "$match": {
        "is_public": true
      }
    },
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
          doc! { "$limit": 3 }
        ]
      }
    },
    doc! {
      "$match": {
        "resources": { "$ne": [] }
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
  ];

  let res = ctx.models.aggregate::<List, Document>(pipeline).await?;

  debug!("Returning lists to the client");
  let res = HttpResponse::Ok().json(res);
  Ok(res)
}
