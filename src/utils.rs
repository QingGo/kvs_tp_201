use anyhow::anyhow;
use anyhow::Result;
use slog::Drain;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

pub fn get_root_logger(process: String) -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().stderr().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("process" => process))
}

pub fn get_last_engine() -> Option<String> {
    if let Ok(file) = File::open("last_engine.txt") {
        let mut file = BufReader::new(file);
        let mut line = String::new();
        file.read_line(&mut line).unwrap();
        return Some(line.trim().to_string());
    } else {
        None
    }
}

pub fn write_engine_to_file(engine: &str) -> Result<()> {
    let mut file = File::create("last_engine.txt")?;
    file.write_all(engine.as_bytes())?;
    Ok(())
}

pub fn get_engine(last_engine: Option<String>, config_engine: Option<String>) -> Result<String> {
    let engine: String;
    if let Some(last_engine) = last_engine {
        if let Some(config_engine) = config_engine {
            if config_engine != last_engine {
                return Err(anyhow!("engine not match lase used"));
            } else {
                engine = config_engine;
            }
        } else {
            engine = last_engine;
        }
    } else if let Some(config_engine) = config_engine {
        engine = config_engine;
    } else {
        engine = "kvs".to_string();
    }
    write_engine_to_file(&engine)?;
    Ok(engine)
}

pub fn parse_ip_port(ip_port: &str) -> Result<(std::net::IpAddr, u16)> {
    let mut iter = ip_port.split(':');
    let ip = iter.next().ok_or_else(|| anyhow!("invalid ip"))?;
    let port = iter.next().ok_or_else(|| anyhow!("invalid port"))?;
    let port = port.parse::<u16>().map_err(|_| anyhow!("invalid port"))?;
    let ip = ip
        .parse::<std::net::IpAddr>()
        .map_err(|_| anyhow!("invalid ip"))?;
    Ok((ip, port))
}
