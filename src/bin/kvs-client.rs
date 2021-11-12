#![feature(backtrace)]
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use kvs::utils::*;
use lazy_static::lazy_static;
use std::io::prelude::*;

lazy_static! {
    static ref ROOT_LOGGER: slog::Logger = get_root_logger("kvs-server".to_string());
}

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "QingGo")]
struct Config {
    #[clap(short('V'))]
    // 更改了默认 -v 的行为
    version: bool,
    command: String,
    key: String,
    value: Option<String>,
    #[clap(long("addr"), default_value = "127.0.0.1:4000")]
    addr: String,
}

fn main() -> Result<()> {
    info!(ROOT_LOGGER, "start kvs-client"; "version" => env!("CARGO_PKG_VERSION"));
    let config = Config::parse();
    let ip_port = parse_ip_port(&config.addr)?;
    let mut stream = std::net::TcpStream::connect(ip_port)?;
    let input = format!("{:?}", config);
    info!(ROOT_LOGGER, "send request"; "request" => &input);
    stream.write_all(input.as_bytes())?;
    let mut buf = vec![0; 1024];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Err(anyhow!("server closed"));
    }
    info!(ROOT_LOGGER, "recv response"; "response" => &String::from_utf8_lossy(&buf[..n]).to_string());
    Ok(())
}
