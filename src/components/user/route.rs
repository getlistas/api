use actix_web::{web, HttpResponse};
use serde_json::json;
use wither::bson::doc;
use wither::Model;

use crate::errors::ApiError;
use crate::lib::token;
use crate::user::model::User;
use crate::user::model::UserAuthenticate;
use crate::user::model::UserCreate;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("").route(web::post().to(create_user)));
    cfg.service(web::resource("/auth").route(web::post().to(create_token)));
}

async fn create_user(ctx: web::Data<Context>, body: web::Json<UserCreate>) -> Response {
    let mut user = User::new(body.into_inner());

    user.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning created user to the client");
    let res = HttpResponse::Created().json(user.to_display());
    Ok(res)
}

async fn create_token(ctx: web::Data<Context>, body: web::Json<UserAuthenticate>) -> Response {
    let email = &body.email;
    let password = &body.password;
    let user = User::find_one(&ctx.database.conn, doc! { "email": email }, None)
        .await
        .map_err(ApiError::WitherError)?;

    let user = match user {
        Some(user) => user,
        None => return Ok(HttpResponse::Unauthorized().finish()),
    };

    if !user.is_password_match(password) {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let token = token::create_token(&user);
    let payload = json!({ "access_token": token });

    debug!("Returning created user token to the client");
    let res = HttpResponse::Created().json(payload);
    Ok(res)
}
