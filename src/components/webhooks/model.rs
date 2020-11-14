use serde::{Deserialize, Serialize};

// https://auth0.com/docs/users/user-profiles
// https://auth0.com/docs/users/user-profile-structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Auth0User {
    pub email: String,
    pub user_id: String,
    pub name: String,
    pub nickname: String,
    pub picture: String,
}
