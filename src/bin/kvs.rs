use std::process::exit;

use clap::StructOpt;
use kvs::{Args, Command, KvStore, KvStoreError, Result};

fn main() -> Result<()> {
    let args = Args::parse();

    let mut kvstore = KvStore::open("")?;

    match &args.command {
        Command::Set { key, value } => {
            kvstore.set(key.into(), value.into())?;
        }
        Command::Get { key } => match kvstore.get(key.to_string()) {
            Ok(res) => match res {
                Some(value) => println!("{}", value),
                None => {
                    println!("{}", KvStoreError::KeyNotFound);
                }
            },
            Err(err) => println!("{}", err),
        },
        Command::Rm { key } => match kvstore.remove(key.into()) {
            Ok(()) => {}
            Err(KvStoreError::KeyNotFound) => {
                println!("{}", KvStoreError::KeyNotFound);
                exit(1);
            }
            Err(err) => {
                return Err(err);
            }
        },
        Command::Open { path: _ } => {
            unimplemented!();
        }
        Command::Version => {
            println!("we're here");
            unimplemented!();
        }
    }

    Ok(())
}
