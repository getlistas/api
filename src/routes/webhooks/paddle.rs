use actix_web::{web, HttpResponse};
use serde::Deserialize;
use wither::bson::doc;
use wither::bson::to_bson;
use wither::bson::DateTime;
use wither::Model;

use crate::errors::Error;
use crate::lib::date;
use crate::models::user::Subscription;
use crate::models::user::User;
use crate::Context;

// https://developer.paddle.com/webhook-reference/subscription-alerts/subscription-created
// https://developer.paddle.com/webhook-reference/subscription-alerts/subscription-cancelled

#[derive(Clone, Deserialize)]
struct PaddleEvent {
  pub alert_id: String,
  pub alert_name: String,
  pub subscription_id: String,
  pub subscription_plan_id: String,
  pub user_id: String,
  pub email: String,
  pub checkout_id: String,
  pub status: String,
  pub currency: String,
  pub quantity: String,
  pub unit_price: String,
  pub p_signature: String,
  pub event_time: String,
  pub passthrough: Option<String>,
  pub marketing_consent: Option<String>,

  // Subscription created attributes
  pub cancel_url: Option<String>,
  pub update_url: Option<String>,
  pub next_bill_date: Option<String>,
  pub source: Option<String>,

  // Subscription cancelled attributes
  pub cancellation_effective_date: Option<String>,
}

type Response = actix_web::Result<HttpResponse>;
type EventBody = web::Json<PaddleEvent>;
type Form = web::Form<PaddleEvent>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  cfg.service(web::resource("").route(web::post().to(webhook)));
}

async fn webhook(ctx: web::Data<Context>, event: Form) -> Response {
  // TODO: Validate Paddle event with our Public Key.
  debug!("Processing Paddle webhook event");

  match event.alert_name.as_str() {
    "subscription_created" => create_subscription(ctx, event).await,
    "subscription_cancelled" => cancell_subscription(ctx, event).await,
    _ => {
      error!("Paddle event {} not recognized", event.alert_name);
      return Ok(HttpResponse::BadRequest().finish());
    }
  }
}

async fn create_subscription(ctx: web::Data<Context>, event: Form) -> Response {
  let user = User::find_one(&ctx.database.conn, doc! { "email": &event.email }, None)
    .await
    .map_err(Error::WitherError)?;

  let user = match user {
    Some(user) => user,
    None => {
      error!("User not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let event = event.clone();
  let now = date::now();
  println!("aca {:?}", &event.next_bill_date);
  println!("aca2 {:?}", &date::from_ymd(event.next_bill_date.clone().unwrap().as_str()));

  let next_bill_at = date::from_ymd(event.next_bill_date.unwrap().as_str()).unwrap();
  let subscription = Subscription {
    id: event.subscription_id,
    plan_id: event.subscription_plan_id,
    status: event.status,
    checkout_id: event.checkout_id,
    updated_at: now,
    next_bill_at: Some(next_bill_at),
    cancel_url: event.cancel_url,
    update_url: event.update_url,
    cancellation_effective_at: None,
  };

  let update = doc! {
    "$set": doc! {
      "subscription": to_bson(&subscription).unwrap()
    }
  };

  user
    .update(&ctx.database.conn, None, update, None)
    .await
    .map_err(Error::WitherError)?;

  let res = HttpResponse::Ok().finish();
  Ok(res)
}

async fn cancell_subscription(ctx: web::Data<Context>, event: Form) -> Response {
  let user = User::find_one(&ctx.database.conn, doc! { "email": &event.email }, None)
    .await
    .map_err(Error::WitherError)?;

  let user = match user {
    Some(user) => user,
    None => {
      error!("User not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let event = event.clone();
  let now = date::now();
  let cancellation_effective_at =
    date::from_rfc3339(event.cancellation_effective_date.unwrap().as_str()).unwrap();
  let subscription = Subscription {
    id: event.subscription_id,
    plan_id: event.subscription_plan_id,
    status: event.status,
    checkout_id: event.checkout_id,
    updated_at: now,
    next_bill_at: None,
    cancel_url: None,
    update_url: None,
    cancellation_effective_at: Some(cancellation_effective_at),
  };

  let update = doc! {
    "$set": doc! {
      "subscription": to_bson(&subscription).unwrap()
    }
  };

  user
    .update(&ctx.database.conn, None, update, None)
    .await
    .map_err(Error::WitherError)?;

  let res = HttpResponse::Ok().finish();
  Ok(res)
}
