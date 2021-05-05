use lettre_email::Email;
use lettre_email::EmailBuilder;
use maud::html;

use crate::actors::subscription::on_list_removed::ListRemoved;
use crate::errors::Error;
use crate::models::list::List;
use crate::models::user::User;

pub fn create_confirm_email(from: &str, base_url: &str, user: &User) -> Result<Email, Error> {
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
    .from(from)
    .to(user.email.as_str())
    .subject("Confirm your Listas email address")
    .html(html.into_string())
    .build()
    .map_err(Error::BuildEmail)
}

pub fn create_password_reset_email(
  from: &str,
  base_url: &str,
  user: &User,
) -> Result<Email, Error> {
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
    .from(from)
    .to(user.email.as_ref())
    .subject("Reset your Listas password")
    .html(html.into_string())
    .build()
    .map_err(Error::BuildEmail)
}

pub fn create_subscription_removed_email(
  from: &str,
  user: &User,
  list: &List,
  removed_list: &ListRemoved,
) -> Result<Email, Error> {
  let list_url = format!("https://listas.io/list/{}/integrations", list.slug);
  let html = html! {
      head {
          title { "Subscription to " (removed_list.title) " list removed" }
          style type="text/css" {
              "h2, h4 { font-family: Arial, Helvetica, sans-serif; }"
          }
      }

      div {
          h2 { "Subscription to " (removed_list.title) " list removed" }
          p { "Dear " (user.name) "," }
          p {
              "We are sorry to let you know that we had to remove your subscription from"
              "the " (removed_list.title) " list. The list owner removed or made the list"
              "private."
          }
          p {
              "To review your current " (list.title) " list integrations, click"
              a href={(list_url)} { "here" }
          }
      }
  };

  EmailBuilder::new()
    .from(from)
    .to(user.email.as_str())
    .subject("Integration removed")
    .html(html.into_string())
    .build()
    .map_err(Error::BuildEmail)
}
