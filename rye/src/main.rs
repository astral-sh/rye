use std::process;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::utils::QuietExit;

#[macro_use]
mod tui;

mod bootstrap;
mod cli;
mod config;
mod consts;
mod installer;
mod lock;
mod piptools;
mod platform;
mod pyproject;
mod sources;
mod sync;
mod utils;

static SHOW_CONTINUE_PROMPT: AtomicBool = AtomicBool::new(false);

/// Changes the shutdown behavior to request a continue prompt.
pub fn request_continue_prompt() {
    SHOW_CONTINUE_PROMPT.store(true, Ordering::Relaxed);
}

pub fn main() {
    let result = cli::execute();
    let status = match result {
        Ok(()) => 0,
        Err(err) => {
            if let Some(err) = err.downcast_ref::<clap::Error>() {
                err.print().ok();
                err.exit_code()
            } else if let Some(QuietExit(code)) = err.downcast_ref() {
                *code
            } else {
                error!("{:?}", err);
                1
            }
        }
    };

    if SHOW_CONTINUE_PROMPT.load(Ordering::Relaxed) {
        echo!("Press any key to continue");
        console::Term::buffered_stderr().read_key().ok();
    }

    process::exit(status);
}
