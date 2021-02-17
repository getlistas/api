pub mod integration;
pub mod list;
pub mod resource;
pub mod user;

use crate::database::Database;

#[derive(Clone)]
pub struct Models {
  pub integration: integration::Integration,
}

impl Models {
  pub fn new(database: Database) -> Self {
    let integration = integration::Integration::new(database);

    Self { integration }
  }
}
