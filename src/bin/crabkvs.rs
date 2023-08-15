use std::{process, unimplemented};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author,version,about,long_about=None)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug)]
enum Action {
    #[clap(name = "get")]
    Get { value: Option<String> },
    #[clap(name = "set")]
    Set {
        key: Option<String>,
        value: Option<String>,
    },
    #[clap(name = "rm")]
    Remove { value: Option<String> },
}

fn main() {
    // If no argument, exit the program with non-zero code.
    if std::env::args().len() < 2 {
        process::exit(1)
    }
    let args = Args::parse();

    match args.action {
        Action::Get { .. } => print_and_exit(),
        Action::Set { .. } => print_and_exit(),
        Action::Remove { .. } => print_and_exit(),
    }
}

fn print_and_exit() {
    unimplemented!("unimplemented");
}
