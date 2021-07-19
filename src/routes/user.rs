use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;
use wither::bson::doc;
use wither::bson::Bson;
use wither::Model as WitherModelTrait;

use crate::auth;
use crate::auth::UserID;
use crate::lib::create_demo_data_for_user;
use crate::lib::date;
use crate::lib::token;
use crate::models::user::PrivateUser;
use crate::models::user::PublicUser;
use crate::models::user::User;
use crate::models::Model as ModelTrait;
use crate::Context;
use crate::{emails, lib::google};
use crate::{errors::Error, lib::util};

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;

#[derive(Deserialize)]
struct UserCreateBody {
  pub email: String,
  pub password: String,
  pub name: String,
}
#[derive(Deserialize)]
struct PasswordResetBody {
  email: String,
}

#[derive(Deserialize)]
struct PasswordUpdateBody {
  token: String,
  password: String,
}

#[derive(Deserialize)]
pub struct AuthenticateBody {
  pub email: String,
  pub password: String,
}

#[derive(Deserialize)]
pub struct GoogleAuthenticate {
  pub token: String,
}

type GoogleAuthenticateBody = web::Json<GoogleAuthenticate>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(web::resource("/users").route(web::post().to(create_user)));
  cfg.service(
    web::resource("/users/me")
      .route(web::get().to(get_session))
      .wrap(auth),
  );
  cfg.service(web::resource("/users/verification/{token}").route(web::get().to(verify_user_email)));
  cfg.service(web::resource("/users/auth").route(web::post().to(create_token)));
  cfg.service(web::resource("/users/google-auth").route(web::post().to(create_token_from_google)));
  cfg.service(web::resource("/users/reset-password").route(web::post().to(request_password_reset)));
  cfg.service(web::resource("/users/update-password").route(web::post().to(update_password)));
  cfg.service(web::resource("/users/{slug}").route(web::get().to(find_user_by_slug)));
  cfg.service(web::resource("/users/{slug}/metrics").route(web::get().to(get_metrics)));
}

async fn create_user(ctx: web::Data<Context>, body: web::Json<UserCreateBody>) -> Response {
  let password = User::hash_password(body.password.clone()).await?;
  let verification_token = util::create_random_string(40);
  let now = date::now();

  let user = User {
    id: None,
    password,
    email: body.email.clone(),
    name: body.name.clone(),
    slug: User::create_slug(body.email.clone().as_str()),
    avatar: None,
    google_id: None,
    subscription: None,
    verification_token: Some(verification_token),
    verification_token_set_at: Some(now),
    password_reset_token: None,
    password_reset_token_set_at: None,
    created_at: now,
    updated_at: now,
    verified_at: None,
    locked_at: None,
  };

  match user.validate() {
    Ok(_) => (),
    Err(_err) => {
      debug!("Failed to create User, payload is not valid. Returning 400 status code");
      return Ok(HttpResponse::BadRequest().finish());
    }
  };

  let user = ctx.models.user.create(user).await?;

  debug!("Sending confirm email to the user {}", &user.email);
  let send_from = ctx.settings.mailer.from.as_str();
  let base_url = ctx.settings.base_url.as_str();
  let confirm_email = emails::create_confirm_email(send_from, base_url, &user)?;
  ctx.mailer.send(confirm_email).await?;

  debug!("Creating demo list and resource for new user");
  create_demo_data_for_user::create(&ctx.models, user.id.clone().unwrap()).await?;

  debug!("Returning created user");
  let user: PrivateUser = user.into();
  let res = HttpResponse::Created().json(user);
  Ok(res)
}

async fn get_session(ctx: Ctx, user: UserID) -> Response {
  let user_id = user.0;
  let user = ctx
    .models
    .user
    .find_one(doc! { "_id": user_id }, None)
    .await?;

  let user: PrivateUser = match user {
    Some(user) => user.into(),
    None => {
      debug!("User not found, returning 401 status code");
      return Ok(HttpResponse::Unauthorized().finish());
    }
  };

  debug!("Returning user");
  let res = HttpResponse::Ok().json(user);
  Ok(res)
}

