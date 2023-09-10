use std::env::current_dir;
use std::process;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crabkvs::error::Error;
use serde::Serialize;

#[derive(Parser, Debug, Serialize)]
#[command(author,version,about,long_about=None)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug, Serialize)]
enum Action {
    #[clap(name = "get")]
    Get { key: String },
    #[clap(name = "set")]
    Set { key: String, value: String },
    #[clap(name = "rm")]
    Remove { key: String },
}

fn main() -> Result<()> {
    // If no argument, exit the program with non-zero code.
    if std::env::args().len() < 2 {
        process::exit(1)
    }

    let args = Args::parse();

    match args.action {
        Action::Get { key } => {
            let mut writer = crabkvs::KvStore::open(current_dir()?)?;
            writer.get(key).map(|v| match v {
                Some(v) => println!("{}", v),
                None => println!("Key not found"),
            })?;
            Ok(())
        }
        Action::Set { key, value } => {
            let mut writer = crabkvs::KvStore::open(current_dir()?)?;
            writer.set(key, value)?;

            Ok(())
        }
        Action::Remove { key } => {
            let mut writer = crabkvs::KvStore::open(current_dir()?)?;
            match writer.remove(key) {
                Ok(()) => {}
                Err(e) => {
                    if let Some(err) = e.downcast_ref::<Error>() {
                        match err {
                            Error::KeyNotFound => {
                                println!("Key not found");
                                process::exit(1);
                            }
                        }
                    } else {
                        return Err(e);
                    }
                }
            };
            Ok(())
        }
    }
}
