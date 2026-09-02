//! Phase E — exact allocation-request parity.
//!
//! `FIO_createFilename_fromOutDir` calls `calloc(1, strlen(outDirName) + 1 +
//! strlen(filenameStart) + suffixLen + 1)`. The buffer-content comparison in
//! Phase B/C proves the *written* bytes match, and the `malloc_usable_size`
//! lower bound proves neither side under-allocates, but neither pins the exact
//! request. This file does, by LD_PRELOADing a `calloc` interposer (built from
//! `tests/support/calloc_probe.c`) into a child process and reading back the
//! recorded `(nmemb, size)` pair for each implementation.

mod harness;

use harness::*;
use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build (once) the LD_PRELOAD probe. Returns `None` if no C compiler is
/// available, in which case the exact-request tests are skipped rather than
/// reported as failures.
fn probe_so() -> Option<PathBuf> {
    let src = manifest_dir().join("tests/support/calloc_probe.c");
    let out = manifest_dir().join("target/calloc_probe.so");
    if out.is_file() {
        let s_ok = std::fs::metadata(&src).and_then(|m| m.modified()).ok();
        let o_ok = std::fs::metadata(&out).and_then(|m| m.modified()).ok();
        if let (Some(s), Some(o)) = (s_ok, o_ok) {
            if o >= s {
                return Some(out);
            }
        }
    }
    std::fs::create_dir_all(out.parent().unwrap()).ok()?;
    for cc in ["cc", "gcc", "clang"] {
        let st = Command::new(cc)
            .args(["-shared", "-fPIC", "-O1", "-o"])
            .arg(&out)
            .arg(&src)
            .arg("-ldl")
            .status();
        if matches!(st, Ok(s) if s.success()) {
            return Some(out);
        }
    }
    None
}

// --- child side -------------------------------------------------------------

const CASE_ENV: &str = "DIFFTEST_ALLOC_CASE";

unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}
const RTLD_DEFAULT: *mut c_void = std::ptr::null_mut();

fn probe_fn(name: &[u8]) -> Option<extern "C" fn() -> usize> {
    let p = unsafe { dlsym(RTLD_DEFAULT, name.as_ptr() as *const c_char) };
    if p.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute::<*mut c_void, extern "C" fn() -> usize>(p) })
    }
}

fn probe_void(name: &[u8]) -> Option<extern "C" fn()> {
    let p = unsafe { dlsym(RTLD_DEFAULT, name.as_ptr() as *const c_char) };
    if p.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute::<*mut c_void, extern "C" fn()>(p) })
    }
}

/// Cases are encoded as `outdir|path|suffixLen` so the parent and child agree
/// without any shared state beyond the environment.
#[test]
fn child_helper_alloc_probe() {
    let Some(imp) = child_impl() else { return };
    let case = std::env::var(CASE_ENV).expect("case env");
    let mut it = case.split('|');
    let out_s = it.next().unwrap().to_string();
    let path_s = it.next().unwrap().to_string();
    let sfx: usize = it.next().unwrap().parse().unwrap();

    let out = cstr(out_s.as_bytes());
    let path = cstr(path_s.as_bytes());

    let arm = probe_void(b"calloc_probe_arm\0").expect("calloc_probe not preloaded");
    let disarm = probe_void(b"calloc_probe_disarm\0").unwrap();
    let count = probe_fn(b"calloc_probe_count\0").unwrap();
    let nmemb = probe_fn(b"calloc_probe_nmemb\0").unwrap();
    let size = probe_fn(b"calloc_probe_size\0").unwrap();

    arm();
    let r = unsafe {
        (imp.fio_create)(
            path.as_ptr() as *const c_char,
            out.as_ptr() as *const c_char,
            sfx,
        )
    };
    disarm();
    println!(
        "PROBE calls={} nmemb={} size={} null={}",
        count(),
        nmemb(),
        size(),
        r.is_null()
    );
    // Leak deliberately: freeing is irrelevant here and the process exits next.
    std::process::exit(0);
}

