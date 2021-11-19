use std::collections::HashMap;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use kvs::{KvStore, KvsEngine, SledKvsEngine};
use kvs::Result;
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
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(20));
    for engine_name in vec!["kvs", "sled"] {
        let mut rng = SmallRng::seed_from_u64(0);
        group.bench_with_input(engine_name, engine_name, |b, engine_name| {
            b.iter_batched(
                || {
                    let temp_dir = TempDir::new().unwrap();
                    let mut kv_pair = Vec::new();
                    for _ in 0..100 {
                        let k = get_random_ascii_string_by_rng(&mut rng, 10000);
                        let v = get_random_ascii_string_by_rng(&mut rng, 10000);
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
    group.sample_size(20);
    for engine_name in vec!["kvs", "sled"] {
        let mut rng = SmallRng::seed_from_u64(0);
        group.bench_with_input(engine_name, engine_name, |b, engine_name| {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = get_engine_by_name(engine_name, temp_dir.path());
            let mut kv_pair = HashMap::new();
            for _ in 0..1000 {
                let k = get_random_ascii_string_by_rng(&mut rng, 10000);
                let v = get_random_ascii_string_by_rng(&mut rng, 10000);
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

criterion_group!(benches, write_bench, read_bench);
criterion_main!(benches);
