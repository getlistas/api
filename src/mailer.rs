use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, SmtpTransport, Transport};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Mailer {
    transport: Arc<Mutex<SmtpTransport>>,
}

impl Mailer {
    pub fn new() -> Result<Self, lettre::smtp::error::Error> {
        let credentials = Credentials::new(
            "apikey".to_string(),
            "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".to_string(),
        );

        let transport = SmtpClient::new_simple("smtp.sendgrid.net")?
            .credentials(credentials)
            .transport();

        let mailer = Self {
            transport: Arc::new(Mutex::new(transport)),
        };
        Ok(mailer)
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
