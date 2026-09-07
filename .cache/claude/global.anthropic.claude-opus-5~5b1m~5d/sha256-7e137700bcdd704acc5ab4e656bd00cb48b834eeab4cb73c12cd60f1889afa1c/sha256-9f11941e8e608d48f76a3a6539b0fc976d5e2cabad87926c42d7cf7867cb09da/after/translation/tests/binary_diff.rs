//! `driver` executable differential tests — `CONFIGS.md` rows 28-33 and
//! `ERRORS.md` rows 1-8. stdout, stderr and exit status are compared
//! byte-for-byte (stdout goes to a pipe in both cases, so C's full buffering
//! and Rust's line buffering must still yield identical ordering).

mod common;

use common::*;
use std::process::{Command, Output};

fn run(bin: &std::path::Path, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {} failed: {e}", bin.display()))
}

/// Compare C and Rust `driver` invocations. `stderr` is compared with argv[0]
/// normalised away, since the usage line embeds the program path.
fn diff_args(args: &[&str]) {
    let cb = c_bin();
    let rb = rust_bin();
    let co = run(&cb, args);
    let ro = run(&rb, args);

    assert_eq!(
        String::from_utf8_lossy(&co.stdout),
        String::from_utf8_lossy(&ro.stdout),
        "stdout differs for args {args:?} [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        co.status.code(),
        ro.status.code(),
        "exit status differs for args {args:?} [OP={OP} REPEAT={REPEAT}]"
    );

    let norm = |s: &[u8], bin: &std::path::Path| {
        String::from_utf8_lossy(s).replace(&bin.display().to_string(), "<argv0>")
    };
    assert_eq!(
        norm(&co.stderr, &cb),
        norm(&ro.stderr, &rb),
        "stderr differs for args {args:?} [OP={OP} REPEAT={REPEAT}]"
    );
}

/* ---- CONFIGS.md 28: randomized argv pairs ------------------------- */

#[test]
fn cfg28_binary_random_args() {
    let mut rng = Rng::new();
    for _ in 0..96 {
        let a = rng.i32_mixed().to_string();
        let b = rng.i32_mixed().to_string();
        diff_args(&[&a, &b]);
    }
}

/* ---- CONFIGS.md 29: boundary argv -------------------------------- */

#[test]
fn cfg29_binary_boundary_args() {
    let vals: Vec<String> = EDGE.iter().map(|v| v.to_string()).collect();
    for a in &vals {
        for b in &vals {
            diff_args(&[a, b]);
        }
    }
}

/* ---- CONFIGS.md 30 / ERRORS.md 4-8: atoi rejection surface ------- */

#[test]
fn cfg30_err04_err05_err06_err07_err08_atoi_surface() {
    let weird = [
        // ERRORS.md 4 — no digits at all
        "abc", "", "+", "-", " ", "\t", "--5", "++5", "0x1f", ".5", "e5", "null",
        // ERRORS.md 5 — leading whitespace / sign
        " 42", "  -42", "\t-42", "\n-42", "\r\n7", "\u{b}9", "\u{c}9", "+7", "-0", "+0",
        // ERRORS.md 6 — trailing garbage
        "12xyz", "3.9", "5 6", "7-8", "0009", "1,000", "42\n",
        // ERRORS.md 7 — overflows `long` (strtol clamps then truncates)
        "99999999999999999999",
        "-99999999999999999999",
        "9223372036854775808",
        "-9223372036854775809",
        "184467440737095516160",
        // ERRORS.md 8 — fits in `long`, overflows `int` (pure truncation)
        "2147483648",
        "-2147483649",
        "4294967296",
        "4294967295",
        "8589934592",
        "9223372036854775807",
        "-9223372036854775808",
        // exact int boundaries
        "2147483647",
        "-2147483648",
    ];
    for a in weird {
        for b in ["0", "1", "-1", "2147483647", "abc"] {
            diff_args(&[a, b]);
            diff_args(&[b, a]);
        }
    }
}

/* ---- CONFIGS.md 31 / ERRORS.md 1-2: argc < 3 --------------------- */

#[test]
fn cfg31_err01_err02_too_few_args() {
    diff_args(&[]);
    diff_args(&["5"]);
    diff_args(&[""]);

    // And confirm the concrete contract: status 2, empty stdout, usage line.
    let co = run(&c_bin(), &["5"]);
    let ro = run(&rust_bin(), &["5"]);
    assert_eq!(co.status.code(), Some(2));
    assert_eq!(ro.status.code(), Some(2));
    assert!(co.stdout.is_empty() && ro.stdout.is_empty());
    assert!(String::from_utf8_lossy(&co.stderr).ends_with(" A B\n"));
    assert!(String::from_utf8_lossy(&ro.stderr).ends_with(" A B\n"));
    assert!(String::from_utf8_lossy(&ro.stderr).starts_with("usage: "));
}

/* ---- CONFIGS.md 32 / ERRORS.md 3: surplus argv ignored ----------- */

#[test]
fn cfg32_err03_extra_args_ignored() {
    diff_args(&["3", "4", "5"]);
    diff_args(&["3", "4", "5", "6", "7"]);
    diff_args(&["3", "4", "ignored", "--flag"]);

    // Surplus arguments must not change the output at all.
    let two = run(&c_bin(), &["3", "4"]);
    let many = run(&c_bin(), &["3", "4", "99", "zzz"]);
    assert_eq!(two.stdout, many.stdout, "C: surplus argv changed stdout");
    let rtwo = run(&rust_bin(), &["3", "4"]);
    let rmany = run(&rust_bin(), &["3", "4", "99", "zzz"]);
    assert_eq!(rtwo.stdout, rmany.stdout, "Rust: surplus argv changed stdout");
}

/* ---- CONFIGS.md 33: stdout line ordering through a pipe ---------- */

#[test]
fn cfg33_stdout_line_order() {
    let co = run(&c_bin(), &["6", "7"]);
    let ro = run(&rust_bin(), &["6", "7"]);
    let cl: Vec<&str> = std::str::from_utf8(&co.stdout).unwrap().lines().collect();
    let rl: Vec<&str> = std::str::from_utf8(&ro.stdout).unwrap().lines().collect();
    assert_eq!(cl, rl, "line order differs [OP={OP} REPEAT={REPEAT}]");
    assert_eq!(cl.len(), 5, "expected 5 stdout lines, got {cl:?}");
    assert!(cl[0].starts_with("helper.call="), "line 0: {}", cl[0]);
    assert!(cl[1].starts_with("helper.ptr="), "line 1: {}", cl[1]);
    assert!(cl[2].starts_with("gen.acc="), "line 2: {}", cl[2]);
    assert!(cl[3].starts_with(&format!("op={OP} call=")), "line 3: {}", cl[3]);
    assert!(cl[4].starts_with("summary="), "line 4: {}", cl[4]);
}
