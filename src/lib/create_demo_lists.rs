use wither::bson::oid::ObjectId;
use wither::mongodb::Database;
use wither::Model;

use crate::models::list::List;
use crate::lib::date;
use crate::errors::ApiError;

pub async fn create(database: &Database, user: ObjectId) -> Result<(), ApiError> {
  let now = date::now();
  let mut list = List {
    id: None,
    user,
    title: "Demo list".to_owned(),
    description: Some("This is a demo list create automatically".to_owned()),
    slug: "demo-list".to_owned(),
    is_public: false,
    tags: vec!["demo".to_owned()],
    fork: None,
    created_at: now,
    updated_at: now,
  };

  list
    .save(&database, None)
    .await
    .map_err(ApiError::WitherError)?;

  Ok(())
}