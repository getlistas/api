use serde::{Deserialize, Serialize};
use serde_json::json;
use wither::bson::{doc, oid::ObjectId, Bson};
use wither::bson::{DateTime, Document};
use wither::mongodb::options::FindOneOptions;
use wither::mongodb::Database;
use wither::Model;

use crate::errors::ApiError;
use crate::lib::{date, util};

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct Resource {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user: ObjectId,
    pub list: ObjectId,
    pub url: String,
    pub title: String,
    pub position: i32,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub completed_at: Option<DateTime>,
}

impl Resource {
    pub async fn find_last(
        conn: &Database,
        user_id: &ObjectId,
        list_id: &ObjectId,
    ) -> Result<Option<Self>, ApiError> {
        let query = doc! { "user": user_id, "list": list_id };
        let sort = doc! { "position": -1 };
        let options = FindOneOptions::builder().sort(Some(sort)).build();

        Self::find_one(conn, query, Some(options))
            .await
            .map_err(ApiError::WitherError)
    }

    pub async fn find_next(
        conn: &Database,
        user_id: &ObjectId,
        list_id: &ObjectId,
    ) -> Result<Option<Self>, ApiError> {
        let query = doc! {
            "user": user_id,
            "list": list_id,
            "completed_at": Bson::Null
        };
        let sort = doc! { "position": 1 };
        let options = FindOneOptions::builder().sort(Some(sort)).build();

        Self::find_one(conn, query, Some(options))
            .await
            .map_err(ApiError::WitherError)
    }

    pub async fn find_last_completed(
        conn: &Database,
        user_id: &ObjectId,
        list_id: &ObjectId,
    ) -> Result<Option<Self>, ApiError> {
        let query = doc! {
            "user": user_id,
            "list": list_id,
            "completed_at": doc! { "$exists": true }
        };
        let sort = doc! { "completed_at": -1 };
        let options = FindOneOptions::builder().sort(sort).build();

        Self::find_one(conn, query, Some(options))
            .await
            .map_err(ApiError::WitherError)
    }

    pub async fn get_position(conn: &Database, query: Document) -> Result<Option<i32>, ApiError> {
        let this = Self::find_one(conn, query, None)
            .await
            .map_err(ApiError::WitherError)?;

        match this {
            Some(this) => Ok(Some(this.position)),
            None => Ok(None),
        }
    }

    pub async fn find_by_url(
        conn: &Database,
        user_id: &ObjectId,
        url: String,
    ) -> Result<Option<Self>, ApiError> {
        let query = doc! { "user": user_id, "url": url };

        Self::find_one(conn, query, None)
            .await
            .map_err(ApiError::WitherError)
    }

    pub fn to_json(&self) -> serde_json::Value {
        let this = self.clone();
        json!({
            "id": this.id.clone().unwrap().to_hex(),
            "user": this.user.to_hex(),
            "list": this.list.to_hex(),
            "url": this.url,
            "title": this.title,
            "description": this.description,
            "thumbnail": this.thumbnail,
            "position": this.position,
            "tags": this.tags,
            "created_at": date::to_rfc3339(this.created_at),
            "updated_at": date::to_rfc3339(this.updated_at),
            "completed_at": this.completed_at.map(|date| date::to_rfc3339(date))
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
}

impl ResourceUpdate {
    pub fn new(update: &mut Self) -> &mut Self {
        if update.tags.is_some() {
            update.tags = Some(
                update
                    .tags
                    .clone()
                    .map(util::sanitize_tags)
                    .unwrap_or(vec![]),
            );
        }

        update.updated_at = Some(date::now());
        update
    }
}
