//! Parity tests for the higher-level functions.
//!
//! `compare_allocations` branches on the relative addresses returned by two
//! independent `malloc(sizeof(int))` calls, and `arity4` (hence `arity2`,
//! `arity3` and `arity`) folds that result into its return value. The outcome is
//! therefore a deterministic function of the process-wide allocator state, not
//! of the arguments alone: calling C and Rust back to back in one process
//! compares two *different* heap states and proves nothing.
//!
//! So each side is exercised in a freshly spawned child process. Both children
//! run byte-identical code and `dlopen` both shared objects in the same order —
//! only the function pointers they invoke differ. Any divergence in the dumped
//! output is a genuine behavioural difference.

mod common;
use common::*;
use std::ffi::c_int;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

const LIB_ENV: &str = "HARVEST_DUMP_LIB";
const OUT_ENV: &str = "HARVEST_DUMP_OUT";

/// Inputs used for `compare_allocations`.
fn alloc_cases() -> Vec<(c_int, c_int)> {
    let vals: [c_int; 9] = [0, 1, -1, 2, -2, 100, -100, c_int::MIN, c_int::MAX];
    let mut out = Vec::new();
    for &a in &vals {
        for &b in &vals {
            out.push((a, b));
        }
    }
    out
}

/// Inputs used for `arity4` / `arity3` / `arity2`.
fn arity_params() -> Vec<[c_int; 4]> {
    let base: [c_int; 10] = [0, 1, 2, 3, 4, -1, -2, -3, 7, 100];
    let mut out = Vec::new();
    for &p1 in &base {
        for &p2 in &[0, 1, -1, 50, -50] {
            for &p3 in &[0, 1, -1, 3, 100, -100] {
                for &p4 in &[0, 7, -7] {
                    out.push([p1, p2, p3, p4]);
                }
            }
        }
    }
    // Extremes that make the internal arithmetic wrap.
    out.push([c_int::MAX, c_int::MAX, c_int::MAX, c_int::MAX]);
    out.push([c_int::MIN, c_int::MIN, c_int::MIN, c_int::MIN]);
    out.push([c_int::MAX, 1, -1, c_int::MIN]);
    out.push([c_int::MIN, -1, c_int::MAX, 1]);
    out.push([1, c_int::MAX, 2, c_int::MIN]);
    out.push([-2147483647, 2147483647, -1, 0]);
    out
}

/// `len` values fed to `arity`, chosen to probe the `unsigned char` truncation
/// in the definition versus the `int` in the public header.
fn arity_lens() -> Vec<c_int> {
    vec![
        0, 1, 2, 3, 4, 5, 6, 100, 127, 128, 200, 254, 255, 256, 257, 258, 259, 260, 511, 512, 1024,
        65538, -1, -2, -256, -255, i32::MIN, i32::MAX,
    ]
}

