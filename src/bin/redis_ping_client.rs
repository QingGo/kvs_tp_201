#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;
#[macro_use]
extern crate lazy_static;

use anyhow::Result;
use kvs::RESP;
use slog::Drain;
use std::io;
use std::io::prelude::*;
use std::net::TcpStream;

lazy_static! {
    static ref ROOT_LOGGER: slog::Logger = get_root_logger();
}

fn get_root_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("process" => "redis_ping_client"))
}

// can not connect to real redis server for unknown reason
fn main() -> Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6379")?;
    let mut buf = [0; 1024];
    loop {
        print!("redis> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        info!(ROOT_LOGGER, "recv input"; "input" => &input);
        let encoded_input = encode_input(&input);
        info!(ROOT_LOGGER, "send request"; "request" => &encoded_input);
        stream.write_all(encoded_input.as_bytes())?;
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        info!(ROOT_LOGGER, "recv response"; "response" => &String::from_utf8_lossy(&buf[..n]).to_string());
        let resp: RESP = serde_json::from_slice(&buf[..n])?;
        match resp {
            RESP::Error(err) => println!("{}", err),
            RESP::SimpleString(s) => println!("{}", s),
            _ => println!("unexpect resp: {:?}", resp),
        }
    }
    Ok(())
}

fn encode_input(input: &str) -> String {
    let commands_string: Vec<_> = input.trim().split_whitespace().collect();
    let commands = Vec::from_iter(
        commands_string
            .iter()
            .map(|s| RESP::BulkString(s.to_string())),
    );
    let input = RESP::Array(commands);
    serde_json::to_string(&input)
        .unwrap()
        .trim_matches('"')
        .to_string()
}
