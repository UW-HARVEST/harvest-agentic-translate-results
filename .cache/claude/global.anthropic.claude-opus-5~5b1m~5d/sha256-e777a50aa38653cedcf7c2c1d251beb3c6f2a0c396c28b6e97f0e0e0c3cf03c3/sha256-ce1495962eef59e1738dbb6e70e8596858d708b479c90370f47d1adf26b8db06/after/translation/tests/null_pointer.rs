// Phase C -- ERRORS.md rows E6 and E21: the two entry points that dereference a
// caller-supplied pointer with NO null check.
//
//   add_node(..., name = NULL, ...)   -> strncpy(dst, NULL, 49)   (lib.c:56)
//   process_string(str = NULL)        -> `if (*str)`              (lib.c:102)
//
// Both kill the process, so the differential comparison is done out-of-process:
// this test binary re-executes itself (an `#[ignore]`d helper test) once per
// library and asserts the two children die in exactly the same way -- same
// signal, same exit code. "Both crashed somehow" is not enough; the signal
// numbers must be equal.

mod common;
use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
struct Death {
    code: Option<i32>,
    signal: Option<i32>,
}

fn run_child(target: &str) -> Death {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "zz_crash_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HARVEST_CRASH_TARGET", target)
        .output()
        .expect("spawn child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("SURVIVED"),
        "child {target} did NOT crash on the null dereference:\n{stdout}"
    );
    Death {
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn e6_add_node_null_name_dereferences_null() {
    let c = run_child("c_add_null");
    let r = run_child("rust_add_null");
    assert_eq!(
        c, r,
        "[E6] add_node(name=NULL): C died as {c:?}, Rust died as {r:?}"
    );
    assert_eq!(c.signal, Some(11), "[E6] expected SIGSEGV, got {c:?}");
}

#[test]
fn e21_process_string_null_dereferences_null() {
    let c = run_child("c_ps_null");
    let r = run_child("rust_ps_null");
    assert_eq!(
        c, r,
        "[E21] process_string(NULL): C died as {c:?}, Rust died as {r:?}"
    );
    assert_eq!(c.signal, Some(11), "[E21] expected SIGSEGV, got {c:?}");
}

/// Not a test: the crash payload, run in a child process by the two tests
/// above. Selected with `HARVEST_CRASH_TARGET`.
#[test]
#[ignore = "crash payload; run out-of-process by e6/e21"]
fn zz_crash_child() {
    let target = std::env::var("HARVEST_CRASH_TARGET").unwrap_or_default();
    let p = Pair::fresh();
    let n: i32 = unsafe {
        match target.as_str() {
            "c_add_null" => (p.c.add_node)(1, -1, std::ptr::null(), 1.0),
            "rust_add_null" => (p.rust.add_node)(1, -1, std::ptr::null(), 1.0),
            "c_ps_null" => (p.c.process_string)(std::ptr::null_mut()),
            "rust_ps_null" => (p.rust.process_string)(std::ptr::null_mut()),
            other => panic!("unknown HARVEST_CRASH_TARGET {other:?}"),
        }
    };
    println!("SURVIVED target={target} returned={n}");
}
