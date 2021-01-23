use actix_web::{web, HttpResponse};
use serde::Deserialize;
use wither::bson::doc;
use wither::Model;

use crate::errors::ApiError;
use crate::models::user::Subscription;
use crate::models::user::User;
use crate::Context;

// https://developer.paddle.com/webhook-reference/subscription-alerts/subscription-created
// https://developer.paddle.com/webhook-reference/subscription-alerts/subscription-cancelled

#[derive(Deserialize)]
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
    pub marketing_consent: String,
    pub passthrough: String,
    pub quantity: String,
    pub unit_price: String,
    pub p_signature: String,
    pub event_time: String,

    // Subscription cancelled attributes
    pub cancellation_effective_date: Option<String>,

    // Subscription created attributes
    pub cancel_url: Option<String>,
    pub update_url: Option<String>,
    pub next_bill_date: Option<String>,
    pub source: Option<String>,
}

type Response = actix_web::Result<HttpResponse>;
type EventBody = web::Json<PaddleEvent>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/paddle/events").route(web::post().to(event)));
}

async fn event(ctx: web::Data<Context>, event: EventBody) -> Response {
    // TODO: Validate Paddle event with our Public Key.

    match event.alert_name.as_str() {
        "subscription_created" => create_subscription(ctx, event).await,
        "subscription_cancelled" => cancell_subscription(ctx, event).await,
        _ => {
            error!("Paddle event {} not recognized", event.alert_name);
            return Ok(HttpResponse::Ok().finish());
        }
    }
}

pub async fn create_subscription(ctx: web::Data<Context>, event: EventBody) -> Response {
    let user = User::find_one(&ctx.database.conn, doc! { "email": &event.email }, None)
        .await
        .map_err(ApiError::WitherError)?;

    let mut user = match user {
        Some(user) => user,
        None => {
            error!("User not found, returning 404 status code");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    let subscription = Subscription {
        id: event.subscription_id,
        plan_id: event.subscription_plan_id,
        status: event.status,
        checkout_id: event.checkout_id,
        updated_at: event.event_time,
        next_bill_at: event.next_bill_date,
        cancel_url: event.cancel_url,
        update_url: event.update_url,
        cancellation_effective_at: None,
    };

    let update = doc! {
      "$set": doc! {
        "subscription": subscription.into()
      }
    };

    user.update(&ctx.database.conn, None, update, None)
        .await
        .map_err(ApiError::WitherError)?;

    let res = HttpResponse::Ok().finish();
    Ok(res)
}

pub async fn cancell_subscription(ctx: web::Data<Context>, event: EventBody) -> Response {
    let user = User::find_one(&ctx.database.conn, doc! { "email": &event.email }, None)
        .await
        .map_err(ApiError::WitherError)?;

    let mut user = match user {
        Some(user) => user,
        None => {
            error!("User not found, returning 404 status code");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    let subscription = Subscription {
        id: event.subscription_id,
        plan_id: event.subscription_plan_id,
        status: event.status,
        checkout_id: event.checkout_id,
        updated_at: event.event_time,
        next_bill_at: None,
        cancel_url: None,
        update_url: None,
        cancellation_effective_at: event.cancellation_effective_date,
    };

    let update = doc! {
      "$set": doc! {
        "subscription": subscription.into()
      }
    };

    user.update(&ctx.database.conn, None, update, None)
        .await
        .map_err(ApiError::WitherError)?;

    let res = HttpResponse::Ok().finish();
    Ok(res)
}
