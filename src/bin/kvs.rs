use clap::Parser;
use kvs::{KvStore, KvsError, Result};

#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "QingGo")]
struct Opts {
    #[clap(short('V'))]
    // 更改了默认 -v 的行为
    version: bool,
    command: String,
    key: String,
    value: Option<String>,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let mut db = KvStore::open(".")?;
    // db.set("333".to_string(),"3333".to_string())?;
    match opts.command.as_str() {
        "get" => {
            if let Some(value) = opts.value {
                Err(KvsError::UnexpectedCommand(format!("{:?}", value)))?;
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
                if let KvsError::KeyNotFound(_) = err {
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
