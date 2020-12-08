use lettre_email::EmailBuilder;
use maud::html;

pub fn create_confirm_email(name: &String, email: &String, token: &String) -> EmailBuilder {
    let callback_url = format!("http://localhost:8080/users/verification/{}", token);
    let html = html! {
        head {
            title { "Hello from Doneq" }
            style type="text/css" {
                "h2, h4 { font-family: Arial, Helvetica, sans-serif; }"
            }
        }
        div {
            h2 { "Hello from doneq!" }
            p { "Dear " (name) "," }
            p {
                "To use your account, please confirm your doneq email address "
                a href={(callback_url)} { "here" }
            }
        }
    };

    EmailBuilder::new()
        .to(email.as_str())
        .subject("Confirm your doneq email address")
        .html(html.into_string())
}
