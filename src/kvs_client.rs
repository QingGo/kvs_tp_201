use anyhow::anyhow;
use anyhow::Result;
use slog::Logger;
use std::io::prelude::*;
use std::net::TcpStream;

pub struct KvsClient {
    logger: Logger,
    stream: TcpStream,
}

impl KvsClient {
    pub fn new(ip_port: (std::net::IpAddr, u16), logger: Logger) -> Result<Self> {
        // let (ip, port) = ip_port;
        let stream = std::net::TcpStream::connect(ip_port)?;
        info!(logger, "connect to server"; "addr" => format!("{:?}", ip_port));
        Ok(KvsClient {
            logger,
            stream,
        })
    }

    pub fn send(&mut self, input: &str) -> Result<String> {
        debug!(self.logger, "send request"; "request" => input);
        self.stream.write_all(input.as_bytes())?;
        let mut buf = vec![0; 1024];
        let n = self.stream.read(&mut buf)?;
        if n == 0 {
            return Err(anyhow!("server closed"));
        }
        let output = String::from_utf8_lossy(&buf[..n]).to_string();
        debug!(self.logger, "recv response"; "response" => &output);
        Ok(output)
    }
}