fn dump(which: &str, out_path: &Path) {
    let l = libs();
    let lib = match which {
        "c" => &l.c,
        "rust" => &l.rust,
        other => panic!("unknown library selector {other:?}"),
    };

    let compare_allocations = sym!(lib, "compare_allocations", Fn2);
    let arity4 = sym!(lib, "arity4", Fn4);
    let arity3 = sym!(lib, "arity3", Fn3);
    let arity2 = sym!(lib, "arity2", Fn2);
    let arity = sym!(lib, "arity", FnArity);

    let mut s = String::new();

    for (a, b) in alloc_cases() {
        let v = unsafe { compare_allocations(a, b) };
        writeln!(s, "compare_allocations {a} {b} -> {v}").unwrap();
    }

    for p in arity_params() {
        let v = unsafe { arity4(p[0], p[1], p[2], p[3]) };
        writeln!(s, "arity4 {} {} {} {} -> {v}", p[0], p[1], p[2], p[3]).unwrap();
    }

    for p in arity_params() {
        let v = unsafe { arity3(p[0], p[1], p[2]) };
        writeln!(s, "arity3 {} {} {} -> {v}", p[0], p[1], p[2]).unwrap();
    }

    for p in arity_params() {
        let v = unsafe { arity2(p[0], p[1]) };
        writeln!(s, "arity2 {} {} -> {v}", p[0], p[1]).unwrap();
    }

    for len in arity_lens() {
        // Always provide at least four readable elements.
        let mut params: Vec<c_int> = vec![11, -22, 33, -44, 55, 66, 77, 88];
        let v = unsafe { arity(len, params.as_mut_ptr()) };
        writeln!(s, "arity {len} -> {v}").unwrap();

        let mut params2: Vec<c_int> = vec![0, 0, 0, 0, 0, 0, 0, 0];
        let v2 = unsafe { arity(len, params2.as_mut_ptr()) };
        writeln!(s, "arity_zero {len} -> {v2}").unwrap();

        let mut params3: Vec<c_int> = vec![3, 100, 7, -9, 1, 2, 3, 4];
        let v3 = unsafe { arity(len, params3.as_mut_ptr()) };
        writeln!(s, "arity_mix {len} -> {v3}").unwrap();
    }

    std::fs::write(out_path, s.as_bytes()).expect("write dump");
}

/// Entry point used when this binary is re-invoked as a child process.
#[test]
fn child_dump() {
    let Ok(which) = std::env::var(LIB_ENV) else {
        return; // Not a child invocation: nothing to do.
    };
    let out = std::env::var(OUT_ENV).expect("child needs an output path");
    dump(&which, Path::new(&out));
}

fn run_child(which: &str, tag: &str) -> Vec<u8> {
    let exe = std::env::current_exe().expect("current_exe");
    let out: PathBuf = std::env::temp_dir().join(format!(
        "harvest_dump_{tag}_{}_{}.txt",
        std::process::id(),
        which
    ));
    let _ = std::fs::remove_file(&out);
    let status = Command::new(&exe)
        .args(["--exact", "child_dump", "--test-threads=1", "--quiet"])
        .env(LIB_ENV, which)
        .env(OUT_ENV, &out)
        .env(common::SKIP_BUILD_ENV, "1")
        .status()
        .expect("spawn child");
    assert!(status.success(), "child for {which} failed: {status}");
    let bytes = std::fs::read(&out).expect("read child dump");
    let _ = std::fs::remove_file(&out);
    assert!(!bytes.is_empty(), "child for {which} produced no output");
    bytes
}

#[test]
fn high_level_functions_match_byte_for_byte() {
    if std::env::var(LIB_ENV).is_ok() {
        return; // Child invocation; the dump test does the work.
    }

    // Force both artifacts to be (re)built before the children, which are told
    // to skip building, load them.
    let _ = libs();

    // The comparison is only meaningful if a single side is reproducible across
    // processes, so check that first.
    let c1 = run_child("c", "a");
    let c2 = run_child("c", "b");
    assert_eq!(
        c1, c2,
        "the C library is not reproducible across processes; \
         allocator-state dependent output cannot be compared this way"
    );

    let r1 = run_child("rust", "a");
    let r2 = run_child("rust", "b");
    assert_eq!(r1, r2, "the Rust library is not reproducible across processes");

    if c1 != r1 {
        let ct = String::from_utf8_lossy(&c1);
        let rt = String::from_utf8_lossy(&r1);
        let mut shown = 0;
        let mut total = 0;
        let mut report = String::new();
        for (cl, rl) in ct.lines().zip(rt.lines()) {
            if cl != rl {
                total += 1;
                if shown < 20 {
                    let _ = writeln!(report, "  C: {cl}\n  R: {rl}");
                    shown += 1;
                }
            }
        }
        panic!(
            "{total} differing line(s) between C and Rust (first {shown} shown):\n{report}\n\
             C lines: {}, Rust lines: {}",
            ct.lines().count(),
            rt.lines().count()
        );
    }
}
