//! Phase C — crash-parity rows of `ERRORS.md` (2, 4, 5, 24, 33, 35, 37) plus the
//! generic FFI boundary crashes (NULL key, NULL out-parameter, oversized length).
//!
//! Each scenario runs in a **child process** that loads exactly ONE of the two
//! libraries; the parent compares the two wait-statuses (exit code / fatal
//! signal) and the `assert()` expression text glibc prints on `stderr`.

mod common;

use common::*;
use std::ffi::{c_char, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// name -> what the C is expected to do (documentation only; the assertion is
/// always "C and Rust agree")
const SCENARIOS: &[&str] = &[
    "control_survive",          // harness self-test: exit 0
    "arrgrowf_oom",             // row 2  : realloc fails -> write through NULL+32
    "arrgrowf_size_overflow",   // row 2b : elemsize*min_cap wraps -> small alloc, no crash
    "arrfreef_null",            // row 4  : free(NULL-32)
    "hash_string_null",         // row 5  : *NULL
    "hash_bytes_null_len0",     // row 7  : *not* a crash
    "hash_bytes_null_len1",     // generic: NULL + non-zero length
    "hash_bytes_huge_len",      // generic: oversized length
    "hmdel_ptr_to_string",      // row 24 : assert(slot >= 0)
    "stralloc_forged_arena",    // row 33 : storage == NULL but remaining > 0
    "stralloc_null_str",        // row 35 : strlen(NULL)
    "stralloc_huge_block",      // row 34b: forged block -> realloc fails
    "strreset_null",            // row 37 : a->storage through NULL
    "hmput_key_null_key_bin",   // generic: NULL key, binary mode
    "hmput_key_null_key_str",   // generic: NULL key, string mode
    "hmget_key_ts_null_temp",   // generic: NULL out-parameter
    "hmput_key_huge_keysize",   // generic: oversized keysize
    "hmfree_func_null",         // row 11 : *not* a crash
    "hmdel_key_null_map",       // row 26 : *not* a crash
];

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    assertion: Option<String>,
}

fn run_child(scenario: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args([
            "--exact",
            "crash_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HARVEST_SCENARIO", scenario)
        .env("HARVEST_WHICH", which)
        .env("HARVEST_C_LIB", c_lib_path())
        .env("HARVEST_RUST_LIB", rust_lib_path())
        .output()
        .expect("spawning the child test binary");
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    // glibc prints: `prog: <file>:<line>: <func>: Assertion `<expr>' failed.`
    // Everything from the *basename* of `__FILE__` on must be byte-identical;
    // the directory part is a property of the build tree, not of the library.
    let assertion = err.find("lib.c:").map(|i| {
        let rest = &err[i..];
        rest[..rest.find('\n').unwrap_or(rest.len())].to_string()
    });
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        assertion,
    }
}

#[test]
fn crash_parity_all_scenarios() {
    // (the child processes load the libraries themselves, but resolving the
    // paths here makes sure they exist and are handed down explicitly)
    let _ = c_lib_path();
    let _ = rust_lib_path();
    let mut report = Vec::new();
    for s in SCENARIOS {
        let c = run_child(s, "c");
        let r = run_child(s, "rust");
        report.push(format!("{:<26} C={:?} Rust={:?}", s, c, r));
        assert_eq!(
            c, r,
            "scenario `{}` behaves differently:\n  C   : {:?}\n  Rust: {:?}",
            s, c, r
        );
    }
    for line in report {
        println!("{}", line);
    }
}

// ---------------------------------------------------------------------------
// the child
// ---------------------------------------------------------------------------

