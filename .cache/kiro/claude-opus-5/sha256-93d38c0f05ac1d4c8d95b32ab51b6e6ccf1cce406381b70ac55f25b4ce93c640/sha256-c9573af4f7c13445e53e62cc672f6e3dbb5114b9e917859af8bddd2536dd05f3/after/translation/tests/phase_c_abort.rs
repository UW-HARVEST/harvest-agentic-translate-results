//! Phase C rows 1–2 — the `assert(string != NULL)` paths.
//!
//! Both implementations terminate the process, so each call is made in a fresh
//! child process (this same test binary re-executed with a filter) and the two
//! are compared on how they died: same terminating signal, same
//! "did it return normally" answer. The C `.so` is built with asserts LIVE
//! (`nm -D` shows `U __assert_fail`), so the expected outcome is SIGABRT (6).

mod common;

use common::*;
use std::process::{Command, Stdio};

const PROBE_ENV: &str = "W_UTF8_NULL_PROBE";

/// The child half. Does nothing unless `W_UTF8_NULL_PROBE` is set, so it is a
/// no-op during a normal test run.
#[test]
fn child_null_probe() {
    let spec = match std::env::var(PROBE_ENV) {
        Ok(v) => v,
        Err(_) => return,
    };
    let (which, func) = spec.split_once(':').expect("spec is <lib>:<func>");
    let path = match which {
        "c" => c_so_path(),
        "rust" => rust_so_path(),
        other => panic!("unknown lib {other}"),
    };
    let d = Driver::open(&path);
    unsafe {
        match func {
            "drop" => {
                let r = (d.drop_fn)(std::ptr::null());
                println!("RETURNED:{}", if r.is_null() { "null" } else { "nonnull" });
            }
            "filter0" => {
                let r = (d.filter_fn)(std::ptr::null(), 0);
                println!("RETURNED:{}", if r.is_null() { "null" } else { "nonnull" });
            }
            "filter1" => {
                let r = (d.filter_fn)(std::ptr::null(), 1);
                println!("RETURNED:{}", if r.is_null() { "null" } else { "nonnull" });
            }
            "filter255" => {
                let r = (d.filter_fn)(std::ptr::null(), 255);
                println!("RETURNED:{}", if r.is_null() { "null" } else { "nonnull" });
            }
            other => panic!("unknown func {other}"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
    returned: Option<String>,
}

fn probe(which: &str, func: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "child_null_probe", "--nocapture", "--test-threads=1"])
        .env(PROBE_ENV, format!("{which}:{func}"))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("spawn child probe");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // libtest with --nocapture can prefix the line, so search for the marker.
    let returned = stdout.find("RETURNED:").map(|i| {
        let rest = &stdout[i + "RETURNED:".len()..];
        rest.split_whitespace().next().unwrap_or("").to_string()
    });
    use std::os::unix::process::ExitStatusExt;
    Outcome {
        signal: out.status.signal(),
        code: out.status.code(),
        returned,
    }
}

#[test]
fn err01_and_err02_null_pointer_aborts_identically() {
    for func in ["drop", "filter0", "filter1", "filter255"] {
        let c = probe("c", func);
        let r = probe("rust", func);
        assert_eq!(
            c.signal, r.signal,
            "NULL {func}: C died with signal {:?}, Rust with {:?} (C outcome {c:?}, Rust {r:?})",
            c.signal, r.signal
        );
        assert_eq!(
            c.returned, r.returned,
            "NULL {func}: C returned {:?}, Rust {:?}",
            c.returned, r.returned
        );
        // The C `assert` is live, so the expected behaviour is SIGABRT (6) and
        // no value ever returned.
        assert_eq!(
            c.signal,
            Some(6),
            "expected the C build to abort on NULL (asserts are live); got {c:?}"
        );
        assert_eq!(c.returned, None, "C should not have returned a value: {c:?}");
    }
}
