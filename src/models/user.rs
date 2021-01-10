use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
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

    pub verification_token: Option<String>,
    pub password_reset_token: Option<String>,

    pub created_at: DateTime,
    pub updated_at: DateTime,

    pub verified_at: Option<DateTime>,
    pub locked_at: Option<DateTime>,
    pub password_reset_token_set_at: Option<DateTime>,
}

impl User {
    pub fn new(body: UserCreate) -> Self {
        let now = chrono::Utc::now().into();
        let password = bcrypt::hash(body.password, bcrypt::DEFAULT_COST).unwrap();

        Self {
            id: None,
            password,
            email: body.email.clone(),
            name: body.name.clone(),
            slug: body.slug.clone(),
            avatar: None,

            verification_token: Some(create_random_token()),
            password_reset_token: None,

            created_at: now,
            updated_at: now,
            verified_at: None,
            locked_at: None,
            password_reset_token_set_at: None,
        }
    }

    pub fn is_password_match(&self, password: &str) -> bool {
        bcrypt::verify(password, &self.password).unwrap_or(false)
    }

    pub fn set_password_reset_token(&mut self) -> String {
        let now = chrono::Utc::now().into();
        let token = create_random_token();

        self.password_reset_token = Some(token.clone());
        self.password_reset_token_set_at = Some(now);

        token
    }

    pub fn unset_password_reset_token(&mut self) {
        self.password_reset_token = None;
        self.password_reset_token_set_at = None;
    }

    pub fn set_password(&mut self, password: String) {
        let password = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
        self.password = password;
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

pub fn create_random_token() -> String {
    nanoid!(40, &nanoid::alphabet::SAFE)
}
