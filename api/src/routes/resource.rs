use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::{Deserialize, Serialize};
use validator::Validate;
use wither::bson;
use wither::bson::{doc, Bson};
use wither::mongodb;
use wither::mongodb::options::FindOneAndUpdateOptions;

use crate::actors::subscription;
use crate::auth::UserID;
use crate::lib::id::ID;
use crate::lib::util;
use crate::lib::util::to_object_id;
use crate::models::resource::PrivateResource;
use crate::models::resource::Resource;
use crate::models::resource::ResourceUpdate;
use crate::models::Model as ModelTrait;
use crate::Context;
use crate::{auth, lib::date};

type ResourceUpdateBody = web::Json<ResourceUpdate>;

#[derive(Deserialize)]
struct Query {
  list: Option<String>,
  completed: Option<bool>,
  sort: Option<String>,
  search_text: Option<String>,
  skip: Option<u32>,
  limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct ResourceCreate {
  pub list: String,
  pub url: String,
  pub title: Option<String>,
  pub description: Option<String>,
  pub thumbnail: Option<String>,
  pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct PositionUpdate {
  pub list: String,
  pub previus: Option<String>,
}

type Response = actix_web::Result<HttpResponse>;
type Ctx = web::Data<Context>;
type ResourceCreateBody = web::Json<ResourceCreate>;
type PositionUpdateBody = web::Json<PositionUpdate>;

pub fn create_router(cfg: &mut web::ServiceConfig) {
  let auth = HttpAuthentication::bearer(auth::validator);

  // TODO: Move this route to its own resource-metrics endpoint
  cfg.service(
    web::resource("/resources/metrics")
      .route(web::get().to(get_resource_metrics))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/resources/{id}")
      .route(web::get().to(get_resource_by_id))
      .route(web::put().to(update_resource))
      .route(web::delete().to(remove_resource))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/resources/{id}/complete")
      .route(web::post().to(complete_resource))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/resources/{id}/undo-complete")
      .route(web::post().to(undo_complete_resource))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/resources/{id}/position")
      .route(web::put().to(update_position))
      .wrap(auth.clone()),
  );

  cfg.service(
    web::resource("/resources")
      .route(web::get().to(query_resources))
      .route(web::post().to(create_resource))
      .wrap(auth),
  );
}

async fn get_resource_by_id(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let resource = ctx
    .models
    .resource
    .find_one(doc! { "_id": &resource_id, "user": &user_id }, None)
    .await?;

  let resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Returning resource");
  let res = HttpResponse::Ok().json(resource.to_json());
  Ok(res)
}

async fn query_resources(ctx: Ctx, user_id: UserID, qs: web::Query<Query>) -> Response {
  let user_id = user_id.0;
  let mut pipeline = vec![];
  let mut filter = vec![];
  let mut must = vec![];

  filter.push(doc! {
    "equals": {
      "path": "user",
      "value": user_id
    }
  });

  if let Some(list_id) = qs.list.clone() {
    let list_id = util::to_object_id(list_id)?;
    filter.push(doc! {
      "equals": {
        "path": "list",
        "value": list_id
      }
    });
  }

  if let Some(ref search_text) = qs.search_text {
    must.push(doc! {
      "text": {
        "query": search_text,
        "path": ["title", "description", "tags"],
        "fuzzy": {
          "maxEdits": 2,
          "prefixLength": 3
        }
      }
    });
  }

  pipeline.push(doc! {
    "$search": {
      "index": "search",
      "compound": {
        "filter": filter,
        "must": must
      }
    }
  });

  // TODO: Remove this $match stage because it can drastically slow down query
  // results.
  // https://docs.atlas.mongodb.com/reference/atlas-search/performance/#-match-aggregation-stage-usage
  if let Some(is_completed) = qs.completed {
    // The { item : null } query matches documents that either contain the
    // item field whose value is null or that do not contain the item field.
    let key = if is_completed { "$ne" } else { "$eq" };
    pipeline.push(doc! {
      "$match": {
        "completed_at": { key: Bson::Null }
      }
    });
  }

  // TODO: Remove this $sort stage because it can drastically slow down query
  // results.
  // https://docs.atlas.mongodb.com/reference/atlas-search/performance/#-sort-aggregation-stage-usage
  let sort = match qs.sort.clone().as_deref() {
    Some("position_asc") => doc! { "position": 1 },
    Some("position_des") => doc! { "position": -1 },
    _ => match qs.completed {
      Some(true) => doc! { "completed_at": -1 },
      _ => doc! { "created_at": -1 },
    },
  };

  // When querying using full text search, use the score order to sort data.
  if qs.search_text.is_none() {
    pipeline.push(doc! { "$sort": sort });
  }

  if let Some(skip) = qs.skip {
    pipeline.push(doc! { "$skip": skip });
  }

  if let Some(limit) = qs.limit {
    pipeline.push(doc! { "$limit": limit });
  }

  let resources = ctx
    .models
    .resource
    .aggregate::<PrivateResource>(pipeline)
    .await?;

  debug!("Returning resources");
  let res = HttpResponse::Ok().json(resources);
  Ok(res)
}

async fn create_resource(ctx: Ctx, body: ResourceCreateBody, user_id: UserID) -> Response {
  let list_id = to_object_id(body.list.clone())?;
  let user_id = user_id.0;
  let url = util::parse_url(body.url.clone().as_str())?;
  let tags = body
    .tags
    .clone()
    .map(util::sanitize_tags)
    .unwrap_or_default();

  let list = ctx
    .models
    .list
    .find_one(doc! { "_id": &list_id,"user": &user_id }, None)
    .await?;

  let list = match list {
    Some(list) => list,
    None => {
      debug!("Failed creating Resource, asociated List not found");
      return Ok(HttpResponse::BadRequest().finish());
    }
  };

  if list.user != user_id {
    debug!("Failed creating Resource, Can not create resource in a not owned List");
    return Ok(HttpResponse::BadRequest().finish());
  }

  let position = ctx
    .models
    .list
    .get_position_for_new_resource(&list_id)
    .await?;

  let resource = Resource {
    id: None,
    position,
    tags,
    user: user_id,
    list: list_id,
    url: url.to_string(),
    title: body.title.clone(),
    description: body.description.clone(),
    thumbnail: body.thumbnail.clone(),
    created_at: date::now(),
    updated_at: date::now(),
    completed_at: None,
    html: None,
    text: None,
    author: None,
    length: None,
    publisher: None,
  };

  // TODO: Integrate validate method into a create method.
  match resource.validate() {
    Ok(_) => (),
    Err(_err) => {
      debug!("Failed creating Resource, payload is not valid. Returning 400 status code");
      return Ok(HttpResponse::BadRequest().finish());
    }
  };

  let resource = ctx.models.resource.create(resource).await?;
  let resource_id = resource.id.clone().unwrap();

  ctx
    .actors
    .subscription
    .try_send(subscription::on_resource_created::ResourceCreated {
      resource_id: resource_id.clone(),
    })
    .map_err(|err| error!("Failed to send message to subscription actor, {}", err))?;

  ctx
    .jobs
    .queue("populate_resources", vec![resource_id.clone().to_string()])
    .await;

  // TODO: Last acivity should be calculated in a reactive way.
  ctx
    .models
    .list
    .update_last_activity_at(&resource.list)
    .await
    .map_err(|err| {
      error!(
        "Failed to update last activity for list {}. Error {}",
        &resource.list, err
      )
    })?;

  debug!("Returning created resource");
  let resource: PrivateResource = resource.into();
  let res = HttpResponse::Created().json(resource);
  Ok(res)
}

async fn update_resource(ctx: Ctx, id: ID, body: ResourceUpdateBody, user_id: UserID) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let resource = ctx
    .models
    .resource
    .find_one(doc! { "_id": &resource_id, "user": &user_id }, None)
    .await?;

  let resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let mut body = body.into_inner();
  let body = ResourceUpdate::new(&mut body);
  let mut update = bson::to_document(&body).unwrap();

  match &body.list {
    Some(list_id) if !resource.list.eq(list_id) => {
      let last_position = ctx
        .models
        .list
        .get_position_for_new_resource(list_id)
        .await?;

      update.insert("position", last_position);
    }
    _ => {}
  };

  let options = FindOneAndUpdateOptions::builder()
    .return_document(mongodb::options::ReturnDocument::After)
    .build();

  let resource = ctx
    .models
    .resource
    .find_one_and_update(
      doc! { "_id": &resource_id, "user": &user_id },
      doc! { "$set": update },
      Some(options),
    )
    .await?;

  let resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Returning updated resource");
  let res = HttpResponse::Ok().json(resource.to_json());
  Ok(res)
}

async fn remove_resource(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let resource = ctx.models.resource.find_by_id(&resource_id).await?;
  let resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  let result = ctx
    .models
    .resource
    .delete_one(doc! { "_id": resource_id, "user": user_id })
    .await?;

  if result.deleted_count == 0 {
    debug!("Resource not found, returning 404 status code");
    return Ok(HttpResponse::NotFound().finish());
  }

  ctx
    .models
    .list
    .update_last_activity_at(&resource.list)
    .await
    .map_err(|err| {
      error!(
        "Failed to update last activity for list {}. Error {}",
        &resource.list, err
      )
    })?;

  debug!("Resource removed, returning 204 status code");
  let res = HttpResponse::NoContent().finish();
  Ok(res)
}

async fn complete_resource(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let resource = ctx
    .models
    .resource
    .find_one(doc! { "_id": &resource_id, "user": &user_id }, None)
    .await?;

  let resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  if resource.completed_at.is_some() {
    debug!("Resource was already completed, returnig 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  ctx
    .models
    .resource
    .update_one(
      doc! { "_id": &resource_id, "user": &user_id },
      doc! { "$set": { "completed_at": Bson::DateTime(date::now().into()) } },
      None,
    )
    .await?;

  debug!("Resource marked as completed, returning 202 status code");
  let res = HttpResponse::Accepted().finish();
  Ok(res)
}

async fn undo_complete_resource(ctx: Ctx, id: ID, user_id: UserID) -> Response {
  let resource_id = id.0;
  let user_id = user_id.0;

  let resource = ctx
    .models
    .resource
    .find_one(doc! { "_id": &resource_id, "user": &user_id }, None)
    .await?;

  let resource = match resource {
    Some(resource) => resource,
    None => {
      debug!("Resource not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  if resource.completed_at.is_none() {
    debug!("Resource is not complete, returnig 400 status code");
    return Ok(HttpResponse::BadRequest().finish());
  }

  ctx
    .models
    .resource
    .update_one(
      doc! { "_id": &resource_id, "user": &user_id },
      doc! { "$set": { "completed_at": Bson::Null } },
      None,
    )
    .await?;

  debug!("Resource unmarked as completed, returning 202 status code");
  let res = HttpResponse::Accepted().finish();
  Ok(res)
}

async fn update_position(ctx: Ctx, id: ID, user_id: UserID, body: PositionUpdateBody) -> Response {
  let resource_id = id.0;
  let list_id = to_object_id(body.list.clone())?;
  let user_id = user_id.0;
  let previus_resource_id = body.previus.clone();

  let resource_exists = ctx
    .models
    .resource
    .exists(doc! { "_id": &resource_id, "user": &user_id, "list": &list_id })
    .await?;

  if !resource_exists {
    debug!("Resource not found, returning 404 status code");
    return Ok(HttpResponse::NotFound().finish());
  }

  let position = match previus_resource_id {
    // If previus position is not sent, the new resource position is 0, it will
    // be inserted at the top of the list.
    None => 0,
    Some(previus_resource_id) => {
      let previus_resource_id = to_object_id(previus_resource_id)?;
      let query = doc! {
          "_id": &previus_resource_id,
          "user": &user_id,
          "list": &list_id,
      };
      let position = match ctx.models.resource.get_position(query).await? {
        Some(position) => position,
        None => {
          debug!("Resource not found, returning 404 status code");
          return Ok(HttpResponse::NotFound().finish());
        }
      };

      position + 1
    }
  };

  ctx
    .models
    .resource
    .update_many(
      doc! {
          "_id": doc! { "$ne": &resource_id },
          "user": &user_id,
          "list": &list_id,
          "position": doc! { "$gte": &position },
      },
      doc! { "$inc": { "position": 1 } },
      None,
    )
    .await?;

  ctx
    .models
    .resource
    .update_one(
      doc! { "_id": &resource_id },
      doc! {
        "$set": {
          "position": position,
          "updated_at": bson::to_bson(&date::now()).unwrap()
        }
      },
      None,
    )
    .await?;

  debug!("Resource position updated, returning 202 status code");
  let res = HttpResponse::Accepted().finish();
  Ok(res)
}

async fn get_resource_metrics(ctx: Ctx, user_id: UserID, qs: web::Query<Query>) -> Response {
  let user_id = user_id.0;
  let mut pipeline = vec![];
  let mut filter = vec![];
  let mut must = vec![];

  filter.push(doc! {
    "equals": {
      "path": "user",
      "value": user_id
    }
  });

  if let Some(list_id) = qs.list.clone() {
    let list_id = util::to_object_id(list_id)?;
    filter.push(doc! {
      "equals": {
        "path": "list",
        "value": list_id
      }
    });
  }

  if let Some(ref search_text) = qs.search_text {
    must.push(doc! {
      "text": {
        "query": search_text,
        "path": ["title", "description", "tags"]
      }
    });
  }

  pipeline.push(doc! {
    "$search": {
      "index": "search",
      "compound": {
        "filter": filter,
        "must": must
      }
    }
  });

  pipeline.push(doc! {
    "$group": {
      "_id":       Bson::Null,
      "total":     { "$sum": 1 },
      "completed": {
        "$sum": {
          "$cond": [{ "$eq": [ "$completed_at", Bson::Null ] }, 0, 1 ]
        }
      }
    }
  });

  let metrics = ctx.models.resource.aggregate::<Metric>(pipeline).await?;

  let metric = match metrics.get(0) {
    Some(metric) => metric,
    None => {
      debug!("Resource metrics not found, returning 404 status code");
      return Ok(HttpResponse::NotFound().finish());
    }
  };

  debug!("Returning resource metrics");
  let res = HttpResponse::Ok().json(metric);
  Ok(res)
}

#[derive(Debug, Serialize, Deserialize)]
struct Metric {
  total: i64,
  completed: i64,
}
