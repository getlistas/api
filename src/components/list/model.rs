use itertools::Itertools;
use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct List {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user: ObjectId,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,

    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl List {
    pub fn new(body: ListCreate, user_id: ObjectId) -> Self {
        let now = chrono::Utc::now().into();
        let tags = body.tags.map(sanitize_tags).unwrap_or(vec![]);

        Self {
            id: None,
            user: user_id,

            title: body.title.clone(),
            description: body.description.clone(),
            tags,

            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListCreate {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
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

fn sanitize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.to_lowercase().trim().to_owned())
        .filter(|tag| tag.len() >= 1)
        .unique()
        .collect()
}
