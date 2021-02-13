use lettre_email::EmailBuilder;
use maud::html;

use crate::models::user::User;

pub fn create_confirm_email(base_url: &String, user: &User) -> EmailBuilder {
  let token = user.verification_token.as_ref().unwrap();
  let callback_url = format!("{}/users/verification/{}", base_url, token);
  let html = html! {
      head {
          title { "Hello from Listas" }
          style type="text/css" {
              "h2, h4 { font-family: Arial, Helvetica, sans-serif; }"
          }
      }
      div {
          h2 { "Hello from Listas!" }
          p { "Dear " (user.name) "," }
          p {
              "To use your Listas account, please confirm your email address "
              a href={(callback_url)} { "here" }
          }
      }
  };

  EmailBuilder::new()
    .to(user.email.as_str())
    .subject("Confirm your Listas email address")
    .html(html.into_string())
}

pub fn create_password_reset_email(base_url: &String, user: &User) -> EmailBuilder {
  let token = user.password_reset_token.as_ref().unwrap();
  let callback_url = format!("{}/password-reset?token={}", base_url, token);
  let html = html! {
      head {
          title { "Reset your Listas password" }
          style type="text/css" {
              "h2, h4 { font-family: Arial, Helvetica, sans-serif; }"
          }
      }
      div {
          h2 { "Reset your Listas password" }
          p { "Dear " (user.name) "," }
          p {
              "We received a request to reset the password for the Listas account "
              "associated with your email address. Click "
              a href={(callback_url)} { "here" }
              " to proceed."
          }
      }
  };

  EmailBuilder::new()
    .to(user.email.as_ref())
    .subject("Reset your Listas password")
    .html(html.into_string())
}
