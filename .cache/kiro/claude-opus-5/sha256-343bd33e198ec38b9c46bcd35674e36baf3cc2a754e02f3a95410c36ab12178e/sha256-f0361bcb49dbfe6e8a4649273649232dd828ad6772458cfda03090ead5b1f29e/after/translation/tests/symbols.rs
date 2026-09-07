//! Phase D — symbol parity, plus self-checks that prove the differential
//! harness is not passing vacuously.

mod common;

use common::{capture, libs, GotomachFn, OpFn};
use std::collections::BTreeSet;
use std::process::Command;

/// Defined (exported) dynamic symbols of a shared object, as reported by
/// `nm -D --defined-only`.
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

/// Undefined (imported) dynamic symbols.
fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let l = libs();
    assert_ne!(
        l.c_path.canonicalize().unwrap(),
        l.r_path.canonicalize().unwrap(),
        "the harness must load two DIFFERENT shared objects"
    );

    let c = defined_symbols(&l.c_path);
    let r = defined_symbols(&l.r_path);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The four symbols we expect, sanity-checked against the C source.
    for want in ["gotomach", "process_value", "double_value", "triple_value"] {
        assert!(c.contains(want), "C .so should export {want}");
        assert!(r.contains(want), "Rust .so should export {want}");
    }

    // Every expected symbol must be resolvable via dlsym in BOTH libraries.
    for lib in [&l.c, &l.r] {
        let _: libloading::Symbol<GotomachFn> = unsafe { lib.get(b"gotomach\0") }.unwrap();
        for n in [
            &b"process_value\0"[..],
            &b"double_value\0"[..],
            &b"triple_value\0"[..],
        ] {
            let _: libloading::Symbol<OpFn> = unsafe { lib.get(n) }.unwrap();
        }
    }
}

#[test]
fn phase_d_static_c_functions_are_not_exported_by_either_library() {
    let l = libs();
    let c = defined_symbols(&l.c_path);
    let r = defined_symbols(&l.r_path);
    for hidden in [
        "is_valid_state",
        "check_char_flag",
        "init_processor",
        "cleanup_processor",
    ] {
        assert!(!c.contains(hidden), "C must not export static {hidden}");
        assert!(!r.contains(hidden), "Rust must not export {hidden}");
    }
}

#[test]
fn phase_d_rust_has_no_undefined_non_libc_symbols() {
    let l = libs();
    let c_undef = undefined_symbols(&l.c_path);
    // The C library's imports must all be satisfiable by the Rust library too.
    for s in ["malloc", "free", "puts"] {
        assert!(c_undef.contains(s), "expected C to import {s}");
    }
    // The C source has no `printf` import: the format string has no conversion
    // specifiers, so the compiler lowers it to `puts`. The Rust translation
    // must match, otherwise output buffering/bytes could differ.
    assert!(
        !c_undef.contains("printf"),
        "C imports printf; the Rust translation's use of puts would be wrong"
    );

    let r_undef = undefined_symbols(&l.r_path);
    // Everything the Rust cdylib imports must be libc / platform runtime.
    let known = [
        "abort", "bcmp", "calloc", "close", "dl", "environ", "exit", "fstat", "fstat64", "free",
        "getcwd", "getenv", "gettid", "gettimeofday", "lseek", "lseek64", "malloc", "memchr",
        "memcmp", "memcpy", "memmove", "memrchr", "memset", "mmap", "mmap64", "munmap", "open",
        "poll", "posix_", "pthread_", "puts", "read", "readlink", "realloc", "realpath",
        "sigaction", "sigaltstack", "signal", "stat", "strlen", "syscall", "sysconf", "write",
    ];
    let suspicious: Vec<&String> = r_undef
        .iter()
        .filter(|s| !s.starts_with('_') && !known.iter().any(|p| s.starts_with(p)))
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has undefined symbols that are not libc/runtime: {suspicious:?}"
    );

    // No symbol the C library needs may be left unresolved by the Rust one.
    for s in c_undef.iter().filter(|s| !s.starts_with('_')) {
        assert!(
            r_undef.contains(s),
            "Rust .so does not import {s}, which the C .so needs"
        );
    }
}

