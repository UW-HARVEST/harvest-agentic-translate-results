//! Phase C (continued) — error paths that terminate the process.
//!
//! ERRORS.md rows 32, 33, 34 describe inputs on which the C library does not
//! *return* anything: it traps. Those cannot be compared in-process, so each
//! case is executed in a fresh child process (this same test binary re-invoked
//! with `DIFF_CRASH=<lib>:<case>`) and the two libraries' **termination
//! signals / exit codes are compared**.

mod common;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// The worker. Only ever does something when `DIFF_CRASH` is set, which the
/// parent tests below arrange for a child process.
#[test]
fn crash_worker() {
    let spec = match std::env::var("DIFF_CRASH") {
        Ok(s) => s,
        Err(_) => return, // normal run: nothing to do
    };
    let (which, case) = spec.split_once(':').expect("DIFF_CRASH=<lib>:<case>");
    let p = common::pair();
    let lib = match which {
        "c" => &p.c,
        "rust" => &p.r,
        other => panic!("unknown lib {other}"),
    };
    lib.reset();
    let out = match case {
        "divide_min_neg1" => unsafe { (lib.divide_op)(i32::MIN, -1, 0, 0) },
        "modulo_min_neg1" => unsafe { (lib.modulo_op)(i32::MIN, -1, 0, 0) },
        "add_null_label" => unsafe { lib.add_raw(1, 2, -1, std::ptr::null()) },
        other => panic!("unknown case {other}"),
    };
    println!("RESULT={out}");
    // make sure the value cannot be optimised away
    std::process::exit(if out == 0x7fff_dead { 3 } else { 0 });
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: String,
}

fn run_case(which: &str, case: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args(["--exact", "crash_worker", "--nocapture", "--test-threads=1"])
        .env("DIFF_CRASH", format!("{which}:{case}"))
        // pass the resolved library paths down so the child agrees with us
        .env("C_SO", common::c_so_path())
        .env("RUST_SO", common::rust_so_path())
        .output()
        .expect("spawn child");
    let text = String::from_utf8_lossy(&out.stdout);
    let result_line = text
        .lines()
        .find(|l| l.starts_with("RESULT="))
        .unwrap_or("")
        .to_string();
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: result_line,
    }
}

fn assert_same(case: &str) {
    let c = run_case("c", case);
    let r = run_case("rust", case);
    assert_eq!(
        c, r,
        "crash/return parity mismatch for {case}\n  C   : {c:?}\n  Rust: {r:?}"
    );
    println!("{case}: both -> {c:?}");
}

// ERRORS.md row 32
#[test]
fn err32_divide_int_min_by_minus_one() {
    let c = run_case("c", "divide_min_neg1");
    assert_eq!(
        c.signal,
        Some(8),
        "C reference must die from SIGFPE, got {c:?}"
    );
    assert_same("divide_min_neg1");
}

// ERRORS.md row 33
#[test]
fn err33_modulo_int_min_by_minus_one() {
    let c = run_case("c", "modulo_min_neg1");
    assert_eq!(
        c.signal,
        Some(8),
        "C reference must die from SIGFPE, got {c:?}"
    );
    assert_same("modulo_min_neg1");
}

// ERRORS.md row 34
#[test]
fn err34_add_tree_node_null_label() {
    let c = run_case("c", "add_null_label");
    assert_eq!(
        c.signal,
        Some(11),
        "C reference must die from SIGSEGV, got {c:?}"
    );
    let r = run_case("rust", "add_null_label");
    assert_eq!(
        r.signal, c.signal,
        "Rust must terminate on the same signal as C\n  C   : {c:?}\n  Rust: {r:?}"
    );
}
