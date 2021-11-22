use super::super::thread_pool::*;
use super::error::Result;
use slog::Logger;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

use super::engine::KvsEngine;

use super::protocol::{Command, Response};

pub struct KvsServer<E: KvsEngine, T: ThreadPool> {
    logger: Logger,
    listener: TcpListener,
    engine: E,
    pool: T,
}

impl<E: KvsEngine, T: ThreadPool> KvsServer<E, T> {
    pub fn new(ip_port: (std::net::IpAddr, u16), engine: E, logger: Logger) -> Result<Self> {
        // let (ip, port) = ip_port;
        let listener = TcpListener::bind(ip_port)?;
        info!(logger, "Listening on"; "addr" => format!("{:?}", ip_port));
        let pool = T::new(num_cpus::get() as u32)?;
        Ok(KvsServer {
            logger,
            listener,
            engine,
            pool,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream?;
            let engine = self.engine.clone();
            let logger = self.logger.clone();
            self.pool.spawn(move || {
                if let Err(e) = serve(logger.clone(), engine, stream) {
                    error!(logger, "Error serving client"; "err" => format!("{:?}", e));
                }
            });
        }
        Ok(())
    }
}

pub fn serve(logger: Logger, engine: impl KvsEngine, stream: TcpStream) -> Result<()> {
    let mut buf = vec![0; 1024];
    loop {
        let mut stream = stream.try_clone()?;
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let command: Command = serde_json::from_slice(&buf[..n])?;
        debug!(logger, "recv request"; "request" => format!("{:?}", command));
        let response = match command {
            // log error but not stop server
            Command::Get(key) => {
                let value = engine.get(key)?;
                Response::Success(value)
            }
            Command::Set(key, value) => {
                engine.set(key, value)?;
                Response::Success(None)
            }
            Command::Remove(key) => match engine.remove(key) {
                Ok(()) => Response::Success(None),
                // Err(KvsError::KeyNotFound{key: _, backtrace: _}) => Response::Error("Key not foundddd".to_string()),
                Err(e) => Response::Error(e.to_string()),
            },
        };
        let response_serialized = serde_json::to_string(&response)?;
        debug!(logger, "send response"; "response" => response_serialized.to_string());
        stream.write_all(response_serialized.as_bytes())?;
    }
    Ok(())
}
