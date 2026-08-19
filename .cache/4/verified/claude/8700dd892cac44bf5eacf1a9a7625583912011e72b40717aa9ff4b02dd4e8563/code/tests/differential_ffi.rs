//! Differential tests for the exported symbol `printLine`, driven through
//! `dlopen`/`dlsym` on **both** shared objects — the C one built from
//! `c_src/src/main.c` and the Rust `cdylib`. The Rust function is never called
//! directly, only through its `#[no_mangle] extern "C"` export, so the wrapper
//! itself is under test.
//!
//! Covers `CONFIGS.md` rows F1–F8 and `ERRORS.md` rows E1, E2, G1–G5.

mod common;

use common::{c_so, probe_exe, rust_so, Diff, Outcome, Rng};
use std::process::{Command, Stdio};

/// Run the `dlopen` probe against one shared object with a list of ops.
fn probe(so: &std::path::Path, ops: &[String]) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let out = Command::new(probe_exe())
        .arg(so)
        .args(ops)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn ffi_probe");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn hex_op(bytes: &[u8]) -> String {
    let mut s = String::from("hex:");
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Compare the C `.so` and the Rust `.so` for one sequence of `printLine`
/// calls. Only stdout/stderr/status are compared — exactly what an external
/// caller can observe.
fn check(d: &mut Diff, case: &str, ops: &[String]) {
    let c = probe(&c_so(), ops);
    let r = probe(&rust_so(), ops);
    assert!(
        c.code == Some(0),
        "probe against the C .so failed for {case}: {}",
        c.describe()
    );
    d.check(case, &c, &r);
}

/// F1 / E1 / G1: `printLine(NULL)` — the guard's false branch produces no
/// output whatsoever.
#[test]
fn f1_null_pointer() {
    let mut d = Diff::new("F1,E1,G1 printLine(NULL)");
    check(&mut d, "single null", &["null".to_string()]);
    check(
        &mut d,
        "three nulls",
        &["null".to_string(), "null".to_string(), "null".to_string()],
    );
    // NULL interleaved with real strings: only the strings appear.
    check(
        &mut d,
        "null between strings",
        &[
            hex_op(b"before"),
            "null".to_string(),
            hex_op(b"after"),
        ],
    );
    d.finish();
}

/// F2 / E2 / G2: the empty string prints a bare newline (not "nothing").
#[test]
fn f2_empty_string() {
    let mut d = Diff::new("F2,E2,G2 printLine(\"\")");
    check(&mut d, "empty", &[hex_op(b"")]);
    check(&mut d, "empty x3", &[hex_op(b""), hex_op(b""), hex_op(b"")]);
    check(
        &mut d,
        "empty then text",
        &[hex_op(b""), hex_op(b"x"), hex_op(b"")],
    );
    d.finish();
}

/// F3: ordinary short ASCII payloads, including the exact literal the program
/// itself passes.
#[test]
fn f3_short_ascii() {
    let mut d = Diff::new("F3 short ASCII payloads");
    for s in [
        "fgets() failed.",
        "A",
        "AAAAA",
        &"A".repeat(99),
        "hello world",
        "  leading and trailing  ",
        "tab\tseparated",
        "%s %d %n %%",
        "quote\" backslash\\",
    ] {
        check(&mut d, &format!("ascii {s:?}"), &[hex_op(s.as_bytes())]);
    }
    let mut rng = Rng::with_seed(0x00A5_C11);
    for _ in 0..60 {
        let len = rng.below(40) as usize;
        let payload: Vec<u8> = (0..len)
            .map(|_| 0x20u8 + rng.below(0x5f) as u8)
            .collect();
        check(
            &mut d,
            &format!("rand-ascii {:?}", String::from_utf8_lossy(&payload)),
            &[hex_op(&payload)],
        );
    }
    d.finish();
}

/// F4: a payload that already contains newlines — relevant because it changes
/// where a line-buffered stream would flush.
#[test]
fn f4_payload_with_newlines() {
    let mut d = Diff::new("F4 payload containing '\\n'");
    for s in ["a\nb", "\n", "\n\n\n", "line1\nline2\n", "trailing\n"] {
        check(&mut d, &format!("newlines {s:?}"), &[hex_op(s.as_bytes())]);
    }
    check(
        &mut d,
        "mixed cr lf",
        &[hex_op(b"a\r\nb\rc\nd")],
    );
    d.finish();
}

