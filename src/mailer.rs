use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, SmtpTransport, Transport};
use std::sync::{Arc, Mutex};

use crate::settings::Settings;

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

  pub async fn send(
    &self,
    email: lettre::SendableEmail,
  ) -> Result<(), actix_web::error::BlockingError<lettre::smtp::error::Error>> {
    let transport = self.transport.clone();
    match actix_web::web::block(move || transport.lock().unwrap().send(email)).await {
      Ok(_) => Ok(()),
      Err(err) => {
        error!("Failed to send email. {}", &err);
        Err(err)
      }
    }
  }
}
