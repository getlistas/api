use wither::bson::oid::ObjectId;

use crate::errors::Error;
use crate::lib::date;
use crate::models::list::List;
use crate::models::resource::Resource;
use crate::models::Models;

pub async fn create(models: &Models, user: ObjectId) -> Result<(), Error> {
  let list_title = "To read later";
  let list_description = "This is a demo list create automatically by Listas";
  let list_slug = "to-read-later";

  let resource_title = "Hello World — Listas";
  let resource_url = "https://collectednotes.com/getlistas/hello-world";
  let resource_description =
    "Here are a few things I want to share about why we are working on Listas.";
  let resource_thumbnail =
    "https://photos.collectednotes.com/photos/63/622fd450-952c-44c2-81a5-e27c8494bd11";

  let tags = vec!["tutorial".to_owned()];

  let now = date::now();
  let list = models
    .create(List {
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
      archived_at: None,
    })
    .await?;

  models
    .create(Resource {
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
    })
    .await?;

  Ok(())
}
