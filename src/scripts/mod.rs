mod populate_resources;

use clap::{App, Arg};

use crate::context::Context;

pub async fn run(context: &Context) {
  let matches = App::new("Listas scripts CLI")
    .subcommand(
      App::new("cli").subcommand(
        App::new("populate-resources")
          .help("Populates all resources")
          .arg(
            Arg::with_name("user")
              .short("u")
              .long("user")
              .value_name("user")
              .help("Populate resource for a specific user")
              .takes_value(true),
          ),
      ),
    )
    .get_matches();

  // Subcommand "cli" is always present because this arg is used to stop the API
  // execution and start the scripts CLI instead.
  let matches = matches
    .subcommand_matches("cli")
    .expect("Failed to get cli subcommand matches");

  if let Some(matches) = matches.subcommand_matches("populate-resources") {
    let user = matches.value_of("user");
    populate_resources::run(context, user).await;
  }
}
