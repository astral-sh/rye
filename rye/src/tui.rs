use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

static ECHO_TO_STDERR: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // use eprintln and println so that tests can still intercept this
    if ECHO_TO_STDERR.load(Ordering::Relaxed) {
        eprintln!("{}", args);
    } else {
        println!("{}", args);
    }
}

/// Until the guard is dropped, echo goes to stderr.
pub fn redirect_to_stderr(yes: bool) -> RedirectGuard {
    let old = ECHO_TO_STDERR.load(Ordering::Relaxed);
    ECHO_TO_STDERR.store(yes, Ordering::Relaxed);
    RedirectGuard(old)
}

#[must_use]
pub struct RedirectGuard(bool);

impl Drop for RedirectGuard {
    fn drop(&mut self) {
        ECHO_TO_STDERR.store(self.0, Ordering::Relaxed);
    }
}

/// Echo a line to the output stream (usually stdout).
macro_rules! echo {
    () => {
        $crate::tui::_print(format_args!(""))
    };
    (if verbose $out:expr, $($arg:tt)+) => {
        match $out {
            $crate::utils::CommandOutput::Verbose => {
                $crate::tui::_print(format_args!($($arg)*))
            }
            _ => {}
        }
    };
    (if $out:expr, $($arg:tt)+) => {
        match $out {
            $crate::utils::CommandOutput::Normal | $crate::utils::CommandOutput::Verbose => {
                $crate::tui::_print(format_args!($($arg)*))
            }
            _ => {}
        }
    };
    ($($arg:tt)+) => {
        // TODO: this is bloaty, but this way capturing of outputs
        // for stdout works in tests still.
        $crate::tui::_print(format_args!($($arg)*))
    };
}

/// Like echo but always goes to stderr.
macro_rules! elog {
    ($($arg:tt)*) => { eprintln!($($arg)*) }
}

/// Emits a warning
macro_rules! warn {
    ($($arg:tt)+) => {
        elog!(
            "{} {}",
            console::style("warning:").yellow().bold(),
            format_args!($($arg)*)
        )
    }
}

/// Logs errors
macro_rules! error {
    ($($arg:tt)+) => {
        elog!(
            "{} {}",
            console::style("error:").red().bold(),
            format_args!($($arg)*)
        )
    }
}
