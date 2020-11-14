use serde::{Deserialize, Serialize};
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub external_id: String, // Auth0 ID
    pub name: String,
    pub nickname: String,
    pub picture: String,

    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    pub fn new(body: UserCreate) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: None,
            email: body.email.clone(),
            external_id: body.external_id.clone(), // Auth0 ID
            name: body.name.clone(),
            nickname: body.nickname.clone(),
            picture: body.picture.clone(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserCreate {
    pub email: String,
    pub external_id: String, // Auth0 ID
    pub name: String,
    pub nickname: String,
    pub picture: String,
}
