#[cfg(unix)]
pub const VENV_BIN: &str = "bin";

#[cfg(windows)]
pub const VENV_BIN: &str = "Scripts";
