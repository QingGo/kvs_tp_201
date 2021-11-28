#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

extern crate anyhow;

use clap::Parser;
use kvs::thread_pool::{SharedQueueThreadPool, ThreadPool};
use kvs::utils::*;
use kvs::IKvsServer;
use kvs::KvStore;
use kvs::KvsEngine;
use kvs::KvsServer;
use kvs::Result;
use kvs::SledKvsEngine;

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
    let root_logger: slog::Logger = get_root_logger("kvs-server".to_string());
    // or slog_stdlog will not work
    let _scope_guard = slog_scope::set_global_logger(root_logger.clone());
    slog_stdlog::init().unwrap();
    info!(root_logger, "Starting kvs-server"; "version" => env!("CARGO_PKG_VERSION"));
    let config = Config::parse();
    info!(root_logger, "Parse config successfully"; "config" => format!("{:?}", config));
    let last_engine = get_last_engine();
    let engine_name = get_engine(last_engine, config.engine)?;
    let ip_port = parse_ip_port(&config.addr)?;

    let log = root_logger.new(o!("engine" => "kvs"));
    log::info!("engine_name: {}", engine_name);
    let pool = SharedQueueThreadPool::new(num_cpus::get() as u32)?;
    match engine_name.as_str() {
        "kvs" => KvsServer::new(ip_port, KvStore::new()?, pool, log)?.run()?,
        "sled" => KvsServer::new(ip_port, SledKvsEngine::new()?, pool, log)?.run()?,
        _ => panic!("Unknown engine name"),
    };
    Ok(())
}
