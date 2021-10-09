use actix_web::error::BlockingError;
use lettre::smtp::authentication::Credentials;
use lettre::smtp::error::Error as SmtpError;
use lettre::{SmtpClient, SmtpTransport, Transport};
use lettre_email::Email;
use std::sync::{Arc, Mutex};

use crate::errors::Error;
use crate::settings::Settings;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum MailerError {
  #[error("Failed to send email: Could not acquire transport mutex")]
  LockTransport,

  #[error("Failed to send email: SMTP transport error {0}")]
  Smtp(#[from] SmtpError),

  #[error("Failed to send email: Blocking error {0}")]
  BlockingError(#[from] BlockingError),
}

#[derive(Clone)]
pub struct Mailer {
  transport: Arc<Mutex<SmtpTransport>>,
}

impl Mailer {
  pub fn new(settings: &Settings) -> Result<Self, lettre::smtp::error::Error> {
    let token = settings.sendgrid.token.clone();
    let credentials = Credentials::new("apikey".to_string(), token);

    let transport = SmtpClient::new_simple("smtp.sendgrid.net")?
      .credentials(credentials)
      .transport();

    Ok(Self {
      transport: Arc::new(Mutex::new(transport)),
    })
  }

  pub async fn send(&self, email: Email) -> Result<(), Error> {
    let transport = self.transport.clone();
    let sent = actix_web::web::block(move || {
      transport
        // If another user of this mutex panicked while holding the mutex, then
        // transport.lock() call will return an error once the mutex is acquired.
        .lock()
        .map_err(|_| MailerError::LockTransport)?
        .send(email.into())
        .map_err(MailerError::Smtp)?;

      Ok::<(), MailerError>(())
    })
    .await;

    match sent {
      Ok(_) => Ok(()),
      Err(err) => Err(Error::SendEmail(MailerError::BlockingError(err))),
    }
  }
}
