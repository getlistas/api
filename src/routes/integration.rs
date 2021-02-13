use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::stream::StreamExt;
use serde::Deserialize;
use wither::bson::doc;
use wither::Model;

use crate::auth;
use crate::errors::ApiError;
use crate::lib::util::parse_url;
use crate::lib::util::to_object_id;
use crate::lib::{date, resource_metadata};
use crate::models::list::List;
use crate::models::{resource::Resource, user::UserID};
use crate::Context;

#[derive(Deserialize)]
struct RSSCreate {
  list: String,
  url: String,
}

type Ctx = web::Data<Context>;
type Response = actix_web::Result<HttpResponse>;
type RSSCreateBody = web::Json<RSSCreate>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  cfg.service(
    web::resource("/integrations/rss")
      .route(web::post().to(create_integration))
      .wrap(auth.clone()),
  );
}

async fn create_integration(ctx: Ctx, body: RSSCreateBody, user_id: UserID) -> Response {
  let list_id = to_object_id(body.list.clone())?;
  let url = parse_url(body.url.as_str())?;

  let list = List::find_one(
    &ctx.database.conn,
    doc! {
        "_id": &list_id,
        "user": &user_id.0,
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

  let mut rss_entries = ctx.rss.get_entries(&url).await?;
  let conn = ctx.database.conn.clone();
  let mut resources_from_entries_futures = vec![];
  for entry in rss_entries.iter_mut() {
    let conn = conn.clone();
    let user_id = user_id.clone();
    let list_id = list_id.clone();

    let task = async move {
      let url = parse_url(entry.link.as_str()).unwrap();
      let metadata = resource_metadata::get_website_metadata(&url).await;

      let resource = Resource {
        id: None,
        // Placeholder position, we are not saving the resource yet.
        position: 0,
        // TODO allow user to add custom tags to all integration
        // resources.
        tags: vec![],
        user: user_id.0,
        list: list_id,
        url: url.to_string(),
        title: entry.title.clone(),
        // TODO Use metadata description if no description was found in
        // the RSS response.
        description: Some(entry.description.clone()),
        thumbnail: metadata.thumbnail,
        created_at: date::now(),
        updated_at: date::now(),
        completed_at: None,
      };

      resource
    };

    resources_from_entries_futures.push(task);
  }

  debug!("Fetching resources metadata from RSS feed");
  let mut resources = futures::stream::iter(resources_from_entries_futures)
    .buffer_unordered(10)
    .collect::<Vec<Resource>>()
    .await;

  let last_resource = Resource::find_last(&ctx.database.conn, &user_id.0, &list_id).await?;
  let position = last_resource
    .map(|resource| resource.position + 1)
    .unwrap_or(0);

  let mut resource_futures = vec![];
  for (index, resource) in resources.iter_mut().enumerate() {
    let conn = ctx.database.conn.clone();
    let task = async move {
      resource.position = position + (index as i32);
      resource
        .save(&conn, None)
        .await
        .map_err(ApiError::WitherError)
    };
    resource_futures.push(task);
  }

  debug!("Creating resources from RSS feed");
  futures::stream::iter(resource_futures)
    .buffer_unordered(20)
    .collect::<Vec<Result<(), ApiError>>>()
    .await
    .into_iter()
    .collect::<Result<(), ApiError>>()?;

  debug!("Returning 200 status code");
  let res = HttpResponse::Ok().finish();
  Ok(res)
}
