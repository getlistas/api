use crate::database::Database;
use crate::mailer::Mailer;

#[derive(Clone)]
pub struct Context {
    pub database: Database,
    pub mailer: Mailer,
}
