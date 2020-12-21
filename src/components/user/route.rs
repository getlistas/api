use actix_web::{http, web, HttpResponse};
use serde_json::json;
use wither::bson::doc;
use wither::Model;

use crate::emails;
use crate::errors::ApiError;
use crate::lib::token;
use crate::user::model::User;
use crate::user::model::UserAuthenticate;
use crate::user::model::UserCreate;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/users").route(web::post().to(create_user)));
    cfg.service(web::resource("/users/verification/{token}").route(web::get().to(verify_user)));
    cfg.service(web::resource("/users/auth").route(web::post().to(create_token)));
}

async fn create_user(ctx: web::Data<Context>, body: web::Json<UserCreate>) -> Response {
    let mut user = User::new(body.into_inner());

    user.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    let verification_token = user.verification_token.clone().unwrap();

    debug!("Sending confirm email to the user {}", &user.email);
    let confirm_email = emails::create_confirm_email(
        &ctx.settings.base_url,
        &user.name,
        &user.email,
        &verification_token,
    );
    ctx.send_email(confirm_email).await;

    debug!("Returning created user to the client");
    let res = HttpResponse::Created().json(user.to_display());
    Ok(res)
}

async fn verify_user(ctx: web::Data<Context>, token: web::Path<String>) -> Response {
    let user = User::find_one(
        &ctx.database.conn,
        doc! { "verification_token": token.into_inner() },
        None,
    )
    .await
    .map_err(ApiError::WitherError)?;

    let mut user = match user {
        Some(user) => user,
        // TODO: Replace with frontend 404 page.
        None => return redirect_to("https://github.com/ndelvalle"),
    };

    debug!("Verifying user with email {}", &user.email);
    user.verified_at = Some(chrono::Utc::now().into());
    user.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    redirect_to("https://listas.io/verify-email/success")
}

async fn create_token(ctx: web::Data<Context>, body: web::Json<UserAuthenticate>) -> Response {
    let email = &body.email;
    let password = &body.password;
    let user = User::find_one(&ctx.database.conn, doc! { "email": email }, None)
        .await
        .map_err(ApiError::WitherError)?;

    let user = match user {
        Some(user) => user,
        None => {
            debug!("User not found, returning 401 response to the client");
            return Ok(HttpResponse::Unauthorized().finish());
        }
    };

    if user.verified_at.is_none() {
        // TODO: Give feedback so the user can understand why he can not login.
        debug!("User is not verified, returning 401 response to the client");
        return Ok(HttpResponse::Unauthorized().finish());
    }

    if !user.is_password_match(password) {
        debug!("User password does not match, returning 401 response to the client");
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let private_key = ctx.settings.auth.secret.as_str();
    let token = token::create_token(&user, private_key);
    let payload = json!({ "access_token": token });

    debug!("Returning created user token to the client");
    let res = HttpResponse::Created().json(payload);
    Ok(res)
}

fn redirect_to(url: &str) -> Response {
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, url)
        .finish()
        .into_body())
}