// ---------------------------------------------------------------------------
// Harness self-checks. If stdout capture silently produced empty buffers, every
// stdout comparison in Phases B and C would pass vacuously. These tests pin the
// exact expected bytes so that cannot happen unnoticed.
// ---------------------------------------------------------------------------

fn goto_out(which: usize, it: i32, sd: i32, m: i32, th: i32) -> (i32, Vec<u8>) {
    let l = libs();
    let lib = if which == 0 { &l.c } else { &l.r };
    let f: libloading::Symbol<GotomachFn> = unsafe { lib.get(b"gotomach\0") }.unwrap();
    capture(|| unsafe { f(it, sd, m, th) })
}

#[test]
fn harness_stdout_capture_sees_the_exact_log_bytes() {
    let cases: &[(i32, i32, i32, i32, i32, &str)] = &[
        (
            -1,
            0,
            0,
            0,
            -1,
            "[INFO] Starting gotomach function\n[ERROR] Invalid iteration count\n",
        ),
        (
            65536,
            0,
            0,
            0,
            -1,
            "[INFO] Starting gotomach function\n[ERROR] Invalid iteration count\n",
        ),
        (
            4,
            -1,
            0,
            0,
            -2,
            "[INFO] Starting gotomach function\n[ERROR] Invalid seed value\n",
        ),
        (
            4,
            65536,
            0,
            0,
            -2,
            "[INFO] Starting gotomach function\n[ERROR] Invalid seed value\n",
        ),
        (
            4,
            1,
            0,
            i32::MIN,
            0,
            "[INFO] Starting gotomach function\n[INFO] Processing completed successfully\n",
        ),
        (
            4,
            1,
            99,
            i32::MIN,
            0,
            "[INFO] Starting gotomach function\n[WARNING] Invalid mode, using default\n\
             [INFO] Processing completed successfully\n",
        ),
        (
            // mode 0 from seed 0 cycles produced values 10,20,...,1000 (sum
            // 50500 per 100 iterations) because current_value = produced % 1000
            // wraps to 0 at 1000. 65535 iterations = 655 full cycles + 35 more:
            // 655*50500 + (10+20+...+350) = 33077500 + 6300 = 33083800.
            65535,
            0,
            0,
            i32::MAX,
            33_083_800,
            "[INFO] Starting gotomach function\n[WARNING] Reached maximum count\n\
             [INFO] Processing completed successfully\n",
        ),
    ];
    for &(it, sd, m, th, want_ret, want_out) in cases {
        for which in [0usize, 1usize] {
            let side = if which == 0 { "C" } else { "Rust" };
            let (ret, out) = goto_out(which, it, sd, m, th);
            assert!(!out.is_empty(), "{side}: captured stdout was empty");
            assert_eq!(
                String::from_utf8_lossy(&out),
                want_out,
                "{side}: stdout bytes for gotomach({it}, {sd}, {m}, {th})"
            );
            assert_eq!(
                ret, want_ret,
                "{side}: return value for gotomach({it}, {sd}, {m}, {th})"
            );
        }
    }
}

/// Negative control: the comparison logic must actually be able to fail. We
/// feed `diff_gotomach`'s assertions two knowingly-different C invocations.
#[test]
fn harness_detects_a_real_divergence() {
    let (r_a, o_a) = goto_out(0, 4, 1, 0, i32::MIN);
    let (r_b, o_b) = goto_out(0, 4, 1, 0, i32::MAX);
    assert_ne!(
        (r_a, o_a.clone()),
        (r_b, o_b.clone()),
        "two different configurations produced identical (retval, stdout); the \
         harness could not distinguish anything"
    );
    // And a stdout-only difference (same return value, different log lines).
    let (r_c, o_c) = goto_out(0, 4, 1, 0, i32::MIN);
    let (r_d, o_d) = goto_out(0, 4, 1, 55, i32::MIN);
    assert_eq!(r_c, r_d, "expected the same return value here");
    assert_ne!(
        o_c, o_d,
        "expected the extra [WARNING] line to be visible in captured stdout"
    );
}
