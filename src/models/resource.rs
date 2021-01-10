use serde::{Deserialize, Serialize};
use serde_json::json;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::mongodb;
use wither::mongodb::options::FindOneOptions;
use wither::Model;

use crate::errors::ApiError;
use crate::lib::date;

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

    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub completed_at: Option<DateTime>,
}

impl Resource {
    pub fn new(body: ResourceCreate, user_id: ObjectId, list_id: ObjectId, position: i32) -> Self {
        let now = chrono::Utc::now().into();
        Self {
            id: None,
            user: user_id,
            list: list_id,

            position,
            url: body.url.clone(),
            title: body.title.clone(),
            description: body.description.clone(),
            thumbnail: body.thumbnail.clone(),

            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    pub async fn find_last(
        conn: &mongodb::Database,
        user_id: &ObjectId,
        list_id: &ObjectId,
    ) -> Result<Option<Self>, wither::WitherError> {
        let query = doc! { "user": user_id, "list": list_id };
        let sort = doc! { "position": -1 };
        let options = FindOneOptions::builder().sort(Some(sort)).build();
        Self::find_one(conn, query, Some(options)).await
    }

    pub async fn is_unique_by_user(
        conn: &mongodb::Database,
        user_id: &ObjectId,
        url: String,
    ) -> Result<bool, ApiError> {
        let query = doc! { "user": user_id, "url": url };
        let count = Self::collection(conn)
            .count_documents(query, None)
            .await
            .map_err(ApiError::MongoError)?;

        Ok(count == 0)
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
            "created_at": date::to_rfc3339(this.created_at),
            "updated_at": date::to_rfc3339(this.updated_at),
            "completed_at": this.completed_at.map(|date| date::to_rfc3339(date))
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceCreate {
    pub list: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
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
    pub updated_at: Option<DateTime>,
}

impl ResourceUpdate {
    pub fn new(update: &mut Self) -> &mut Self {
        update.updated_at = Some(chrono::Utc::now().into());
        update
    }
}
