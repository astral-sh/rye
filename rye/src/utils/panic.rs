use std::any::Any;
use std::{panic, process};

fn is_bad_pipe(payload: &dyn Any) -> bool {
    payload
        .downcast_ref::<String>()
        .map_or(false, |x| x.contains("failed printing to stdout: "))
}

/// Registers a panic hook that hides stdout printing failures.
pub fn set_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if !is_bad_pipe(info.payload()) {
            default_hook(info)
        }
    }));
}

/// Catches down panics that are caused by bad pipe errors.
pub fn trap_bad_pipe<F: FnOnce() -> i32 + Send + Sync>(f: F) -> ! {
    process::exit(match panic::catch_unwind(panic::AssertUnwindSafe(f)) {
        Ok(status) => status,
        Err(panic) => {
            if is_bad_pipe(&panic) {
                1
            } else {
                panic::resume_unwind(panic);
            }
        }
    });
}