async fn get_metrics(ctx: Ctx, slug: web::Path<String>) -> Response {
  let slug = slug.clone();
  let user = ctx
    .models
    .user
    .find_one(doc! { "slug": &slug }, None)
    .await?;

  let user = match user {
    Some(user) => user,
    None => {
      debug!("User not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let pipeline = vec![
    doc! {
      "$match": {
        "user": user.id.unwrap(),
        "completed_at": { "$exists": true, "$ne": Bson::Null }
      }
    },
    doc! {
      "$group": {
        "_id": {
          "$dateToString": {
            "date":   "$completed_at",
            "format": "%Y-%m-%d",
          }
        },
        "completed_count": { "$sum": 1 }
      }
    },
    doc! { "$sort": { "_id": 1 } },
  ];

  let metrics = ctx.models.resource.aggregate::<Metric>(pipeline).await?;

  debug!("Returning user metrics");
  let res = HttpResponse::Ok().json(metrics);
  Ok(res)
}

async fn verify_user_email(ctx: web::Data<Context>, token: web::Path<String>) -> Response {
  let user = User::find_one(
    &ctx.database.conn,
    doc! { "verification_token": token.into_inner() },
    None,
  )
  .await
  .map_err(Error::WitherError)?;

  let user = match user {
    Some(user) => user,
    None => {
      let url = format!("{}/verify-email/failure", &ctx.settings.client_url);
      return util::redirect_to(url);
    }
  };

  let user_id = user.id.clone().unwrap();
  ctx
    .models
    .user
    .update_one(
      doc! { "_id": user_id },
      doc! { "$set": { "verified_at": Bson::DateTime(date::now().into()) } },
      None,
    )
    .await?;

  let url = format!("{}/verify-email/success", &ctx.settings.client_url);
  util::redirect_to(url.as_str())
}

async fn create_token(ctx: web::Data<Context>, body: web::Json<AuthenticateBody>) -> Response {
  let email = &body.email;
  let password = &body.password;

  let user = ctx
    .models
    .user
    .find_one(doc! { "email": email }, None)
    .await?;

  let user = match user {
    Some(user) => user,
    None => {
      debug!("User not found, returning 401 to the user");
      return Ok(HttpResponse::Unauthorized().finish());
    }
  };

  if user.locked_at.is_some() {
    debug!("User is locked, returning 401 to the user");
    return Ok(HttpResponse::Unauthorized().finish());
  }

  if user.verified_at.is_none() {
    debug!("User is not verified, returning 401 to the user");
    return Ok(HttpResponse::Unauthorized().finish());
  }

  if !user.is_password_match(password) {
    debug!("User password does not match, returning 401 to the user");
    return Ok(HttpResponse::Unauthorized().finish());
  }

  let private_key = ctx.settings.auth.secret.as_str();
  let token = token::create_token(user, private_key);
  let payload = json!({ "access_token": token });

  debug!("Returning created user token to the client");
  let res = HttpResponse::Created().json(payload);
  Ok(res)
}

async fn create_token_from_google(
  ctx: web::Data<Context>,
  body: GoogleAuthenticateBody,
) -> Response {
  let id_token = &body.token;
  let client_id = ctx.settings.oauth.google.client_id.as_str();
  let google_token = google::validate(id_token, client_id).await;

  let google_token = match google_token {
    Ok(token) => token,
    Err(_) => {
      debug!("Failed to validate google token, returning 401 to the user");
      return Ok(HttpResponse::Unauthorized().finish());
    }
  };

  // These fields are only included when the user has granted the "profile"
  // and "email" OAuth scopes to the application.
  let subject = google_token.sub;
  let email = google_token.email.unwrap();
  let name = google_token.name.unwrap();
  let is_google_email_verified = google_token.email_verified.unwrap();
  let avatar = google_token.picture.unwrap();

  let user = ctx
    .models
    .user
    .find_one(doc! { "email": &email }, None)
    .await?;

  let user = match user {
    Some(user) => {
      let query = doc! { "_id": user.id.as_ref().unwrap() };
      let update = doc! { "$set": { "avatar": avatar } };
      ctx
        .models
        .user
        .find_one_and_update(query, update, None)
        .await?;

      user
    }
    None => {
      debug!("User not found, creating a new user based on google authentication");

      let password = User::hash_password(util::create_random_string(10)).await?;
      let slug = User::create_slug(email.as_str());
      let now = date::now();
      let mut user = User {
        id: None,
        password,
        email,
        name,
        slug,
        avatar: Some(avatar),
        google_id: Some(subject),
        subscription: None,
        verification_token: None,
        verification_token_set_at: None,
        password_reset_token: None,
        password_reset_token_set_at: None,
        created_at: now,
        updated_at: now,
        verified_at: Some(now),
        locked_at: None,
      };

      if !is_google_email_verified {
        let token = util::create_random_string(40);

        user.verification_token = Some(token);
        user.verification_token_set_at = Some(now);
        user.verified_at = None;

        debug!("Sending confirm email to the user {}", &user.email);
        let send_from = ctx.settings.mailer.from.as_str();
        let base_url = ctx.settings.base_url.as_str();
        let confirm_email = emails::create_confirm_email(send_from, base_url, &user)?;
        ctx.mailer.send(confirm_email).await?;
      }

      let user = ctx.models.user.create(user).await?;

      user
    }
  };

  if user.locked_at.is_some() {
    debug!("User is locked, returning 401 response to the client");
    return Ok(HttpResponse::Unauthorized().finish());
  }

  // TODO: Handle response to let the user know why he can not login.
  if user.verified_at.is_none() {
    debug!("User is not verified, returning 401 response to the client");
    return Ok(HttpResponse::Unauthorized().finish());
  }

  let private_key = ctx.settings.auth.secret.as_str();
  let token = token::create_token(user, private_key);
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

  let user = ctx
    .models
    .user
    .find_one(doc! { "email": email }, None)
    .await?;

  let mut user = match user {
    Some(user) => user,
    None => {
      debug!("User not found, returning 204 response to the user");
      return Ok(HttpResponse::NoContent().finish());
    }
  };

  user.set_password_reset_token();
  user
    .save(&ctx.database.conn, None)
    .await
    .map_err(Error::WitherError)?;

  debug!("Sending password reset email to the user {}", &user.email);
  let send_from = ctx.settings.mailer.from.as_str();
  let base_url = &ctx.settings.base_url.as_str();
  let email = emails::create_password_reset_email(send_from, base_url, &user)?;
  ctx.mailer.send(email).await?;

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
  .map_err(Error::WitherError)?;

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
  user
    .save(&ctx.database.conn, None)
    .await
    .map_err(Error::WitherError)?;

  debug!("Returning 204 status to the user");
  let res = HttpResponse::NoContent().finish();
  Ok(res)
}

async fn find_user_by_slug(ctx: web::Data<Context>, slug: web::Path<String>) -> Response {
  let slug = slug.clone();
  let user = ctx
    .models
    .user
    .find_one(doc! { "slug": &slug }, None)
    .await?;

  let user: PublicUser = match user {
    Some(user) => user.into(),
    None => {
      debug!("User not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Returning public user");
  let res = HttpResponse::Ok().json(user);
  Ok(res)
}

#[derive(Debug, Serialize, Deserialize)]
struct Metric {
  #[serde(alias = "_id")]
  date: String,
  completed_count: i64,
}
