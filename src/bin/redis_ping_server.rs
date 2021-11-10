#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;
#[macro_use]
extern crate lazy_static;

use anyhow::Result;
use kvs::RESP;
use slog::Drain;
use std::io::prelude::*;
use std::net::TcpListener;
use std::thread::JoinHandle;

lazy_static! {
    static ref ROOT_LOGGER: slog::Logger = get_root_logger();
}

fn get_root_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("process" => "redis_ping_server"))
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;
    loop {
        let (stream, _) = listener.accept()?;
        let mut stream = stream.try_clone()?;
        let _: JoinHandle<Result<()>> = std::thread::spawn(move || {
            let mut buf = [0; 1024];
            loop {
                let n = stream.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                let request_string = &String::from_utf8_lossy(&buf[..n]).to_string();
                info!(ROOT_LOGGER, "recv request"; "request" => &request_string);
                let response = handler(&buf[..n]).unwrap_or_else(|_| {
                    RESP::Error(format!("ERR unknown command: {}", request_string))
                });
                let response_string = serde_json::to_string(&response)?;
                info!(ROOT_LOGGER, "send response"; "response" => &response_string);
                stream.write_all(response_string.as_bytes())?;
            }
            Ok(())
        });
    }
}

fn handler(input: &[u8]) -> Result<RESP> {
    let command_string = String::from_utf8_lossy(input);
    let command: RESP = serde_json::from_str(&command_string)?;
    let response = match command {
        RESP::Array(ref array) => {
            if array.len() == 1 && array[0] == RESP::BulkString("PING".to_string()) {
                RESP::SimpleString("PONG".to_string())
            } else if array.len() == 2 && array[0] == RESP::BulkString("PING".to_string()) {
                (array[1]).clone()
            } else {
                RESP::Error("ERR unknown command".to_string())
            }
        }
        _ => RESP::Error("ERR wrong number of arguments for 'ping' command".to_string()),
    };
    Ok(response)
}