/// F5 / G4: high-bit and non-UTF-8 bytes must pass through verbatim — a
/// translation that routed the string through Rust's `str` would corrupt or
/// reject these.
#[test]
fn f5_non_utf8_bytes() {
    let mut d = Diff::new("F5,G4 non-UTF-8 bytes");
    // Every single non-zero byte value, one call each (batched to keep the
    // number of spawned probes low).
    let mut ops = Vec::new();
    for b in 1u8..=255 {
        ops.push(hex_op(&[b]));
    }
    check(&mut d, "every byte 0x01..=0xff individually", &ops);

    // Known-invalid UTF-8 sequences.
    for seq in [
        vec![0x80],
        vec![0xff, 0xfe],
        vec![0xc3],             // truncated 2-byte sequence
        vec![0xe2, 0x82],       // truncated 3-byte sequence
        vec![0xf0, 0x9f, 0x92], // truncated 4-byte sequence
        vec![0xed, 0xa0, 0x80], // surrogate half
        vec![0xc0, 0x80],       // overlong NUL encoding
        vec![0xfe, 0xff, 0xfe, 0xff],
    ] {
        check(&mut d, &format!("invalid utf8 {seq:?}"), &[hex_op(&seq)]);
    }

    let mut rng = Rng::with_seed(0xBADF_00D5);
    for _ in 0..60 {
        let len = 1 + rng.below(32) as usize;
        // Any byte except NUL (a NUL would just terminate the C string early).
        let payload: Vec<u8> = (0..len)
            .map(|_| {
                let mut b = rng.byte();
                if b == 0 {
                    b = 1;
                }
                b
            })
            .collect();
        check(&mut d, &format!("rand-bytes {payload:?}"), &[hex_op(&payload)]);
    }
    d.finish();
}

/// F6 / G3: payloads bigger than glibc's 4096-byte stdout buffer, which forces
/// the C side to flush mid-string while the Rust side accumulates — the final
/// byte stream must still be identical.
#[test]
fn f6_oversized_payloads() {
    let mut d = Diff::new("F6,G3 oversized payloads");
    for n in [
        1usize, 99, 100, 4095, 4096, 4097, 8192, 16384, 65535, 65536, 100_000,
    ] {
        check(
            &mut d,
            &format!("{n} x 'A'"),
            &[format!("rep:41:{n}")],
        );
    }
    // A large payload split across several calls.
    check(
        &mut d,
        "3 x 4096 in one process",
        &[
            "rep:41:4096".to_string(),
            "rep:42:4096".to_string(),
            "rep:43:4096".to_string(),
        ],
    );
    d.finish();
}

/// F7 / G5: many sequential calls — checks ordering and that nothing is lost
/// or duplicated as the shared buffer fills.
#[test]
fn f7_call_sequences() {
    let mut d = Diff::new("F7,G5 sequences of calls");
    let mut rng = Rng::with_seed(0x5E90_1234);
    for trial in 0..20 {
        let n = 1 + rng.below(40) as usize;
        let ops: Vec<String> = (0..n)
            .map(|i| {
                if rng.below(5) == 0 {
                    "null".to_string()
                } else {
                    let len = rng.below(20) as usize;
                    let payload: Vec<u8> = (0..len)
                        .map(|_| b'a' + (rng.below(26) as u8))
                        .collect();
                    let _ = i;
                    hex_op(&payload)
                }
            })
            .collect();
        check(&mut d, &format!("sequence #{trial} of {n} calls"), &ops);
    }
    d.finish();
}

/// F8: broad randomized payload sweep.
#[test]
fn f8_random_payload_sweep() {
    let mut d = Diff::new("F8 randomized payloads");
    let mut rng = Rng::with_seed(0x7A57_E500);
    // Batch 8 payloads per probe process to keep the spawn count reasonable.
    for batch in 0..32 {
        let ops: Vec<String> = (0..8)
            .map(|_| {
                let len = rng.below(301) as usize;
                let payload: Vec<u8> = (0..len)
                    .map(|_| {
                        let mut b = rng.byte();
                        if b == 0 {
                            b = b'.';
                        }
                        b
                    })
                    .collect();
                hex_op(&payload)
            })
            .collect();
        check(&mut d, &format!("random batch #{batch}"), &ops);
    }
    d.finish();
}

/// Harness sanity: confirm the probe really is loading the symbol and that the
/// C side produces the expected bytes, so the comparisons cannot pass vacuously.
#[test]
fn harness_sanity_probe_actually_calls_printline() {
    let out = probe(&c_so(), &[hex_op(b"hello")]);
    assert_eq!(out.code, Some(0), "probe failed: {}", out.describe());
    assert_eq!(out.stdout, b"hello\n");

    let out = probe(&rust_so(), &[hex_op(b"hello")]);
    assert_eq!(out.code, Some(0), "probe failed: {}", out.describe());
    assert_eq!(out.stdout, b"hello\n");

    // NULL really produces nothing at all.
    assert!(probe(&c_so(), &["null".to_string()]).stdout.is_empty());
    assert!(probe(&rust_so(), &["null".to_string()]).stdout.is_empty());

    // A missing symbol would make the probe fail loudly rather than silently
    // comparing two empty outputs.
    let bogus = Command::new(probe_exe())
        .arg("/nonexistent/libnope.so")
        .arg("null")
        .output()
        .unwrap();
    assert!(!bogus.status.success(), "probe must fail on a bad .so path");
}
