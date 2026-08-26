//! Phase C — ERRORS.md rows whose trigger kills the process (rows 2, 5, 28, 33).
//!
//! Each scenario is re-executed in a child process against exactly ONE of the
//! two shared objects, and the child's termination status (exit code + signal)
//! must be identical for the C `.so` and the Rust `.so`. `STBDS_ASSERT` is
//! `assert()` from `<assert.h>` and the cmake build defines no `NDEBUG`, so a
//! failing assertion aborts (SIGABRT); the Rust translation calls `abort()` in
//! the same places.
mod common;

use common::*;
use std::ffi::{c_char, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const SCENARIOS: [&str; 4] = [
    "arrgrowf_alloc_failure",   // ERRORS.md row 2  -> SIGSEGV
    "arrfreef_null",            // ERRORS.md row 5  -> SIGSEGV (free of NULL-32)
    "hmdel_outofrange_mode",    // ERRORS.md row 28 -> SIGABRT (STBDS_ASSERT(slot >= 0))
    "stralloc_corrupt_arena",   // ERRORS.md row 33 -> SIGSEGV
];

/// Sentinel exit code the child uses when the scenario did NOT crash.
const NO_CRASH: i32 = 70;

#[derive(Debug, PartialEq, Eq, Clone)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stderr: String,
}

fn run_child_with(scenario: &str, which: &str, rust_so: Option<&std::path::Path>) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(exe);
    cmd.args([
        "--exact",
        "crash_runner",
        "--nocapture",
        "--test-threads=1",
    ])
    .env("STBDS_CRASH_SCENARIO", scenario)
    .env("STBDS_CRASH_LIB", which)
    .env("RUST_BACKTRACE", "0")
    .env("LIBC_FATAL_STDERR_", "1")
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    if let Some(p) = rust_so {
        cmd.env("RUST_SO", p);
    }
    let out = cmd.output().expect("failed to spawn the crash child");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

fn run_child(scenario: &str, which: &str) -> (Option<i32>, Option<i32>) {
    let o = run_child_with(scenario, which, None);
    (o.code, o.signal)
}

/// The release cdylib is the artifact this crate actually produces
/// (`crate-type = ["cdylib"]` + `[profile.release] panic = "abort"`), so that is
/// the build the C library is compared against bit-for-bit.
fn release_so() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libarr_ins_lib.so")
}

/// Rust's `debug_assertions` builds insert UB checks that turn a null-pointer
/// dereference into a non-unwinding panic (SIGABRT) instead of letting the CPU
/// fault (SIGSEGV). Two of the four scenarios in ERRORS.md are exactly that
/// kind of C-level UB, so the *signal* legitimately differs for a debug build
/// while the observable "the process dies immediately" does not.
fn is_rust_ub_check_panic(stderr: &str) -> bool {
    stderr.contains("null pointer dereference")
        || stderr.contains("unsafe precondition")
        || stderr.contains("non-unwinding panic")
}

