use clap::Parser;
use kvs::KvStore;

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

fn main() {
    let opts: Opts = Opts::parse();
    let _db = KvStore::new();
    match opts.command.as_str() {
        "get" => {
            panic!("unimplemented");
            // db.get(opts.key).unwrap();
        }
        "set" => {
            panic!("unimplemented");
            // db.set(opts.key, opts.value.unwrap()).unwrap();
        }
        "rm" => {
            panic!("unimplemented");
            // db.remove(opts.key);
        }
        _ => {
            println!("Unknown command: {}", opts.command);
            panic!();
        }
    }
}
