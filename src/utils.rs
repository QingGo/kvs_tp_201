use anyhow::anyhow;
use anyhow::Result;
use slog::Drain;

pub fn get_root_logger(process: String) -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().stderr().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("process" => process))
}

pub fn get_last_engine() -> Option<String> {
    Some("kvs".to_string())
}

pub fn get_engine(last_engine: Option<String>, config_engine: Option<String>) -> Result<String> {
    let engine: String;
    if let Some(last_engine) = last_engine {
        if let Some(config_engine) = config_engine {
            if config_engine != last_engine {
                return Err(anyhow!("engine not match"));
            } else {
                engine = config_engine;
            }
        } else {
            engine = last_engine;
        }
    } else {
        engine = "kvs".to_string();
    }
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
