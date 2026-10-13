//! The only place cadetta-cli reads the process environment.

/// Name of the environment variable that selects the output format.
pub(crate) const FORMAT_ENV: &str = "CADETTA_FORMAT";

/// Reads `CADETTA_FORMAT`: `None` when unset or not valid unicode.
pub(crate) fn format_var() -> Option<String> {
    std::env::var(FORMAT_ENV).ok()
}

/// Reads `HOME`: `None` when unset (used to find `~/.gitconfig` for `ack`).
pub(crate) fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}
