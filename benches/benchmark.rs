#![allow(dead_code)]
// #![allow(unused_variables)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;

use criterion::measurement::WallTime;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkGroup, Criterion};
use kvs::thread_pool::{RayonThreadPool, SharedQueueThreadPool, ThreadPool};
use kvs::utils::*;
use kvs::{Command, KvsClient, KvsServer, Result};
use kvs::{KvStore, KvsEngine, SledKvsEngine};
use rand::prelude::*;
use rand::{rngs::SmallRng, SeedableRng};
use tempfile::TempDir;

/// # if you want to run the benchmarks on an otherwise unloaded machine
/// rustup target add x86_64-unknown-linux-gnu
/// # get linux linker toolchain
/// brew tap SergioBenitez/osxct
/// brew install x86_64-unknown-linux-gnu
/// CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc cargo bench --no-run --target x86_64-unknown-linux-gnu
/// copy to binary in target/x86_64-unknown-linux-gnu/release/deps/benchmark-xxx and run benchmark-xxx --bench

fn get_random_ascii_string_by_rng(rng: &mut SmallRng, size: usize) -> String {
    (0..size)
        .map(|_| {
            const CHARSET: &[u8] =
                b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
            const LEN: usize = CHARSET.len();
            let idx = rng.gen_range(0..LEN);
            CHARSET[idx] as char
        })
        .collect::<String>()
}

enum EngineEnum {
    KvStore(KvStore),
    SledKvsEngine(SledKvsEngine),
}

impl EngineEnum {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        match self {
            EngineEnum::KvStore(engine) => engine.set(key, value),
            EngineEnum::SledKvsEngine(engine) => engine.set(key, value),
        }
    }

    fn get(&mut self, key: String) -> Result<Option<String>> {
        match self {
            EngineEnum::KvStore(engine) => engine.get(key),
            EngineEnum::SledKvsEngine(engine) => engine.get(key),
        }
    }
}

fn get_engine_by_name(name: &str, path: impl Into<PathBuf>) -> EngineEnum {
    match name {
        "kvs" => EngineEnum::KvStore(KvStore::open(path).unwrap()),
        "sled" => EngineEnum::SledKvsEngine(SledKvsEngine::open(path).unwrap()),
        _ => panic!("Unknown engine name"),
    }
}

// With the kvs/sled engine, write 100 values with random keys of length 1-100000 bytes and random values of length 1-100000 bytes.
fn write_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_bench");
    group
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(5));
    for engine_name in vec!["kvs", "sled"] {
        let mut rng = SmallRng::seed_from_u64(0);
        group.bench_with_input(engine_name, engine_name, |b, engine_name| {
            b.iter_batched(
                || {
                    let temp_dir = TempDir::new().unwrap();
                    let mut kv_pair = Vec::new();
                    for _ in 0..100 {
                        let k = get_random_ascii_string_by_rng(&mut rng, 10);
                        let v = get_random_ascii_string_by_rng(&mut rng, 10);
                        kv_pair.push((k, v));
                    }
                    (
                        get_engine_by_name(engine_name, temp_dir.path()),
                        temp_dir,
                        kv_pair,
                    )
                },
                |(mut engine, _temp_dir, kv_pair)| {
                    for (key, value) in kv_pair {
                        engine.set(key, value).unwrap();
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

// With the kvs/sled engine, read 1000 values from previously written keys, with keys and values of random length.
fn read_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_bench");
    group
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(5));
    for engine_name in vec!["kvs", "sled"] {
        let mut rng = SmallRng::seed_from_u64(0);
        group.bench_with_input(engine_name, engine_name, |b, engine_name| {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = get_engine_by_name(engine_name, temp_dir.path());
            let mut kv_pair = HashMap::new();
            for _ in 0..100 {
                let k = get_random_ascii_string_by_rng(&mut rng, 10);
                let v = get_random_ascii_string_by_rng(&mut rng, 10);
                kv_pair.insert(k, v);
            }
            for (key, value) in kv_pair.iter() {
                engine.set(key.clone(), value.clone()).unwrap();
            }
            drop(engine);
            let mut engine_new = get_engine_by_name(engine_name, temp_dir.path());
            b.iter(move || {
                for (key, value) in &kv_pair {
                    assert_eq!(engine_new.get(key.to_string()).unwrap().unwrap(), *value);
                }
            })
        });
    }
    group.finish();
}

fn get_kvs_server_by_config<E: KvsEngine, T: ThreadPool>(num_thread: u32) -> KvsServer<E, T> {
    let ip_port = parse_ip_port("127.0.0.1:4000").unwrap();
    let root_logger: slog::Logger = get_root_logger("kvs-server".to_string());
    let pool = T::new(num_thread).unwrap();
    KvsServer::new(ip_port, E::new().unwrap(), pool, root_logger).unwrap()
}

fn get_kvs_client() -> KvsClient {
    let ip_port = parse_ip_port("127.0.0.1:4000").unwrap();
    let root_logger: slog::Logger = get_root_logger("kvs-client".to_string());
    KvsClient::new(ip_port, root_logger).unwrap()
}

fn run_write_bench(g: &mut BenchmarkGroup<WallTime>, name: &str) {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut client = get_kvs_client();
    g.bench_function(name, |b| {
        let mut kv_pair = Vec::new();
        for _ in 0..100 {
            let k = get_random_ascii_string_by_rng(&mut rng, 10);
            let v = k.clone();
            kv_pair.push((k, v));
        }
        b.iter(|| {
            for (key, value) in &kv_pair {
                client
                    .send(&Command::Set(key.clone(), value.clone()))
                    .unwrap();
            }
        })
    });
}

fn write_queued_kvstore(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_queued_kvstore");
    group
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(5));
    for config in vec![
        ("shared_queue_pool", "kvs"),
        ("rayon", "kvs"),
        ("rayon", "sled"),
    ] {
        for num_thread in vec![1, 2, 4, 8, 16, 32] {
            match config {
                ("shared_queue_pool", "kvs") => {
                    // run server on another thread on benchtest will be block here
                    // to-do: how to stop the server
                    thread::spawn(move || {
                        get_kvs_server_by_config::<KvStore, SharedQueueThreadPool>(num_thread)
                            .run()
                            .unwrap()
                    });
                    // wait for server to start
                    thread::sleep(std::time::Duration::from_secs(3));
                    run_write_bench(
                        &mut group,
                        &format!("{}_{}_{}", config.0, config.1, num_thread),
                    );
                }
                ("rayon", "kvs") => {
                    thread::spawn(move || {
                        get_kvs_server_by_config::<KvStore, RayonThreadPool>(num_thread)
                            .run()
                            .unwrap()
                    });
                    // wait for server to start
                    thread::sleep(std::time::Duration::from_secs(3));
                    run_write_bench(
                        &mut group,
                        &format!("{}_{}_{}", config.0, config.1, num_thread),
                    )
                }
                ("rayon", "sled") => {
                    thread::spawn(move || {
                        get_kvs_server_by_config::<SledKvsEngine, RayonThreadPool>(num_thread)
                            .run()
                            .unwrap()
                    });
                    // wait for server to start
                    thread::sleep(std::time::Duration::from_secs(3));
                    run_write_bench(
                        &mut group,
                        &format!("{}_{}_{}", config.0, config.1, num_thread),
                    )
                }
                _ => {}
            }
        }
    }
}

// fn read_queued_kvstore(c: &mut Criterion) {
//     let mut group = c.benchmark_group("read_queued_kvstore");
// }

criterion_group!(benches, write_queued_kvstore);
criterion_main!(benches);