#[test]
#[ignore = "spawned by crash_parity_all_scenarios"]
fn crash_child() {
    let scenario = match std::env::var("HARVEST_SCENARIO") {
        Ok(s) => s,
        Err(_) => return,
    };
    let which = std::env::var("HARVEST_WHICH").unwrap();
    let api = load_single(&which);
    let mut keys = Keys::new();
    unsafe {
        (api.rand_seed)(0x3141_5926);
        match scenario.as_str() {
            "control_survive" => {
                let p = (api.arrgrowf)(std::ptr::null_mut(), 8, 1, 0);
                (api.arrfreef)(p);
            }
            // ---- row 2 : realloc failure, then a write through NULL+32 ------
            "arrgrowf_oom" => {
                let p = (api.arrgrowf)(std::ptr::null_mut(), 1 << 50, 0, 1);
                std::hint::black_box(p);
            }
            // ---- row 2b: elemsize*min_cap wraps to 0 -> 32 byte allocation ---
            "arrgrowf_size_overflow" => {
                let p = (api.arrgrowf)(std::ptr::null_mut(), 1 << 62, 0, 1);
                let h = read_header(p as *mut u8);
                assert_eq!((h.length, h.capacity), (0, 4));
                (api.arrfreef)(p);
            }
            // ---- row 4 -------------------------------------------------------
            "arrfreef_null" => (api.arrfreef)(std::ptr::null_mut()),
            // ---- row 5 -------------------------------------------------------
            "hash_string_null" => {
                let h = (api.hash_string)(std::ptr::null_mut(), 1);
                std::hint::black_box(h);
            }
            // ---- row 7 : NOT a crash ----------------------------------------
            "hash_bytes_null_len0" => {
                let h = (api.hash_bytes)(std::ptr::null_mut(), 0, 12345);
                assert_ne!(h, 0xdead);
            }
            "hash_bytes_null_len1" => {
                let h = (api.hash_bytes)(std::ptr::null_mut(), 1, 12345);
                std::hint::black_box(h);
            }
            "hash_bytes_huge_len" => {
                let buf = [0u8; 64];
                let h = (api.hash_bytes)(buf.as_ptr() as *mut c_void, usize::MAX / 2, 1);
                std::hint::black_box(h);
            }
            // ---- row 24: assert(slot >= 0) -----------------------------------
            "hmdel_ptr_to_string" => {
                // DEFAULT string map, 4 entries, then delete the *first* one
                // with mode == STBDS_HM_PTR_TO_STRING(2): the re-lookup of the
                // moved entry hashes the address of the key pointer instead of
                // the string, misses, and trips the assert.
                let mut t: *mut c_void = std::ptr::null_mut();
                for i in 0..4 {
                    let k = keys.string(format!("key-{}", i).as_bytes());
                    t = (api.hmput_key)(t, 16, k, 8, STBDS_HM_STRING);
                    let temp = read_header((t as *mut u8).sub(16)).temp;
                    *((t as *mut u8).offset(16 * temp).add(8) as *mut u64) = i as u64;
                }
                let first = std::ptr::read_unaligned(t as *const *mut c_void);
                let t2 = (api.hmdel_key)(t, 16, first, 8, 0, STBDS_HM_PTR_TO_STRING);
                std::hint::black_box(t2);
            }
            // ---- row 33 ------------------------------------------------------
            "stralloc_forged_arena" => {
                let mut a = Arena {
                    storage: std::ptr::null_mut(),
                    remaining: 100,
                    block: 0,
                    mode: 0,
                };
                let s = b"short\0";
                let p = (api.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                std::hint::black_box(p);
            }
            // ---- row 35 ------------------------------------------------------
            "stralloc_null_str" => {
                let mut a = Arena {
                    storage: std::ptr::null_mut(),
                    remaining: 0,
                    block: 0,
                    mode: 0,
                };
                let p = (api.stralloc)(&mut a, std::ptr::null_mut());
                std::hint::black_box(p);
            }
            // ---- row 34b: 512 << 50 bytes cannot be allocated ----------------
            "stralloc_huge_block" => {
                let mut a = Arena {
                    storage: std::ptr::null_mut(),
                    remaining: 0,
                    block: 100,
                    mode: 0,
                };
                let s = b"x\0";
                let p = (api.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                std::hint::black_box(p);
            }
            // ---- row 37 ------------------------------------------------------
            "strreset_null" => (api.strreset)(std::ptr::null_mut()),
            // ---- generic FFI boundary ---------------------------------------
            "hmput_key_null_key_bin" => {
                let t = (api.hmput_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, 0);
                std::hint::black_box(t);
            }
            "hmput_key_null_key_str" => {
                let t = (api.hmput_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, 1);
                std::hint::black_box(t);
            }
            "hmget_key_ts_null_temp" => {
                let k = keys.raw(b"key");
                let t = (api.hmget_key_ts)(std::ptr::null_mut(), 16, k, 8, std::ptr::null_mut(), 0);
                std::hint::black_box(t);
            }
            "hmput_key_huge_keysize" => {
                let k = keys.raw(b"key");
                let t = (api.hmput_key)(std::ptr::null_mut(), 16, k, usize::MAX / 2, 0);
                std::hint::black_box(t);
            }
            "hmfree_func_null" => (api.hmfree_func)(std::ptr::null_mut(), 16),
            "hmdel_key_null_map" => {
                let k = keys.raw(b"key");
                let t = (api.hmdel_key)(std::ptr::null_mut(), 16, k, 8, 0, 0);
                assert!(t.is_null());
            }
            other => panic!("unknown scenario {}", other),
        }
    }
    // survived: report a distinctive exit code so "survived" cannot be confused
    // with a libtest failure (101) or a signal
    let _ = std::io::Write::flush(&mut std::io::stdout());
    std::process::exit(7);
}
