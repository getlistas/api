use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::Deserialize;
use wither::bson::doc;

use crate::auth::UserID;
use crate::lib::id::ID;
use crate::lib::util::to_object_id;
use crate::models::like::{Like, PublicLike};
use crate::models::Model as ModelTrait;
use crate::Context;
use crate::{auth, lib::date};

#[derive(Deserialize)]
pub struct LikeCreate {
  pub list: String,
}

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;
type LikeCreateBody = web::Json<LikeCreate>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(
    web::resource("/likes/{id}")
      .route(web::delete().to(remove_like))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/likes")
      .route(web::get().to(query_likes))
      .route(web::post().to(create_like))
      .wrap(auth),
  );
}

async fn query_likes(ctx: Ctx, user_id: UserID) -> Response {
  let user_id = user_id.0;
  let likes = ctx
    .models
    .like
    .find(doc! { "user": user_id }, None)
    .await?
    .into_iter()
    .map(Into::into)
    .collect::<Vec<PublicLike>>();

  debug!("Returning likes");
  let res = HttpResponse::Ok().json(likes);
  Ok(res)
}

async fn create_like(ctx: Ctx, body: LikeCreateBody, user_id: UserID) -> Response {
  let list_id = to_object_id(body.list.clone())?;
  let user_id = user_id.0;

  let like = ctx
    .models
    .like
    .find_one(doc! { "_id": &list_id,"user": &user_id }, None)
    .await?;

  if like.is_some() {
    debug!("Can not create a duplicate Like");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let like = Like {
    id: None,
    user: user_id,
    list: list_id,
    created_at: date::now(),
  };
  let like = ctx.models.like.create(like).await?;

  debug!("Returning created like");
  let like: PublicLike = like.into();
  let res = HttpResponse::Created().json(like);
  Ok(res)
}

async fn remove_like(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let like_id = id.0;
  let user_id = user_id.0;

  let result = ctx
    .models
    .like
    .delete_one(doc! { "_id": like_id, "user": user_id })
    .await?;

  let res = match result.deleted_count {
    0 => {
      debug!("Like not found, returning 404 status code");
      HttpResponse::NotFound().finish()
    }
    _ => {
      debug!("Like removed, returning 204 status code");
      HttpResponse::NoContent().finish()
    }
  };

  Ok(res)
}
