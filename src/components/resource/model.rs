use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct Resource {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user: ObjectId,
    pub list: ObjectId,

    pub url: String,
    pub title: String,
    pub description: Option<String>,

    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub completed_at: Option<DateTime>,
}

impl Resource {
    pub fn new(body: ResourceCreate, user_id: ObjectId, list_id: ObjectId) -> Self {
        let now = chrono::Utc::now().into();
        Self {
            id: None,
            user: user_id,
            list: list_id,

            url: body.url.clone(),
            title: body.title.clone(),
            description: body.description.clone(),

            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceCreate {
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
