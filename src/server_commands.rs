use std::{
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    process::exit,
};

use crate::{kvs_error::Result, KvStoreError};
use crate::{Command, KvStore};
use bincode::deserialize_from;
use clap::Parser;
use log::info;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct ServerArgs {
    #[clap(short, long)]
    pub addr: Option<String>,
    #[clap(short, long)]
    pub engine: Option<String>,
}

#[derive(Debug)]
pub struct KvsServer {
    addr: SocketAddr,
    kvs: KvStore,
    engine: String,
}

impl KvsServer {
    pub fn new(
        addr: Option<String>,
        engine: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let sock_addr;
        let res_engine;

        match addr {
            Some(addr) => match addr.parse::<SocketAddr>() {
                Ok(sock) => sock_addr = sock,
                Err(_) => {
                    eprintln!("invalid address, dumbass");
                    exit(1);
                }
            },
            None => sock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4000),
        }

        match engine {
            Some(name) => match name.as_str() {
                "kvs" => res_engine = String::from("kvs"),
                "sled" => res_engine = String::from("sled"),
                _ => {
                    eprintln!("invalid engine, my good sir");
                    exit(1);
                }
            },
            None => res_engine = String::from("kvs"),
        }

        let kvs = KvStore::open(path)?;

        Ok(Self {
            addr: sock_addr,
            kvs,
            engine: res_engine,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        info!(env!("CARGO_PKG_VERSION"));
        info!(
            "Server listening on {}, via the engine {}",
            self.addr, self.engine
        );
        let listener = TcpListener::bind(self.addr)?;
        for stream in listener.incoming() {
            self.handle_stream(stream?)?;
        }
        Ok(())
    }

    fn handle_stream(&mut self, stream: TcpStream) -> Result<()> {
        let cmd = deserialize_from::<_, Command>(&stream)?;
        match cmd {
            Command::Set { key, value } => {
                self.kvs.set(key.into(), value.into())?;
            }
            Command::Get { key } => match self.kvs.get(key.to_string()) {
                Ok(res) => match res {
                    Some(value) => println!("{}", value),
                    None => {
                        println!("{}", KvStoreError::KeyNotFound);
                    }
                },
                Err(err) => println!("{}", err),
            },
            Command::Rm { key } => match self.kvs.remove(key.into()) {
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
        }
        Ok(())
    }
}
