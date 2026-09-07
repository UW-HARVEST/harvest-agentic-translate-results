//! `CONFIGS.md` row 41 — configurations whose outcome may be either a normal
//! return **or** a fatal `assert`, so the differential comparison has to happen
//! across a process boundary.
//!
//! The child prints a canonical one-line summary of everything observable
//! (return value, output-buffer digest, output-buffer bytes, `cp_error_reason`)
//! to stdout; the parent then requires the C and Rust children to agree on
//! stdout, stderr and exit status / signal.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Every (align, in_bytes) pair row 41 covers.
fn cases() -> Vec<(usize, i32)> {
    let mut v = Vec::new();
    for align in 0..4usize {
        for in_bytes in 1..=6i32 {
            v.push((align, in_bytes));
        }
    }
    v
}

const OUT_BYTES: i32 = 256;
const OUT_CAP: usize = 1024;

fn child_body(which: &str, idx: usize) {
    let lib = match which {
        "c" => Lib::open("C", &c_so_path()),
        "rust" => Lib::open("RUST", &rust_so_path()),
        _ => panic!("bad lib"),
    };
    let (align, in_bytes) = cases()[idx];
    // fixed, deterministic input contents (identical in both children)
    let data: Vec<u8> = (0..8u8).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
    let mut inbuf = AlignedBuf::new(&data, align);
    let mut out = vec![0xCDu8; OUT_CAP];
    let ret = unsafe {
        (lib.cp_inflate())(
            inbuf.ptr() as *mut c_void,
            in_bytes,
            out.as_mut_ptr() as *mut c_void,
            OUT_BYTES,
        )
    };
    let reason = lib
        .error_reason()
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_else(|| "<null>".to_string());
    println!(
        "RET={ret} DIGEST={:016x} HEAD={:02x?} REASON={reason}",
        fnv1a(&out),
        &out[..32]
    );
}

#[test]
fn risky_child() {
    if let Ok(idx) = std::env::var("CP_RISKY_IDX") {
        let which = std::env::var("CP_RISKY_LIB").expect("CP_RISKY_LIB");
        child_body(&which, idx.parse().unwrap());
        std::process::exit(0);
    }
}

struct Out {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn spawn(idx: usize, which: &str) -> Out {
    let exe = std::env::current_exe().unwrap();
    let o = Command::new(exe)
        .args(["--exact", "risky_child", "--nocapture", "--test-threads=1"])
        .env("CP_RISKY_IDX", idx.to_string())
        .env("CP_RISKY_LIB", which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn");
    Out {
        code: o.status.code(),
        signal: o.status.signal(),
        stdout: o.stdout,
        stderr: o.stderr,
    }
}

/// Keep only the child's own summary line (drop libtest's framing).
fn summary(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .lines()
        .filter(|l| l.starts_with("RET="))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn row41_in_bytes_smaller_than_first_bytes() {
    for (idx, (align, in_bytes)) in cases().into_iter().enumerate() {
        let c = spawn(idx, "c");
        let r = spawn(idx, "rust");
        let label = format!("row41/align={align} in_bytes={in_bytes}");
        assert_eq!(
            (c.signal, c.code),
            (r.signal, r.code),
            "[{label}] termination differs (C {:?}/{:?} vs RUST {:?}/{:?})\nC stderr:\n{}\nRUST stderr:\n{}",
            c.signal,
            c.code,
            r.signal,
            r.code,
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
            "[{label}] stderr differs"
        );
        assert_eq!(summary(&c.stdout), summary(&r.stdout), "[{label}] observable result differs");
    }
}
