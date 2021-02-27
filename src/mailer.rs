use actix_web::error::BlockingError;
use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, SmtpTransport, Transport};
use std::sync::{Arc, Mutex};
use lettre::smtp::error::Error as SMTPError;

use crate::errors::Error;
use crate::settings::Settings;


#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum MailerError {
  #[error("Failed to acquire mailer transport mutex")]
  LockTransport,

  #[error("Failed to send email using SMTP transport {0}")]
  SMTP(#[from] SMTPError),

  #[error("Failed to send email actix_web::web::block operation was cancelled")]
  Canceled
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

  pub async fn send(&self, email: lettre::SendableEmail) -> Result<(), Error> {
    let transport = self.transport.clone();
    let sent = actix_web::web::block(move || {
      transport
        // If another user of this mutex panicked while holding the mutex, then
        // transport.lock() call will return an error once the mutex is acquired.
        .lock()
        .map_err(|_| MailerError::LockTransport)?
        .send(email)
        .map_err(MailerError::SMTP)?;
      Ok(())
    })
    .await;

    match sent {
      Ok(_) => Ok(()),
      Err(err) => {
        match err {
          BlockingError::Canceled => Err(Error::SendEmail(MailerError::Canceled)),
          BlockingError::Error(err) => Err(Error::SendEmail(err))
        }
      }
    }
  }
}
