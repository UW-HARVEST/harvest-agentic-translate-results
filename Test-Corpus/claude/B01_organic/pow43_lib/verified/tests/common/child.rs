//! Out-of-process caller used for inputs whose C behaviour is undefined
//! (table index outside `g_pow43[0..145]`), where the call may fault.
//!
//! The parent test re-executes the test binary with `POW43_CHILD_IMPL` /
//! `POW43_CHILD_X` set; the child performs a single `pow43(x)` call in the
//! requested library and prints `RESULT=<hex bits>`. This lets the tests
//! compare *termination behaviour* (normal exit / signal) as well as the value,
//! without a fault taking down the whole test run.

#![allow(dead_code)]

use std::ffi::c_int;
use std::process::Command;

use crate::common::impls;

pub const CHILD_IMPL_ENV: &str = "POW43_CHILD_IMPL";
pub const CHILD_X_ENV: &str = "POW43_CHILD_X";

/// Which implementation the child should exercise.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Which {
    C,
    Rust,
}

impl Which {
    fn as_str(self) -> &'static str {
        match self {
            Which::C => "c",
            Which::Rust => "rust",
        }
    }
}

/// Observable outcome of one out-of-process `pow43` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Returned normally with these result bits.
    Value(u32),
    /// Killed by a signal (e.g. 11 = SIGSEGV, 4 = SIGILL, 6 = SIGABRT).
    Signal(i32),
    /// Exited with a non-zero status and no usable result.
    Failed(i32),
}

impl Outcome {
    /// Coarse classification: only "returned a value" vs "died", used when the
    /// exact fault address / signal is not something a translation can control.
    pub fn kind(&self) -> &'static str {
        match self {
            Outcome::Value(_) => "value",
            Outcome::Signal(_) => "signal",
            Outcome::Failed(_) => "failed",
        }
    }
}

/// If this process was spawned as a child worker, perform the call and exit.
/// Returns `false` for a normal (parent) test run.
pub fn run_if_child() -> bool {
    let (Ok(which), Ok(x)) = (
        std::env::var(CHILD_IMPL_ENV),
        std::env::var(CHILD_X_ENV),
    ) else {
        return false;
    };
    let x: c_int = x.parse().expect("POW43_CHILD_X must be an i32");
    let i = impls();
    let f = match which.as_str() {
        "c" => i.c_pow43,
        "rust" => i.rust_pow43,
        other => panic!("unknown POW43_CHILD_IMPL={other}"),
    };
    let v = unsafe { f(x) };
    println!("RESULT={:08x}", v.to_bits());
    true
}

/// Calls `pow43(x)` in the given implementation in a fresh child process.
pub fn call_isolated(which: Which, x: c_int) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .arg("child_worker")
        .arg("--exact")
        .arg("--ignored")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_IMPL_ENV, which.as_str())
        .env(CHILD_X_ENV, x.to_string())
        .env("C_POW43_SO", &impls().c_path)
        .env("RUST_POW43_SO", &impls().rust_path)
        .output()
        .expect("spawn child worker");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // libtest's `--nocapture` prints "test child_worker ... " without a newline
    // first, so the marker can appear in the middle of a line.
    if let Some(pos) = stdout.find("RESULT=") {
        let hex: String = stdout[pos + "RESULT=".len()..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        let bits = u32::from_str_radix(&hex, 16).expect("parse RESULT bits");
        return Outcome::Value(bits);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = out.status.signal() {
            return Outcome::Signal(sig);
        }
    }
    Outcome::Failed(out.status.code().unwrap_or(-1))
}
