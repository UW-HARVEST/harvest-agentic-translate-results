// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#[path = "logic.rs"]
mod logic;

use std::ffi::OsString;
use std::process::ExitCode;

fn arg_bytes(arg: &OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().into_owned().into_bytes()
    }
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, whereas a C
/// program inherits the default disposition. Restore `SIG_DFL` so that writing
/// to a closed pipe terminates this process with `SIGPIPE`, exactly like the C
/// program does.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/*
  Maintain a running total using a static variable
 */
fn main() -> ExitCode {
    restore_default_sigpipe();

    let argv: Vec<OsString> = std::env::args_os().collect();
    let argc = argv.len() as i32;

    let status = logic::run(argc, || arg_bytes(&argv[1]));

    ExitCode::from(status as u8)
}
