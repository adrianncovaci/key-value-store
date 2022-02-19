use clap::StructOpt;
use kvs::{KvsServer, Result, ServerArgs};

fn main() -> Result<()> {
    env_logger::init();
    let args = ServerArgs::parse();
    let mut server = KvsServer::new(args.addr, args.engine, "")?;
    server.run()?;

    Ok(())
}
