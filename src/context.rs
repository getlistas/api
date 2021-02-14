use lettre_email::EmailBuilder;

use crate::database::Database;
use crate::integrations::rss::RSS;
use crate::mailer::Mailer;
use crate::settings::Settings;

#[derive(Clone)]
pub struct Context {
  pub database: Database,
  pub mailer: Mailer,
  pub settings: Settings,
  pub rss: RSS,
}

impl Context {
  pub async fn send_email(&self, email: EmailBuilder) {
    let email = email
      .from(self.settings.mailer.from.as_str())
      .build()
      .unwrap();

    self.mailer.send(email.into()).await.unwrap();
  }
}
