use crate::database::Database;
use crate::lib::mailer::Mailer;

#[derive(Clone)]
pub struct Context {
    pub database: Database,
    pub mailer: Mailer,
}
