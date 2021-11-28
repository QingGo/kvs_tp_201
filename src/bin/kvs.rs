#![feature(backtrace)]
use std::{backtrace::Backtrace};

use clap::Parser;
use kvs::{KvsError, Result, get_engine_by_name};

#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "QingGo")]
struct Opts {
    #[clap(short('V'))]
    // 更改了默认 -v 的行为
    version: bool,
    #[clap(short('e'), default_value("kvs"))]
    engine_name: String,
    command: String,
    key: String,
    value: Option<String>,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let db = get_engine_by_name(opts.engine_name.as_str());

    match opts.command.as_str() {
        "get" => {
            if let Some(value) = opts.value {
                return Err(KvsError::UnexpectedCommand {
                    command: format!("{:?}", value),
                    backtrace: Backtrace::force_capture(),
                });
            }
            let record = db.get(opts.key)?;
            record
                .map(|r| println!("{}", r))
                .unwrap_or_else(|| println!("Key not found"));
        }
        "set" => {
            db.set(opts.key, opts.value.unwrap())?;
        }
        "rm" => {
            db.remove(opts.key).map_err(|err| {
                // If the key does not exist, it prints "Key not found", and exits with a non-zero error code
                if let KvsError::KeyNotFound {
                    key: _,
                    backtrace: _,
                } = err
                {
                    println!("Key not found");
                }
                err
            })?;
        }
        _ => {
            println!("Unknown command: {}", opts.command);
            panic!();
        }
    }

    Ok(())
}
