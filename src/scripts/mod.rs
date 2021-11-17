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
          )
          .arg(
            Arg::with_name("non-populated")
              .short("n")
              .long("non-populated")
              .value_name("non-populated")
              .help("Populate resource that were not previously populated")
              .takes_value(false),
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
    let non_populated = matches.is_present("non-populated");
    populate_resources::run(context, user, non_populated).await;
  }
}
