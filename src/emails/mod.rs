use lettre_email::EmailBuilder;
use maud::html;

pub fn create_confirm_email(
    base_url: &String,
    name: &String,
    email: &String,
    token: &String,
) -> EmailBuilder {
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
            p { "Dear " (name) "," }
            p {
                "To use your account, please confirm your Listas email address "
                a href={(callback_url)} { "here" }
            }
        }
    };

    EmailBuilder::new()
        .to(email.as_str())
        .subject("Confirm your Listas email address")
        .html(html.into_string())
}
