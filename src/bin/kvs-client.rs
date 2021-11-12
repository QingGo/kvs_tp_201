#![feature(backtrace)]
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use anyhow::Result;
use clap::Parser;
use kvs::utils::*;
use kvs::KvsClient;

// If the type has a destructor, then it will not run when the process exits.
// So log won't be printed totally more of the time.
// lazy_static! {
//     static ref ROOT_LOGGER: slog::Logger = get_root_logger("kvs-server".to_string());
// }

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
    let root_logger: slog::Logger = get_root_logger("kvs-server".to_string());
    info!(root_logger, "start kvs-client"; "version" => env!("CARGO_PKG_VERSION"));
    let config = Config::parse();
    info!(root_logger, "Parse config successfully"; "config" => format!("{:?}", config));
    let ip_port = parse_ip_port(&config.addr)?;

    let input = format!("{:?}", config);
    KvsClient::new(ip_port, root_logger)?.send(&input)?;
    Ok(())
}
