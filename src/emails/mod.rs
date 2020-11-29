use lettre_email::Email;
use maud::html;

pub fn create_confirm_email(name: &String, email: &String, token: &String) -> Email {
    let callback_url = format!("http://localhost:8080/users/verification/{}", token);
    let html = html! {
        head {
            title { "Hello from Doneq" }
            style type="text/css" {
                "h2, h4 { font-family: Arial, Helvetica, sans-serif; }"
            }
        }
        div style="display: flex; flex-direction: column; align-items: center;" {
            h2 { "Hello from doneq!" }
            p { "Dear " (name) "," }
            p {
                "To use your account, please confirm your doneq email address "
                a href={(callback_url)} { "here" }
            }
        }
    };

    let email = lettre_email::EmailBuilder::new()
        .from("nicolas.delvalle@gmail.com")
        .to(email.as_str())
        .subject("Confirm your doneq email address")
        .html(html.into_string())
        .build()
        .unwrap();

    email
}
