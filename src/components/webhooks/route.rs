use actix_web::{web, HttpResponse};
use wither::bson::doc;
use wither::Model;

use crate::errors::ApiError;
use crate::user::model::ReqUser;
use crate::user::model::User;
use crate::user::model::UserCreate;
use crate::webhooks::model::Auth0User;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/auth0/users").route(web::post().to(create_user)));
}

// TODO: Validate the request actually comes from Auth0
async fn create_user(ctx: web::Data<Context>, body: web::Json<Auth0User>) -> Response {
    let user = User::find_one(&ctx.database.conn, doc! { "email": &body.email }, None)
        .await
        .map_err(ApiError::WitherError)?;

    if user.is_some() {
        debug!(
            "User with email {} found, returning 200 status code to the client",
            &body.email
        );
        let res = HttpResponse::Ok().json(ReqUser::from_user(user.unwrap()));
        return Ok(res);
    }

    let payload = UserCreate {
        email: body.email.clone(),
        external_id: body.user_id.clone(),
        name: body.name.clone(),
        nickname: body.nickname.clone(),
        picture: body.picture.clone(),
    };
    let mut user = User::new(payload);
    user.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!(
        "User with email {} created, returning 200 status code to the client",
        &body.email
    );
    let res = HttpResponse::Ok().json(ReqUser::from_user(user));
    Ok(res)
}
