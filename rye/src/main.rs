use std::sync::atomic::{AtomicBool, Ordering};

use crate::utils::panic::trap_bad_pipe;
use crate::utils::QuietExit;

#[macro_use]
mod tui;

mod bootstrap;
mod cli;
mod config;
mod consts;
mod installer;
mod lock;
mod platform;
mod pyproject;
mod sources;
mod sync;
mod utils;
mod uv;

static SHOW_CONTINUE_PROMPT: AtomicBool = AtomicBool::new(false);
static DISABLE_CTRLC_HANDLER: AtomicBool = AtomicBool::new(false);

/// Changes the shutdown behavior to request a continue prompt.
pub fn request_continue_prompt() {
    SHOW_CONTINUE_PROMPT.store(true, Ordering::Relaxed);
}

/// Disables the ctrl-c handler
pub fn disable_ctrlc_handler() {
    DISABLE_CTRLC_HANDLER.store(true, Ordering::Relaxed);
}

pub fn main() {
    crate::utils::panic::set_panic_hook();

    ctrlc::set_handler(move || {
        if !DISABLE_CTRLC_HANDLER.load(Ordering::Relaxed) {
            let term = console::Term::stderr();
            term.show_cursor().ok();
            term.flush().ok();
            std::process::exit(if cfg!(windows) {
                0xC000013Au32 as i32
            } else {
                130
            });
        }
    })
    .unwrap();

    trap_bad_pipe(|| {
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
        status
    });
}
