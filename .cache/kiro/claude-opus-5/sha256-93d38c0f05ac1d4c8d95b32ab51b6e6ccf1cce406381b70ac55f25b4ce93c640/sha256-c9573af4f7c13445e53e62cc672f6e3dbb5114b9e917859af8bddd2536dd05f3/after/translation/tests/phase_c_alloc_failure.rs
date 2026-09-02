//! Phase C rows 3–5 — allocation-failure error returns.
//!
//! `strdup` / `malloc` / `realloc` are made to fail for ONE exact requested size
//! via an `LD_PRELOAD` interposer (`tests/support/failmalloc.c`), so the
//! injected failure lands on the call site under test and on nothing else. The
//! call happens in a child process (this binary re-executed with a filter)
//! because `LD_PRELOAD` has to be in place before the process starts.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const PROBE_ENV: &str = "W_UTF8_ALLOC_PROBE";

/// Length of the probe input. Chosen so the derived allocation sizes
/// (`LEN + 1` and `LEN + 1 + 4096`) are unlikely to collide with any unrelated
/// allocation in the child, and the input buffer itself is over-allocated so it
/// never requests those exact sizes.
const LEN: usize = 65521;

fn build_input(scenario: &str) -> Vec<u8> {
    // capacity != LEN + 1 on purpose: the Vec must not request a failing size
    let mut v: Vec<u8> = Vec::with_capacity(LEN + 4096 + 64);
    for i in 0..LEN {
        v.push(b'a' + (i % 26) as u8);
    }
    if scenario != "strdup_fail" {
        // one invalid byte -> takes the malloc/copy-loop path instead of strdup
        v[3] = 0xC0;
    }
    v.push(0);
    v
}

/// The child half — a no-op unless `W_UTF8_ALLOC_PROBE` is set.
#[test]
fn child_alloc_probe() {
    let spec = match std::env::var(PROBE_ENV) {
        Ok(v) => v,
        Err(_) => return,
    };
    let (which, scenario) = spec.split_once(':').expect("spec is <lib>:<scenario>");
    let path = match which {
        "c" => c_so_path(),
        "rust" => rust_so_path(),
        other => panic!("unknown lib {other}"),
    };
    let d = Driver::open(&path);
    let input = build_input(scenario);
    let replacement: u8 = if scenario == "realloc_fail" { 1 } else { 0 };
    let out = unsafe { (d.filter_fn)(input.as_ptr() as *const std::ffi::c_char, replacement) };
    if out.is_null() {
        println!("PROBE_RESULT:NULL");
    } else {
        let mut n = 0usize;
        unsafe {
            let mut p = out as *const u8;
            while *p != 0 {
                n += 1;
                p = p.add(1);
            }
            free(out as *mut std::ffi::c_void);
        }
        println!("PROBE_RESULT:OK:{n}");
    }
}

fn shim_path() -> PathBuf {
    let out_dir = crate_root().join("target/test-support");
    std::fs::create_dir_all(&out_dir).expect("create target/test-support");
    let so = out_dir.join("libfailmalloc.so");
    let src = crate_root().join("tests/support/failmalloc.c");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&so)
        .arg(&src)
        .arg("-ldl")
        .status()
        .expect("run cc to build the LD_PRELOAD shim");
    assert!(status.success(), "failed to compile {}", src.display());
    so
}

fn probe(shim: &PathBuf, which: &str, scenario: &str) -> Option<String> {
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(exe);
    cmd.args(["--exact", "child_alloc_probe", "--nocapture", "--test-threads=1"])
        .env(PROBE_ENV, format!("{which}:{scenario}"))
        .env("LD_PRELOAD", shim)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    match scenario {
        // strdup(s) internally calls malloc(strlen(s) + 1)
        "strdup_fail" => cmd.env("FAILMALLOC_SIZE", (LEN + 1).to_string()),
        // w_utf8_filter: malloc(strlen(string) + 1)
        "malloc_fail" => cmd.env("FAILMALLOC_SIZE", (LEN + 1).to_string()),
        // w_utf8_filter: realloc(copy, size + REPLACEMENT_INC)
        "realloc_fail" => cmd.env("FAILREALLOC_SIZE", (LEN + 1 + 4096).to_string()),
        other => panic!("unknown scenario {other}"),
    };
    let out = cmd.output().expect("spawn child alloc probe");
    let stdout = String::from_utf8_lossy(&out.stdout);
    extract(&stdout)
}

/// The child runs under the libtest harness with `--nocapture`, so the marker
/// can appear in the middle of a harness line; search for the marker itself.
fn extract(stdout: &str) -> Option<String> {
    let i = stdout.find("PROBE_RESULT:")? + "PROBE_RESULT:".len();
    Some(stdout[i..].split_whitespace().next().unwrap_or("").to_string())
}

/// Sanity check: without any injected failure both sides succeed, which proves
/// the harness itself (child re-exec + LD_PRELOAD shim) is not the thing making
/// the calls fail.
#[test]
fn err03_04_05_baseline_without_injection() {
    let shim = shim_path();
    let exe = std::env::current_exe().expect("current_exe");
    for scenario in ["strdup_fail", "malloc_fail", "realloc_fail"] {
        for which in ["c", "rust"] {
            let out = Command::new(&exe)
                .args(["--exact", "child_alloc_probe", "--nocapture", "--test-threads=1"])
                .env(PROBE_ENV, format!("{which}:{scenario}"))
                .env("LD_PRELOAD", &shim)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .expect("spawn");
            let stdout = String::from_utf8_lossy(&out.stdout);
            let res = extract(&stdout);
            assert!(
                res.as_deref().is_some_and(|r| r.starts_with("OK:")),
                "baseline for {which}/{scenario} should succeed, got {res:?}"
            );
        }
    }
}

#[test]
fn err03_strdup_failure_returns_null() {
    let shim = shim_path();
    let c = probe(&shim, "c", "strdup_fail");
    let r = probe(&shim, "rust", "strdup_fail");
    assert_eq!(c, r, "strdup failure: C={c:?} Rust={r:?}");
    assert_eq!(
        c.as_deref(),
        Some("NULL"),
        "the C code propagates strdup's NULL; got {c:?}"
    );
}

#[test]
fn err04_malloc_failure_returns_null() {
    let shim = shim_path();
    let c = probe(&shim, "c", "malloc_fail");
    let r = probe(&shim, "rust", "malloc_fail");
    assert_eq!(c, r, "malloc failure: C={c:?} Rust={r:?}");
    assert_eq!(
        c.as_deref(),
        Some("NULL"),
        "`if (copy == NULL) return NULL;` after malloc; got {c:?}"
    );
}

#[test]
fn err05_realloc_failure_returns_null() {
    let shim = shim_path();
    let c = probe(&shim, "c", "realloc_fail");
    let r = probe(&shim, "rust", "realloc_fail");
    assert_eq!(c, r, "realloc failure: C={c:?} Rust={r:?}");
    assert_eq!(
        c.as_deref(),
        Some("NULL"),
        "`if (copy == NULL) return NULL;` after realloc; got {c:?}"
    );
}
