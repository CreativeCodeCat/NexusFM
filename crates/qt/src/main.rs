//! Starts the Qt window built by `build.rs`. Arguments are passed through to it.

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let status = match Command::new(env!("NFM_QT_BIN"))
        .args(std::env::args_os().skip(1))
        .status()
    {
        Ok(status) => status,
        Err(err) => {
            eprintln!("nexusfm-qt: cannot start the Qt window: {err}");
            return ExitCode::FAILURE;
        }
    };

    match status.code() {
        Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        None => ExitCode::FAILURE,
    }
}