/// The child entry point. Does nothing unless `STBDS_CRASH_SCENARIO` is set, so
/// it is a no-op when the test binary is run normally.
#[test]
fn crash_runner() {
    let scenario = match std::env::var("STBDS_CRASH_SCENARIO") {
        Ok(v) => v,
        Err(_) => return,
    };
    let which = std::env::var("STBDS_CRASH_LIB").unwrap();
    let p = pair();
    let lib: &Lib = if which == "c" { &p.c } else { &p.rust };

    unsafe {
        match scenario.as_str() {
            // ---------------- ERRORS.md row 2 ----------------
            // realloc(NULL, 1 * (1<<62) + 32) fails -> b = NULL, then
            // b = (char*)b + 32 and stbds_header(b)->length = 0 writes to
            // address 0.
            "arrgrowf_alloc_failure" => {
                let out = (lib.arrgrowf)(std::ptr::null_mut(), 1, 0, 1usize << 62);
                // must not be reached
                std::hint::black_box(out);
            }
            // ---------------- ERRORS.md row 5 ----------------
            // stbds_arrfreef has no NULL guard: free((char*)NULL - 32).
            "arrfreef_null" => {
                (lib.arrfreef)(std::ptr::null_mut());
            }
            // ---------------- ERRORS.md row 28 ----------------
            // mode = 2 is an out-of-range STBDS_HM_* enum value. hm_find_slot
            // treats it as "string" (mode >= STBDS_HM_STRING) but the
            // compaction re-find at lib.c:842 tests `mode == STBDS_HM_STRING`,
            // which is false, so it passes the ADDRESS of the key pointer
            // instead of the key text. The lookup misses and
            // STBDS_ASSERT(slot >= 0) aborts.
            "hmdel_outofrange_mode" => {
                let lay = L_STR;
                let mut bufs: Vec<Box<[u8]>> = Vec::new();
                let mut ptrs: Vec<*mut c_char> = Vec::new();
                for i in 0..6usize {
                    let mut v = format!("oor-mode-key-{}", i).into_bytes();
                    v.push(0);
                    while v.len() < 32 {
                        v.push(0);
                    }
                    let mut b = v.into_boxed_slice();
                    let q = b.as_mut_ptr() as *mut c_char;
                    bufs.push(b);
                    ptrs.push(q);
                }
                let mut hp: *mut c_void = std::ptr::null_mut();
                for (i, &q) in ptrs.iter().enumerate() {
                    let v = vec![i as u8; lay.elemsize - 8];
                    hp = map_put_string(lib, hp, lay, q, &v, HM_STRING);
                }
                // delete the FIRST key: old_index(0) != final_index(4), so the
                // compaction + re-find path runs.
                let out = (lib.hmdel_key)(
                    hp,
                    lay.elemsize,
                    ptrs[0] as *mut c_void,
                    lay.keysize,
                    0,
                    2,
                );
                std::hint::black_box(out);
                std::mem::forget(bufs);
            }
            // ---------------- ERRORS.md row 33 ----------------
            // A hand-corrupted arena (remaining > 0 while storage == NULL)
            // makes STBDS_ASSERT(len <= a->remaining) pass, after which
            // `a->storage->storage + a->remaining - len` is dereferenced.
            "stralloc_corrupt_arena" => {
                let mut arena = Arena {
                    storage: std::ptr::null_mut(),
                    remaining: 4096,
                    block: 0,
                    mode: 0,
                };
                let mut buf = b"boom\0".to_vec();
                let out = (lib.stralloc)(&mut arena, buf.as_mut_ptr() as *mut c_char);
                std::hint::black_box(out);
            }
            other => panic!("unknown crash scenario {}", other),
        }
    }

    // Reaching this point means the scenario did NOT terminate the process.
    std::process::exit(NO_CRASH);
}

#[test]
fn err_02_05_28_33_crash_equivalence() {
    // don't run the comparison inside a crash child
    if std::env::var("STBDS_CRASH_SCENARIO").is_ok() {
        return;
    }
    // make sure the two .so files we are about to compare really exist
    assert!(c_so_path().exists(), "C .so missing: {}", c_so_path().display());
    assert!(
        rust_so_path().exists(),
        "Rust .so missing: {}",
        rust_so_path().display()
    );

    let rel = release_so();
    assert!(rel.exists(), "release .so missing: {} (run `cargo build --release`)", rel.display());

    let mut report = String::new();
    for scenario in SCENARIOS {
        let c = run_child_with(scenario, "c", None);
        let r = run_child_with(scenario, "rust", Some(&rel));
        report.push_str(&format!(
            "  {:<26} C: code={:?} signal={:?}   RUST(release): code={:?} signal={:?}\n",
            scenario, c.code, c.signal, r.code, r.signal
        ));
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "scenario `{}` terminated differently: C {:?} vs RUST {:?}",
            scenario,
            (c.code, c.signal),
            (r.code, r.signal)
        );
        assert_ne!(
            c.code,
            Some(NO_CRASH),
            "scenario `{}` was expected to terminate the process but returned normally",
            scenario
        );
        assert_ne!(
            c.code,
            Some(0),
            "scenario `{}` exited successfully; the crash was not reproduced",
            scenario
        );
        assert!(
            c.signal.is_some(),
            "scenario `{}` did not die from a signal (code={:?})",
            scenario,
            c.code
        );
    }
    eprintln!("crash-equivalence report (release .so):\n{}", report);
}

