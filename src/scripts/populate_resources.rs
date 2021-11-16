use futures::stream::StreamExt;
use wither::bson::doc;

use crate::context::Context;
use crate::lib::util::to_object_id;
use crate::models::Model as ModelTrait;

pub async fn run(ctx: &Context) {
  println!("Runing populate-resources script");

  let user_id = to_object_id("606385a800c7fe7c00ee5e09").expect("Failed to parse user ID");
  let mut cursor = ctx
    .models
    .resource
    .cursor(doc! { "user": user_id }, None)
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
