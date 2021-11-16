mod populate_resources;

use crate::context::Context;

pub async fn run(context: &Context) {
  let matches = clap::App::new("Listas scripts CLI")
    .subcommand(
      clap::App::new("cli")
        .subcommand(clap::App::new("populate-resources").help("Populates all resources")),
    )
    .get_matches();

  // Subcommand "cli" is always present because this arg is used to stop the API
  // execution and start the scripts CLI instead.
  let matches = matches
    .subcommand_matches("cli")
    .expect("Failed to get cli subcommand matches");

  if matches.is_present("populate-resources") {
    populate_resources::run(context).await;
  }
}
