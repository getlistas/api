use serde::{Deserialize, Serialize};
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct Resource {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub collection: ObjectId,

    pub url: String,
    pub title: String,
    pub description: Option<String>,

    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Resource {
    pub fn new(body: ResourceCreate) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: None,
            collection: body.collection.clone(),

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
    pub collection: ObjectId,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ResourceUpdate {
    pub fn new(update: &mut Self) -> &mut Self {
        update.updated_at = Some(chrono::Utc::now());
        update
    }
}
