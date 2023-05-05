use std::process;

use crate::utils::QuietExit;

mod bootstrap;
mod cli;
mod config;
mod installer;
mod lock;
mod piptools;
mod pyproject;
mod sources;
mod sync;
mod utils;

pub fn main() -> Result<(), anyhow::Error> {
    match cli::execute() {
        Ok(()) => Ok(()),
        Err(err) => {
            if let Some(QuietExit(code)) = err.downcast_ref() {
                process::exit(*code);
            } else {
                Err(err)
            }
        }
    }
}
