use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

use crate::lib::date;

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct List {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user: ObjectId,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub is_public: bool,

    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl List {
    pub fn to_json(&self) -> serde_json::Value {
        let this = self.clone();
        json!({
            "id": this.id.clone().unwrap().to_hex(),
            "user": this.user.to_hex(),
            "title": this.title,
            "description": this.description,
            "tags": this.tags,
            "slug": this.slug,
            "is_public": this.is_public,
            "created_at": date::to_rfc3339(this.created_at),
            "updated_at": date::to_rfc3339(this.updated_at)
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
}

impl ListUpdate {
    pub fn new(update: &mut Self) -> &mut Self {
        let tags = update.tags.clone().map(sanitize_tags).unwrap_or(vec![]);

        update.updated_at = Some(chrono::Utc::now().into());
        update.tags = Some(tags);
        update
    }
}

pub fn sanitize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.to_lowercase().trim().to_owned())
        .filter(|tag| tag.len() >= 1)
        .unique()
        .collect()
}
