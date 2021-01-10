use actix_web::{http, web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use wither::bson::doc;
use wither::Model;

use crate::emails;
use crate::errors::ApiError;
use crate::lib::token;
use crate::models::user::User;
use crate::models::user::UserAuthenticate;
use crate::models::user::UserCreate;
use crate::Context;

type Response = actix_web::Result<HttpResponse>;

#[derive(Deserialize)]
struct PasswordResetBody {
    email: String,
}

#[derive(Deserialize)]
struct PasswordUpdateBody {
    token: String,
    password: String,
}

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/users").route(web::post().to(create_user)));
    cfg.service(web::resource("/users/verification/{token}").route(web::get().to(verify_user)));
    cfg.service(web::resource("/users/auth").route(web::post().to(create_token)));

    cfg.service(
        web::resource("/users/reset-password").route(web::post().to(request_password_reset)),
    );
    cfg.service(web::resource("/users/update-password").route(web::post().to(update_password)));
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

    debug!("Returning created user to the user");
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
        None => {
            return redirect_to(&format!(
                "{}/verify-email/failure",
                &ctx.settings.client_url
            ))
        }
    };

    debug!("Verifying user with email {}", &user.email);
    user.verified_at = Some(chrono::Utc::now().into());
    user.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    redirect_to(&format!(
        "{}/verify-email/success",
        &ctx.settings.client_url
    ))
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

    if user.locked_at.is_some() {
        debug!("User is locked, returning 401 response to the client");
        return Ok(HttpResponse::Unauthorized().finish());
    }

    if user.verified_at.is_none() {
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

async fn request_password_reset(
    ctx: web::Data<Context>,
    body: web::Json<PasswordResetBody>,
) -> Response {
    let email = &body.email;

    let user = User::find_one(&ctx.database.conn, doc! { "email": email }, None)
        .await
        .map_err(ApiError::WitherError)?;

    let mut user = match user {
        Some(user) => user,
        None => {
            debug!("User not found, returning 204 response to the user");
            return Ok(HttpResponse::NoContent().finish());
        }
    };

    user.set_password_reset_token();
    user.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Sending password reset email to the user {}", &user.email);
    let email = emails::create_password_reset_email(&ctx.settings.base_url, &user);
    ctx.send_email(email).await;

    debug!("Returning 204 status to the user");
    let res = HttpResponse::NoContent().finish();
    Ok(res)
}

async fn update_password(ctx: web::Data<Context>, body: web::Json<PasswordUpdateBody>) -> Response {
    let token = body.token.clone();
    let password = body.password.clone();

    let user = User::find_one(
        &ctx.database.conn,
        doc! { "password_reset_token": token },
        None,
    )
    .await
    .map_err(ApiError::WitherError)?;

    let mut user = match user {
        Some(user) => user,
        None => {
            debug!("User not found, returning 401 status to the user");
            return Ok(HttpResponse::Unauthorized().finish());
        }
    };

    // TODO: Password reset token should have a time to live
    user.set_password(password);
    user.unset_password_reset_token();
    user.save(&ctx.database.conn, None)
        .await
        .map_err(ApiError::WitherError)?;

    debug!("Returning 204 status to the user");
    let res = HttpResponse::NoContent().finish();
    Ok(res)
}

fn redirect_to(url: &str) -> Response {
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, url)
        .finish()
        .into_body())
}
