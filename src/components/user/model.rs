use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use wither::bson::{doc, oid::ObjectId};
use wither::Model;

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub password: String,
    pub email: String,
    pub slug: String,
    pub name: String,
    pub avatar: Option<String>,

    pub verification_token: String,

    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl User {
    pub fn new(body: UserCreate) -> Self {
        let now = chrono::Utc::now();
        let password = bcrypt::hash(body.password, bcrypt::DEFAULT_COST).unwrap();

        Self {
            id: None,
            password,
            email: body.email.clone(),
            name: body.name.clone(),
            slug: body.slug.clone(),
            avatar: None,
            verification_token: nanoid!(),
            created_at: now,
            updated_at: now,
            verified_at: None,
        }
    }

    pub fn is_password_match(&self, password: &str) -> bool {
        bcrypt::verify(password, &self.password).unwrap_or(false)
    }

    pub fn to_display(&self) -> UserPublic {
        UserPublic {
            id: self.id.clone().unwrap().to_string(),
            email: self.email.clone(),
            name: self.name.clone(),
            slug: self.slug.clone(),
        }
    }
}

// This struct is used in actix extractors to retrieve the user ObjectID from
// the authentication token.
pub struct UserID(pub ObjectId);

#[derive(Debug, Serialize, Deserialize)]
pub struct UserCreate {
    pub email: String,
    pub password: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: String,
    pub email: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAuthenticate {
    pub email: String,
    pub password: String,
}
