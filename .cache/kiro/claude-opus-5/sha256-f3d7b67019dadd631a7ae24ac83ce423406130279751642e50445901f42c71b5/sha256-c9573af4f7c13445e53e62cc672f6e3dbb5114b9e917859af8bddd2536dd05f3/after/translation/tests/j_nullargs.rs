//! Phase C: NULL-pointer / zero-argument rejection for EVERY exported symbol.
//!
//! `ERRORS.md` contains 126 `return 0 / NULL / -1` rejection sentinels, most of
//! which are the `if (png_ptr == NULL) return ...` guards at the top of the
//! public functions.  This suite calls all 313 exported entry points with every
//! argument zeroed and compares the observable result between the two `.so`s.
//!
//! Each call runs in its OWN subprocess, so a genuine C-side dereference of NULL
//! is itself a comparable observable (the exit signal) instead of aborting the
//! run.  Divergence in *either* the returned value or the exit status fails.
mod common;
use common::*;
use std::process::Command;

include!("common/null_cases.rs");

const ENV_CASE: &str = "PNG_NULL_CASE";
const ENV_LIB: &str = "PNG_NULL_LIB";

/// Child mode: run exactly one case against one library and print the result.
#[test]
fn null_args_child() {
    let Ok(case) = std::env::var(ENV_CASE) else {
        // Parent mode: nothing to do, the driver test does the work.
        return;
    };
    let idx: usize = case.parse().unwrap();
    let which = std::env::var(ENV_LIB).unwrap_or_else(|_| "C".into());
    preload();
    let l = if which == "C" {
        Lib {
            api: Api::open(C_SO, "C"),
            pv: Priv::open(C_SO, "C"),
            which: "C",
        }
    } else {
        Lib {
            api: Api::open(RUST_SO, "RUST"),
            pv: Priv::open(RUST_SO, "RUST"),
            which: "RUST",
        }
    };
    if idx == usize::MAX - 1 {
        // Special case: valid png_ptr, NULL chunk name.
        let rep = write_session(&l, &mut |l, png, _info| unsafe {
            (l.api.png_write_sig)(png);
            (l.api.png_write_chunk_start)(png, std::ptr::null(), 0);
            log("survived");
        });
        println!("RESULT png_write_chunk_start_valid_ptr out={}", rep.out.len());
        println!("WARN {:?}", rep.warnings);
        println!("ERR {:?}", rep.error);
        return;
    }
    let mut ctxb = Box::new(Ctx::default());
    set_ctx(&mut *ctxb as *mut Ctx);
    let res = run_case(&l, idx);
    println!("RESULT {} {}", NAMES[idx], res);
    println!("WARN {:?}", ctxb.warnings);
    println!("ERR {:?}", ctxb.error);
    set_ctx(std::ptr::null_mut());
}

fn preload() {
    // Same libm/libz pre-load the harness does; see common::libs().
    let _ = libs();
}

/// Symbols whose all-NULL behaviour is DOUBLY undefined in the reference C and
/// therefore not comparable.  Each entry records exactly why.
///
/// * `png_write_chunk_start` — the C is
///   `png_write_chunk_header(png_ptr, PNG_CHUNK_FROM_STRING(chunk_string), length)`
///   and `PNG_CHUNK_FROM_STRING` expands to `chunk_string[0..3]`, so the chunk
///   name is dereferenced at the call site *before* `png_write_chunk_header`
///   performs its `png_ptr == NULL` early return.  With BOTH pointers NULL the C
///   faults on the name load while the Rust build, which contains the identical
///   dereference, has it elided by the optimiser because the value is unused on
///   the NULL-`png_ptr` path.  Neither behaviour is defined.  The comparable
///   case — a VALID `png_ptr` with a NULL chunk name — is checked by
///   `null_chunk_name_with_valid_png_ptr_matches` below: both implementations
///   fault there, identically.
const UB_EXCLUDED: &[&str] = &["png_write_chunk_start"];

#[derive(PartialEq, Debug)]
struct Outcome {
    status: String,
    stdout: String,
}

fn run_child(exe: &str, idx: usize, which: &str) -> Outcome {
    let out = Command::new("timeout")
        .arg("20")
        .arg(exe)
        .arg("--exact")
        .arg("null_args_child")
        .arg("--nocapture")
        .env(ENV_CASE, idx.to_string())
        .env(ENV_LIB, which)
        .output()
        .expect("spawn child");
    let so = String::from_utf8_lossy(&out.stdout);
    // Keep only the deterministic RESULT/WARN/ERR lines.
    let kept: Vec<&str> = so
        .lines()
        .filter(|l| l.starts_with("RESULT ") || l.starts_with("WARN ") || l.starts_with("ERR "))
        .collect();
    Outcome {
        status: format!("{:?}", out.status),
        stdout: kept.join("\n"),
    }
}

#[test]
fn null_args_all_symbols() {
    if std::env::var(ENV_CASE).is_ok() {
        return; // we are the child
    }
    let exe = std::env::current_exe().unwrap();
    let exe = exe.to_str().unwrap().to_string();
    let mut diverged: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for i in 0..NCASES {
        if UB_EXCLUDED.contains(&NAMES[i]) {
            continue;
        }
        checked += 1;
        let a = run_child(&exe, i, "C");
        let b = run_child(&exe, i, "RUST");
        if a != b {
            diverged.push(format!(
                "{} (case {i}):\n    C   : status={} out={:?}\n    RUST: status={} out={:?}",
                NAMES[i], a.status, a.stdout, b.status, b.stdout
            ));
        }
    }
    assert!(
        diverged.is_empty(),
        "NULL-argument behaviour diverges for {} of {} symbols:\n{}",
        diverged.len(),
        checked,
        diverged.join("\n")
    );
    assert_eq!(
        checked,
        NCASES - UB_EXCLUDED.len(),
        "unexpected number of symbols checked"
    );
}

/// The comparable half of the `png_write_chunk_start` UB exclusion: with a valid
/// `png_ptr` and a NULL chunk name, BOTH implementations must behave the same.
#[test]
fn null_chunk_name_with_valid_png_ptr_matches() {
    if std::env::var(ENV_CASE).is_ok() {
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let exe = exe.to_str().unwrap().to_string();
    let a = run_child(&exe, usize::MAX - 1, "C");
    let b = run_child(&exe, usize::MAX - 1, "RUST");
    assert_eq!(
        a, b,
        "png_write_chunk_start(valid png_ptr, NULL name) diverges"
    );
    // Both are expected to fault on the name load; assert that we really did
    // observe a signal rather than a silent success in both.
    assert!(
        a.status.contains("signal") || a.status.contains("unix_wait_status(11)"),
        "expected both implementations to fault, got {}",
        a.status
    );
}
