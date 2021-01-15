use actix_web::{web, HttpResponse};
use futures::stream::TryStreamExt;
use serde::Deserialize;
use wither::bson::doc;
use wither::Model;

use crate::errors::ApiError;
use crate::models::list::List;
use crate::models::user::User;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

#[derive(Deserialize)]
struct Params {
    user_slug: String,
    list_slug: Option<String>,
}

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("users/{user_slug}/lists").route(web::get().to(get_lists_by_slug)));

    cfg.service(
        web::resource("users/{user_slug}/lists/{list_slug}").route(web::get().to(get_list_by_slug)),
    );
}

async fn get_lists_by_slug(ctx: web::Data<Context>, params: web::Path<Params>) -> Response {
    let user = User::find_one(&ctx.database.conn, doc! { "slug": &params.user_slug }, None)
        .await
        .map_err(ApiError::WitherError)?;

    let user = match user {
        Some(user) => user,
        None => {
            debug!("User not found for slug, returning 404 status code to the user");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    let lists = List::find(
        &ctx.database.conn,
        doc! {
            "user": user.id.unwrap(),
            "is_public": true
        },
        None,
    )
    .await
    .map_err(ApiError::WitherError)?
    .try_collect::<Vec<List>>()
    .await
    .map_err(ApiError::WitherError)?;

    let lists = lists
        .iter()
        .map(|list| list.to_json())
        .collect::<Vec<serde_json::Value>>();

    debug!("Returning list to the user");
    let res = HttpResponse::Ok().json(lists);
    Ok(res)
}

async fn get_list_by_slug(ctx: web::Data<Context>, params: web::Path<Params>) -> Response {
    let list_slug = params.list_slug.clone().unwrap();
    let user = User::find_one(&ctx.database.conn, doc! { "slug": &params.user_slug }, None)
        .await
        .map_err(ApiError::WitherError)?;

    let user = match user {
        Some(user) => user,
        None => {
            debug!("User not found for slug, returning 404 status code to the user");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    let list = List::find_one(
        &ctx.database.conn,
        doc! {
            "user": user.id.unwrap(),
            "slug": list_slug,
            "is_public": true
        },
        None,
    )
    .await
    .map_err(ApiError::WitherError)?;

    let list = match list {
        Some(list) => list,
        None => {
            debug!("List not found, returning 404 status code");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    debug!("Returning list to the user");
    let res = HttpResponse::Ok().json(list.to_json());
    Ok(res)
}
