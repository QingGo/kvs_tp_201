use anyhow::Result;
use slog::Logger;
use std::io::prelude::*;
use std::net::TcpListener;

pub struct KvsServer {
    logger: Logger,
    listener: TcpListener,
}

impl KvsServer {
    pub fn new(ip_port: (std::net::IpAddr, u16), logger: Logger) -> Result<Self> {
        // let (ip, port) = ip_port;
        let listener = TcpListener::bind(ip_port)?;
        info!(logger, "Listening on"; "addr" => format!("{:?}", ip_port));
        Ok(KvsServer {
            logger,
            listener,
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
                let request_string = &String::from_utf8_lossy(&buf[..n]).to_string();
                debug!(self.logger, "recv request"; "request" => &request_string);
                debug!(self.logger, "send response"; "response" => &request_string);
                stream.write_all(request_string.as_bytes())?;
            }
        }
    }
}
