use futures::stream::StreamExt;
use wither::bson::doc;

use crate::context::Context;
use crate::lib::util::to_object_id;
use crate::models::Model as ModelTrait;

pub async fn run<S: AsRef<str>>(ctx: &Context, user_id: Option<S>) {
  println!("Runing populate-resources script");

  let mut query = doc! {};
  if let Some(user_id) = user_id {
    let user_id = to_object_id(user_id.as_ref()).expect("Failed to parse user ID");
    query.insert("user", user_id);
  }

  let mut cursor = ctx
    .models
    .resource
    .cursor(query, None)
    .await
    .expect("Failed to get model cursor");

  // TODO: Handle this cursor in parallel.
  while let Some(result) = cursor.next().await {
    let resource = result.expect("Failed to get resource");
    println!("Resource: {:?}", resource.url);
    let resource_id = resource.id.clone().expect("Failed to get resource ID");
    ctx
      .models
      .resource
      .populate(resource_id)
      .await
      .expect("Failed to populate resource");
  }
}
