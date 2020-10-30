use serde::{Deserialize, Serialize};
use wither::bson::{doc, oid::ObjectId};
use wither::mongodb;
use wither::ModelCursor;
use wither::{prelude::*, Result};

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct Resource {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
}

impl Resource {
    pub async fn get(conn: &mongodb::Database) -> Result<ModelCursor<Self>> {
        let cursor = Self::find(conn, None, None).await?;
        Ok(cursor)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
