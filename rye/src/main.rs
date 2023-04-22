mod bootstrap;
mod cli;
mod config;
mod installer;
mod lock;
mod pyproject;
mod sources;
mod sync;
mod utils;

pub fn main() {
    if let Err(err) = cli::execute() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}
