use crate::utils::get_root_logger;
use crate::{KvStore, SledKvsEngine};

use super::super::thread_pool::*;
use super::error::Result;
use slog::Logger;
use std::io::{self, prelude::*};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

use super::engine::KvsEngine;

use super::protocol::{Command, Response};

pub trait IKvsServer {
    fn run(&self) -> Result<()>;
    fn close(&self);
}

pub struct KvsServer<E: KvsEngine, T: ThreadPool> {
    logger: Logger,
    listener: TcpListener,
    engine: E,
    pool: T,
    is_close: Arc<AtomicBool>,
    connetion_num: Arc<AtomicUsize>,
}

pub fn get_kvs_server_by_config<E: KvsEngine, T: ThreadPool>(
    num_thread: u32,
    ip_port: (std::net::IpAddr, u16),
) -> KvsServer<E, T> {
    let root_logger: slog::Logger = get_root_logger("kvs-server".to_string());
    let pool = T::new(num_thread).unwrap();
    KvsServer::new(ip_port, E::new().unwrap(), pool, root_logger).unwrap()
}

pub fn get_kvs_client_by_config_dyn(
    num_thread: u32,
    pool_name: &str,
    engine_name: &str,
    ip_port: (std::net::IpAddr, u16),
) -> Arc<RwLock<dyn IKvsServer>> {
    match (pool_name, engine_name) {
        ("shared_queue_pool", "kvs") => Arc::new(RwLock::new(get_kvs_server_by_config::<
            KvStore,
            SharedQueueThreadPool,
        >(num_thread, ip_port))),
        ("rayon", "kvs") => Arc::new(RwLock::new(get_kvs_server_by_config::<
            KvStore,
            RayonThreadPool,
        >(num_thread, ip_port))),
        ("rayon", "sled") => Arc::new(RwLock::new(get_kvs_server_by_config::<
            SledKvsEngine,
            RayonThreadPool,
        >(num_thread, ip_port))),
        _ => panic!("Unknown config"),
    }
}

impl<E: KvsEngine, T: ThreadPool> IKvsServer for KvsServer<E, T> {
    fn run(&self) -> Result<()> {
        // if not using non-blocking IO, then thread maybe be block here and don't know server should be closed
        self.listener.set_nonblocking(true)?;
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    // stream will be blocking, or if it is not ready
                    // of there will be a Resource temporarily unavailable error when read buffer
                    stream.set_nonblocking(false)?;
                    let engine = self.engine.clone();
                    let logger = self.logger.clone();
                    let is_close = self.is_close.clone();
                    let connetion_num = self.connetion_num.clone();
                    connetion_num.fetch_add(1, Ordering::SeqCst);
                    self.pool.spawn(move || {
                        if let Err(e) = serve(logger.clone(), engine, stream, is_close) {
                            error!(logger, "Error serving client"; "err" => format!("{:?}", e));
                        }
                        connetion_num.fetch_sub(1, Ordering::SeqCst);
                    });
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Decide if we should exit
                    if self.is_close.load(Ordering::SeqCst) {
                        info!(self.logger, "receive closeing server signal");
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("encountered IO error: {}", e),
            }
        }
        Ok(())
    }
    fn close(&self) {
        info!(self.logger, "begin closeing server");
        self.is_close.store(true, Ordering::SeqCst);
        while self.connetion_num.load(Ordering::SeqCst) != 0 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        info!(self.logger, "closeing server successfully");
    }
}

impl<E: KvsEngine, T: ThreadPool> KvsServer<E, T> {
    pub fn new(
        ip_port: (std::net::IpAddr, u16),
        engine: E,
        pool: T,
        logger: Logger,
    ) -> Result<Self> {
        // let (ip, port) = ip_port;
        let listener = TcpListener::bind(ip_port)?;
        info!(logger, "Listening on"; "addr" => format!("{:?}", ip_port));
        // let pool = T::new(num_cpus::get() as u32)?;
        Ok(KvsServer {
            logger,
            listener,
            engine,
            pool,
            is_close: Arc::new(AtomicBool::new(false)),
            connetion_num: Arc::new(AtomicUsize::new(0)),
        })
    }
}

pub fn serve(
    logger: Logger,
    engine: impl KvsEngine,
    stream: TcpStream,
    is_close: Arc<AtomicBool>,
) -> Result<()> {
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
        if is_close.load(Ordering::SeqCst) {
            stream.shutdown(Shutdown::Both)?;
            break;
        }
    }
    Ok(())
}
