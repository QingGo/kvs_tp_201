use super::error::Result;
use super::sled_engine::SledKvsEngine;
use slog::Logger;
use std::io::prelude::*;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use super::engine::KvsEngine;
use super::store::KvStore;

use super::protocol::{Command, Response};

pub struct KvsServer<E: KvsEngine> {
    logger: Logger,
    listener: TcpListener,
    engine: E,
}

impl<E: KvsEngine> KvsServer<E> {
    pub fn new(ip_port: (std::net::IpAddr, u16), engine: E, logger: Logger) -> Result<Self> {
        // let (ip, port) = ip_port;
        let listener = TcpListener::bind(ip_port)?;
        info!(logger, "Listening on"; "addr" => format!("{:?}", ip_port));
        Ok(KvsServer {
            logger,
            listener,
            engine,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut buf = vec![0; 1024];
        loop {
            let (stream, _) = self.listener.accept()?;
            loop {
                let mut stream = stream.try_clone()?;
                let n = stream.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                let command: Command = serde_json::from_slice(&buf[..n])?;
                debug!(self.logger, "recv request"; "request" => format!("{:?}", command));
                let response = match command {
                    // log error but not stop server
                    Command::Get(key) => {
                        let value = self.engine.get(key)?;
                        Response::Success(value)
                    }
                    Command::Set(key, value) => {
                        self.engine.set(key, value)?;
                        Response::Success(None)
                    }
                    Command::Remove(key) => match self.engine.remove(key) {
                        Ok(()) => Response::Success(None),
                        // Err(KvsError::KeyNotFound{key: _, backtrace: _}) => Response::Error("Key not foundddd".to_string()),
                        Err(e) => Response::Error(e.to_string()),
                    },
                };
                let response_serialized = serde_json::to_string(&response)?;
                debug!(self.logger, "send response"; "response" => response_serialized.to_string());
                stream.write_all(response_serialized.as_bytes())?;
            }
        }
    }
}
