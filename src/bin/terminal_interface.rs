use std::env;
use std::io;
use std::process;

use actix::prelude::*;
use tp2::common::log::{LogLevel, Logger};
use tp2::structures::interface::Interface;

type Args = (usize, String);

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        Logger.log(
            LogLevel::Error,
            "Usage: cargo run --bin terminal_interface <ID> <FILE>",
        );
        process::exit(1);
    }
    let id: usize = match args[1].parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid id");
            std::process::exit(1);
        }
    };
    let file = &args[2];
    println!("id {}, file: {}", id, file);
    (id, file.to_string())
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    let (id, file) = parse_args();
    let interface = Interface::new(id, file)?;
    interface.start();
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    Ok(())
}
