use serde::Deserialize;

#[derive(Deserialize)]
pub struct Pagination {
  pub limit: Option<i64>,
  pub skip: Option<i64>,
}
