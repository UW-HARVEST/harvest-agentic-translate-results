use std::io::{self, Write};
use std::process::ExitCode;

fn helloworld() -> i32 {
    let _ = io::stdout().write_all(b"Hello World!\n");
    0
}

fn main() -> ExitCode {
    let status = helloworld();
    ExitCode::from(status as u8)
}