/// Whatever `.so` `RUST_SO` points at (release **or** debug) must still die
/// fatally on every scenario, and any signal difference against the C must be
/// fully explained by Rust's `debug_assertions` UB checks firing on code paths
/// where the C itself is undefined (null-pointer dereference).
#[test]
fn err_02_05_28_33_current_profile_also_terminates() {
    if std::env::var("STBDS_CRASH_SCENARIO").is_ok() {
        return;
    }
    let mut report = String::new();
    for scenario in SCENARIOS {
        let c = run_child_with(scenario, "c", None);
        let r = run_child_with(scenario, "rust", None);
        report.push_str(&format!(
            "  {:<26} C: signal={:?}   RUST({}): signal={:?} ub_check={}\n",
            scenario,
            c.signal,
            rust_so_path().display(),
            r.signal,
            is_rust_ub_check_panic(&r.stderr)
        ));
        assert_ne!(r.code, Some(NO_CRASH), "`{}` did not terminate", scenario);
        assert_ne!(r.code, Some(0), "`{}` exited successfully", scenario);
        assert!(
            r.signal.is_some(),
            "`{}` did not die from a signal (code={:?})",
            scenario,
            r.code
        );
        if (c.code, c.signal) != (r.code, r.signal) {
            assert!(
                is_rust_ub_check_panic(&r.stderr),
                "scenario `{}`: C {:?} vs RUST {:?} and the difference is NOT a \
                 debug_assertions UB check.\nRust stderr:\n{}",
                scenario,
                (c.code, c.signal),
                (r.code, r.signal),
                r.stderr
            );
            assert_eq!(
                r.signal,
                Some(6),
                "a UB-check panic must abort (SIGABRT) - scenario `{}`",
                scenario
            );
        }
    }
    eprintln!("current-profile termination report:\n{}", report);
}

/// The expected signals, spelled out so a change in behaviour is loud.
#[test]
fn err_02_05_28_33_expected_signals() {
    if std::env::var("STBDS_CRASH_SCENARIO").is_ok() {
        return;
    }
    const SIGABRT: i32 = 6;
    const SIGSEGV: i32 = 11;
    let rel = release_so();
    assert!(rel.exists(), "release .so missing: {}", rel.display());
    let expected: [(&str, i32); 4] = [
        // realloc failure -> write through (char*)NULL + 32 - 32
        ("arrgrowf_alloc_failure", SIGSEGV),
        // free((char*)NULL - 32): glibc reads the chunk header at
        // 0xffffffffffffffe0 before it can complain, so this is a SIGSEGV
        // rather than the "free(): invalid pointer" SIGABRT
        ("arrfreef_null", SIGSEGV),
        // STBDS_ASSERT(slot >= 0) in stbds_hmdel_key -> __assert_fail -> abort
        ("hmdel_outofrange_mode", SIGABRT),
        // a->storage->storage dereferenced with a->storage == NULL
        ("stralloc_corrupt_arena", SIGSEGV),
    ];
    for (scenario, sig) in expected {
        let c = run_child_with(scenario, "c", None);
        let r = run_child_with(scenario, "rust", Some(&rel));
        assert_eq!(
            c.signal,
            Some(sig),
            "C: scenario `{}` should die from signal {} (got {:?})",
            scenario,
            sig,
            (c.code, c.signal)
        );
        assert_eq!(
            r.signal,
            Some(sig),
            "RUST(release): scenario `{}` should die from signal {} (got {:?})",
            scenario,
            sig,
            (r.code, r.signal)
        );
    }
}
