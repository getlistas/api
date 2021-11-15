use crate::database;
use crate::models;
use crate::models::user::User;

#[derive(Clone)]
pub struct Model {
  pub database: database::Database,
}

impl models::Model<User> for Model {
  fn get_database(&self) -> &database::Database {
    &self.database
  }
}

impl Model {
  pub fn new(database: database::Database) -> Self {
    Self { database }
  }
}
