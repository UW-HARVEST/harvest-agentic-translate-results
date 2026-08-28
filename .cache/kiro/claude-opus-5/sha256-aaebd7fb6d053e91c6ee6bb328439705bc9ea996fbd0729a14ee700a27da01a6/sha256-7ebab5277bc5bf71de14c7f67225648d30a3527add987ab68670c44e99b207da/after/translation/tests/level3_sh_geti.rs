//! Level 3: the public entry point `sh_geti` from `include/lib.h`.
//!
//! `sh_geti` communicates only through `printf`, so the comparison redirects
//! file descriptor 1 into a temporary file around each call and diffs the two
//! byte streams. Both `.so`s share this process' libc, so `fflush(NULL)` from
//! the test drains their `stdout` buffers.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// Runs `f` with fd 1 pointing at a fresh temporary file and returns whatever
/// was written.
fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "sh_geti_capture_{}_{}.txt",
        std::process::id(),
        tag
    ));
    let mut cpath = path.to_str().unwrap().as_bytes().to_vec();
    cpath.push(0);

    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(
            cpath.as_ptr() as *const c_char,
            O_RDWR | O_CREAT | O_TRUNC,
            0o600 as c_int,
        );
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        close(fd);
    }

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

fn compare_sh_geti(num: c_int) {
    let (c, r) = apis();

    // Both libraries keep their own copy of `stbds_hash_seed`; align them so
    // the two runs see identical table seeds.
    reset_seeds(&c, &r, DEFAULT_SEED);

    let out_c = capture_stdout("c", || unsafe { (c.sh_geti)(num) });

    reset_seeds(&c, &r, DEFAULT_SEED);
    let out_r = capture_stdout("rust", || unsafe { (r.sh_geti)(num) });

    if out_c != out_r {
        let sc = String::from_utf8_lossy(&out_c);
        let sr = String::from_utf8_lossy(&out_r);
        let first_diff = out_c
            .iter()
            .zip(out_r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(out_c.len().min(out_r.len()));
        panic!(
            "sh_geti({}) output mismatch (C {} bytes, Rust {} bytes, first diff at {})\n\
             --- C ---\n{}\n--- Rust ---\n{}",
            num,
            out_c.len(),
            out_r.len(),
            first_diff,
            &sc.chars().take(2000).collect::<String>(),
            &sr.chars().take(2000).collect::<String>()
        );
    }
}

#[test]
fn sh_geti_small_inputs() {
    let _g = serial();
    for num in 0..=20 {
        compare_sh_geti(num);
    }
}

#[test]
fn sh_geti_around_table_growth() {
    let _g = serial();
    // 6 == used_count_threshold for the initial 8-slot table, so these values
    // straddle every doubling boundary.
    for num in [
        5, 6, 7, 8, 9, 11, 12, 13, 15, 16, 17, 23, 24, 25, 31, 32, 33, 47, 48, 49, 63, 64, 65,
    ] {
        compare_sh_geti(num);
    }
}

#[test]
fn sh_geti_larger_inputs() {
    let _g = serial();
    for num in [100, 128, 200, 256, 333, 512, 1000] {
        compare_sh_geti(num);
    }
}

#[test]
fn sh_geti_negative_input() {
    let _g = serial();
    // Every loop is `for (i=0; i < num; ...)` so a negative `num` simply skips
    // all of them; the two `j` iterations still run.
    for num in [-1, -5, i32::MIN] {
        compare_sh_geti(num);
    }
}

#[test]
fn sh_geti_output_is_non_empty_and_well_formed() {
    let _g = serial();
    // Guards against the capture harness silently comparing two empty strings.
    let (c, r) = apis();
    reset_seeds(&c, &r, DEFAULT_SEED);
    let out = capture_stdout("shape", || unsafe { (c.sh_geti)(10) });
    let text = String::from_utf8_lossy(&out);
    assert!(!out.is_empty(), "expected sh_geti(10) to print something");
    for line in text.lines() {
        let mut it = line.split(' ');
        let key = it.next().unwrap();
        let val: i32 = it.next().unwrap().parse().expect("value must be an integer");
        assert!(key.starts_with("test_"), "unexpected key {:?}", key);
        let n: i32 = key["test_".len()..].parse().unwrap();
        assert_eq!(val, n * 3, "line {:?}", line);
        assert!(it.next().is_none(), "extra fields in {:?}", line);
    }
}
