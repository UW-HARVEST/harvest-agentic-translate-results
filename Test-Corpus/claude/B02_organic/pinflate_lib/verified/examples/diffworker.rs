//! Crash-isolated `pinflate` runner.
//!
//! The C reference library is built **with asserts enabled** (see
//! `c_src/CMakeLists.txt`: no `CMAKE_BUILD_TYPE`, hence no `-DNDEBUG`, hence
//! `U __assert_fail` in `nm -D`), so hostile input legitimately makes it
//! `abort()`. It can also loop forever (length symbols 286/287 have
//! `cp_len_base == 0`). Neither is something a test process can survive, so all
//! differential comparisons are funnelled through this worker: one process per
//! library, restarted by the parent whenever a case kills it.
//!
//! Protocol, one case per line on stdin:
//!
//! ```text
//! <hexdata> <in_len> <out_size> <in_off> <out_off> <in_pad> <out_pad> <null_in> <null_out> <err_preset> <tables>
//! ```
//!
//! and one reply per line on stdout, prefixed so it cannot be confused with
//! anything the loaded library writes to stdout:
//!
//! ```text
//! #R# R <ret> <err-hex|-> <out-hex>
//! ```

#[path = "../tests/common/shared.rs"]
mod shared;

use shared::{Case, Lib, Outcome};
use std::io::{BufRead, Write};

unsafe extern "C" {
    /// Bounds every case so a non-terminating `pinflate` shows up as
    /// `Signal(14)` for both libraries instead of wedging the test run.
    fn alarm(seconds: u32) -> u32;
}

fn main() {
    let so = std::env::args().nth(1).expect("usage: diffworker <path-to-.so>");
    let timeout: u32 = std::env::var("DIFFWORKER_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let lib = Lib::open(&so);

    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.is_empty() {
            continue;
        }
        let case = Case::decode(&line);
        unsafe { alarm(timeout) };
        let outcome = lib.run(&case);
        unsafe { alarm(0) };
        let Outcome::Ret { .. } = outcome else { unreachable!() };
        // note: an aborting case never reaches this point -- the parent detects
        // the worker's death and scrapes the assert diagnostic from stderr.
        if writeln!(out, "#R# {}", outcome.encode()).is_err() {
            break;
        }
        if out.flush().is_err() {
            break;
        }
    }
}
