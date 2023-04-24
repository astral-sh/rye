mod bootstrap;
mod cli;
mod config;
mod installer;
mod lock;
mod pyproject;
mod sources;
mod sync;
mod utils;

pub fn main() -> Result<(), anyhow::Error> {
    cli::execute()
}
