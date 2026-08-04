use std::io::{self, Write};
use std::process::ExitCode;

fn helloworld() -> i32 {
    io::stdout().write_all(b"Hello World!\n").unwrap();
    0
}

fn main() -> ExitCode {
    ExitCode::from(helloworld() as u8)
}
