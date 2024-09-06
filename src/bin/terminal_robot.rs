use actix::prelude::*;
use std::env;
use std::io;
use tp2::common::log::{LogLevel, Logger};
use tp2::structures::robot::Robot;

fn parsed_args() -> usize {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        Logger.log(LogLevel::Error, "Uso: cargo run --bin terminal_robot <ID>");
        std::process::exit(1);
    }
    let id: usize = match args[1].parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid id");
            std::process::exit(1);
        }
    };
    id
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    let id = parsed_args();
    Robot::new(id)?.start();
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    Ok(())
}
