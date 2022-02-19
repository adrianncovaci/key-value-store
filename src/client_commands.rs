use crate::{kvs_error::Result, response::Response};
use bincode::{de::read::IoReader, deserialize_from, Deserializer};
use clap::{AppSettings, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufReader, BufWriter, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::PathBuf,
    process::exit,
};

#[derive(Hash, Debug, Eq, PartialEq, Subcommand, Serialize, Deserialize)]
pub enum Command {
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Set {
        key: String,
        value: String,
    },
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Get {
        key: String,
    },
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Rm {
        key: String,
    },
    Open {
        path: PathBuf,
    },
}

#[derive(Debug)]
pub struct CommandPosition {
    pub start: u64,
    pub length: u64,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct ClientArgs {
    #[clap(subcommand)]
    pub command: Command,
    #[clap(short, long)]
    pub addr: Option<String>,
}

#[derive(Debug)]
pub struct KvsClient {
    addr: SocketAddr,
    writer: BufWriter<TcpStream>,
    reader: BufReader<TcpStream>,
}

impl KvsClient {
    pub fn new(addr: Option<String>) -> Result<Self> {
        let sock_addr;

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

        let socket = TcpStream::connect(sock_addr.clone())?;

        Ok(Self {
            addr: sock_addr,
            writer: BufWriter::new(socket.try_clone()?),
            reader: BufReader::new(socket),
        })
    }

    pub fn send(&mut self, cmd: Command) -> Result<Response> {
        let cmd = bincode::serialize(&cmd)?;

        let bytes_written = self.writer.write(&cmd)?;
        println!("{}", bytes_written);
        self.writer.flush()?;

        let response = deserialize_from::<_, Response>(&mut self.reader)?;
        println!("{:?}", response);

        Ok(response)
    }
}
