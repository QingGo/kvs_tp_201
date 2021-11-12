#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;
#[macro_use]
extern crate lazy_static;

extern crate anyhow;

use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use kvs::utils::*;
use std::io::prelude::*;
use std::net::TcpListener;

lazy_static! {
    static ref ROOT_LOGGER: slog::Logger = get_root_logger("kvs-server".to_string());
}

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "QingGo")]
struct Config {
    #[clap(short('V'))]
    // 更改了默认 -v 的行为
    version: bool,
    #[clap(long("addr"), default_value = "127.0.0.1:4000")]
    addr: String,
    #[clap(long("engine"), value_name("ENGINE-NAME"))]
    engine: Option<String>,
}

fn main() -> Result<()> {
    info!(ROOT_LOGGER, "Starting kvs-server"; "version" => env!("CARGO_PKG_VERSION"));
    let config = Config::parse();
    let last_engine = get_last_engine();
    let engine = get_engine(last_engine, config.engine)?;
    let ip_port = parse_ip_port(&config.addr)?;
    match engine.as_str() {
        "kvs" => {
            let log = ROOT_LOGGER.new(o!("engine" => "kvs"));
            let listener = TcpListener::bind(ip_port)?;
            let mut buf = vec![0; 1024];
            loop {
                let (stream, _) = listener.accept()?;
                loop {
                    let mut stream = stream.try_clone()?;
                    let n = stream.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    let request_string = &String::from_utf8_lossy(&buf[..n]).to_string();
                    info!(log, "recv request"; "request" => &request_string);
                    info!(log, "send response"; "response" => &request_string);
                    stream.write_all(request_string.as_bytes())?;
                }
            }
        }
        "sled" => {
            unimplemented!()
        }
        _ => {
            return Err(anyhow!("invalid engine name"));
        }
    }
}
