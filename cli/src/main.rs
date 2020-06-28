use clap::{App, Arg, SubCommand};
use serve::serve;

#[tokio::main]
async fn main() {
    let matches = App::new("barista")
        .subcommand(
            SubCommand::with_name("serve")
                .arg(
                    Arg::with_name("config")
                        .long("config")
                        .short("c")
                        .value_name("FILE")
                        .help("sets a custom config")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("website-path")
                        .value_name("DIR")
                        .help("sets the directory of the web menu")
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("serve", args) => {
            serve(args).await;
        }
        _ => {}
    }
}
