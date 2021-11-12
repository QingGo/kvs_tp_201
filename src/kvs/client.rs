use super::error::Result;
use super::protocol::{Command, Response};
use anyhow::anyhow;
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
        Ok(KvsClient { logger, stream })
    }

    pub fn send(&mut self, input: &Command) -> Result<Option<String>> {
        let serialized_input = serde_json::to_string(&input)?;
        debug!(self.logger, "send request"; "request" => &serialized_input);
        self.stream.write_all(serialized_input.as_bytes())?;
        let mut buf = vec![0; 1024];
        let n = self.stream.read(&mut buf)?;
        if n == 0 {
            return Err(anyhow!("server closed").into());
        }
        let output: Response = serde_json::from_slice(&buf[..n])?;
        debug!(self.logger, "recv response"; "response" => format!("{:?}", &output));
        match output {
            Response::Success(result) => Ok(result),
            Response::Error(err) => Err(anyhow!(err).into()),
        }
    }
}
