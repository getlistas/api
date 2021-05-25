use crate::database;
use crate::models;
use crate::models::like::Like;

#[derive(Clone)]
pub struct Model {
  pub database: database::Database,
}

impl models::Model<Like> for Model {
  fn get_database(&self) -> &database::Database {
    &self.database
  }
}

impl Model {
  pub fn new(database: database::Database) -> Self {
    Self { database }
  }
}
