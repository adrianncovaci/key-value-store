use clap::StructOpt;
use kvs::{ClientArgs, KvsClient, Result};

fn main() -> Result<()> {
    let args = ClientArgs::parse();
    let mut client = KvsClient::new(args.addr)?;

    client.send(args.command)?;

    Ok(())
}
