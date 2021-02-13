use wither::bson::oid::ObjectId;
use wither::mongodb::Database;
use wither::Model;

use crate::models::list::List;
use crate::models::resource::Resource;
use crate::lib::date;
use crate::errors::ApiError;

pub async fn create(database: &Database, user: ObjectId) -> Result<(), ApiError> {

  let list_title = "Demo list";
  let list_description = "This is a demo list create automatically";
  let list_slug = "demo-list";

  let resource_title = "";
  let resource_url = "";
  let resource_description = "";
  let resource_thumbnail = "";

  let tags = vec!["demo".to_owned()];

  let now = date::now();
  let mut list = List {
    id: None,
    user: user.clone(),
    title: list_title.to_owned(),
    description: Some(list_description.to_owned()),
    slug: list_slug.to_owned(),
    is_public: false,
    tags: tags.clone(),
    fork: None,
    created_at: now,
    updated_at: now,
  };

  list
    .save(&database, None)
    .await
    .map_err(ApiError::WitherError)?;

  let mut resource = Resource {
    id: None,
    position: 0,
    tags: tags.clone(),
    user: user.clone(),
    list: list.id.clone().unwrap(),
    url: resource_url.to_owned(),
    title: resource_title.to_owned(),
    description: Some(resource_description.to_owned()),
    thumbnail: Some(resource_thumbnail.to_owned()),
    created_at: now,
    updated_at: now,
    completed_at: None,
  };

  resource
    .save(&database, None)
    .await
    .map_err(ApiError::WitherError)?;

  Ok(())
}