// --- parent side ------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct ProbeResult {
    /// Every `(nmemb, size)` pair the library requested while armed.
    records: Vec<(usize, usize)>,
    /// `Some(is_null)` when the call returned; `None` when the process died
    /// inside the library (the `exit(30)` allocation-failure path).
    returned_null: Option<bool>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn run_probe(probe: &Path, which: &str, case: &str) -> ProbeResult {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args([
            "--exact",
            "child_helper_alloc_probe",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_ENV, which)
        .env(CASE_ENV, case)
        .env("LD_PRELOAD", probe)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn probe child");

    // The interposer writes every armed request to fd 2 immediately, so the
    // record exists even when the library exit()s before returning.
    let err = String::from_utf8_lossy(&out.stderr);
    let mut records: Vec<(usize, usize)> = Vec::new();
    for line in err.lines() {
        if let Some(i) = line.find("PROBEC ") {
            let l = &line[i..];
            let f = |k: &str| -> usize {
                l.split_whitespace()
                    .find_map(|t| t.strip_prefix(k))
                    .unwrap_or_else(|| panic!("bad PROBEC line {l:?}"))
                    .parse()
                    .unwrap()
            };
            records.push((f("nmemb="), f("size=")));
        }
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let returned_null = text.contains("PROBE ").then(|| {
        let line = text.lines().find(|l| l.contains("PROBE ")).unwrap();
        let line = &line[line.find("PROBE ").unwrap()..];
        line.split_whitespace()
            .find_map(|t| t.strip_prefix("null="))
            .map(|v| v == "true")
            .unwrap()
    });

    ProbeResult {
        records,
        returned_null,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn phase_e_exact_calloc_request_matches() {
    let Some(probe) = probe_so() else {
        eprintln!("no C compiler available; skipping exact calloc-request parity");
        return;
    };

    // Sanity: the preload must really be active, otherwise the whole test is
    // vacuous. `run_probe` panics if the probe symbols are missing.
    let mut cases: Vec<String> = Vec::new();
    let outs = ["o", "out/", "out", "/", "a/b/c", "a/b/c/", "\u{1}\u{7f}x/"];
    let paths = ["", "f", "/f", "d/f", "a/b/c.txt", "a/b/", "/", "//", "x"];
    for o in outs {
        for p in paths {
            for sfx in [0usize, 1, 5, 64, 4096] {
                cases.push(format!("{o}|{p}|{sfx}"));
            }
        }
    }
    // Wrapping / oversized requests too.
    for o in ["o", "out/"] {
        for p in ["f", "d/f"] {
            cases.push(format!("{o}|{p}|{}", usize::MAX));
            cases.push(format!("{o}|{p}|{}", 1usize << 63));
        }
    }

    let mut checked = 0usize;
    for case in &cases {
        let c = run_probe(&probe, "c", case);
        let r = run_probe(&probe, "rust", case);
        assert!(
            !c.records.is_empty(),
            "probe recorded no calloc for the C implementation (case {case}) — the \
             LD_PRELOAD interposer is not active, so the test would be vacuous: {c:?}"
        );
        assert_eq!(
            c, r,
            "calloc request / termination mismatch for case {case}:\n  C   = {c:?}\n  Rust= {r:?}"
        );

        // Cross-check the recorded request against the C source formula:
        //   calloc(1, strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1)
        let mut it = case.split('|');
        let o = it.next().unwrap();
        let p = it.next().unwrap();
        let sfx: usize = it.next().unwrap().parse().unwrap();
        let base = p.rsplit_once('/').map(|(_, t)| t).unwrap_or(p);
        let expect = o
            .len()
            .wrapping_add(1)
            .wrapping_add(base.len())
            .wrapping_add(sfx)
            .wrapping_add(1);
        assert_eq!(
            c.records[0],
            (1usize, expect),
            "case {case}: C calloc request {:?} does not match the source formula (1, {expect})",
            c.records[0]
        );
        checked += 1;
    }
    assert!(checked >= 300, "expected a broad case sweep, only ran {checked}");
    eprintln!("phase E: {checked} exact calloc requests matched");
}
