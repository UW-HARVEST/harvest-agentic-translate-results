//! Phase C — error paths that necessarily terminate the process
//! (`assert` → `abort`, or a NULL/wild dereference → `SIGSEGV`).
//!
//! Each scenario is run in a fresh child process, once against the C `.so` and
//! once against the Rust `.so`, and the *termination outcome* (exit code or
//! fatal signal) must be identical.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::process::{Command, Stdio};

const SCENARIOS: &[&str] = &[
    "arrfreef_null",          // ERRORS row 40
    "hmget_ts_null_temp",     // ERRORS row 41
    "arrgrowf_oversize",      // ERRORS row 44 (elemsize*min_cap wraps: no failure)
    "arrgrowf_realloc_fail",  // ERRORS row 44 (realloc really returns NULL)
    "stralloc_null_storage",  // ERRORS row 26
    "sh_none_string_lookup",  // CONFIGS row 24 / lookup half
    "str_dups_ok",            // ERRORS rows 30, 31, 32 (asserts must NOT fire)
    "hmdel_after_hmfree",     // use-after-free: must fail the same way
];

/// Runs one scenario against one implementation inside this test binary.
unsafe fn run_scenario(name: &str, api: &Api) {
    match name {
        // ERRORS row 40: free(stbds_header(NULL)) == free((char*)0 - 32)
        "arrfreef_null" => {
            (api.arrfreef)(std::ptr::null_mut());
        }
        // ERRORS row 41: `*temp = STBDS_INDEX_EMPTY` with temp == NULL
        "hmget_ts_null_temp" => {
            let mut key = [7u8; 8];
            (api.hmget_key_ts)(
                std::ptr::null_mut(),
                16,
                key.as_mut_ptr() as *mut c_void,
                8,
                std::ptr::null_mut(),
                STBDS_HM_BINARY,
            );
        }
        // ERRORS row 44, wrapping variant: `elemsize * min_cap + sizeof(header)`
        // wraps to a *small* value, so realloc succeeds and nothing faults --
        // but the returned capacity is a lie. Both must agree.
        "arrgrowf_oversize" => {
            let a = (api.arrgrowf)(std::ptr::null_mut(), usize::MAX / 2, 0, 4);
            assert!(!a.is_null(), "the size computation wraps, so realloc succeeds");
            std::hint::black_box(a);
        }
        // ERRORS row 44: realloc() genuinely fails, then the code writes the
        // header through NULL + sizeof(stbds_array_header).
        "arrgrowf_realloc_fail" => {
            let a = (api.arrgrowf)(std::ptr::null_mut(), 1, 0, usize::MAX / 2);
            std::hint::black_box(a);
        }
        // ERRORS row 26: an arena claiming `remaining` bytes but with no block
        "stralloc_null_storage" => {
            let mut a = StringArena {
                storage: std::ptr::null_mut(),
                remaining: 1 << 20,
                block: 0,
                mode: 0,
            };
            let mut s = *b"hi\0";
            let p = (api.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            std::hint::black_box(p);
        }
        // SH_NONE + string mode: the `default:` branch memcpy'd string bytes into
        // the element, so any lookup reinterprets them as a `char *`.
        "sh_none_string_lookup" => {
            let e = 16usize;
            let m = (api.shmode_func)(e, STBDS_SH_NONE);
            let mut key = *b"abcdefghijklmnop\0";
            let m = (api.hmput_key)(m, e, key.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            let m = (api.hmget_key)(m, e, key.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            std::hint::black_box(m);
        }
        // ERRORS rows 30-32: the three `str_dups` asserts must never fire.
        "str_dups_ok" => {
            for n in [0i32, 1, 5, 100, 1000] {
                (api.str_dups)(n);
            }
        }
        // Use-after-free of the whole map: both must fault the same way.
        "hmdel_after_hmfree" => {
            let e = 16usize;
            let m = (api.shmode_func)(e, STBDS_SH_STRDUP);
            let mut key = *b"key\0";
            let m = (api.hmput_key)(m, e, key.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            (api.hmfree_func)(hash_to_arr(m, e), e);
            // deliberately touch the freed map
            let m2 = (api.hmdel_key)(m, e, key.as_mut_ptr() as *mut c_void, 8, 0, STBDS_HM_STRING);
            std::hint::black_box(m2);
        }
        other => panic!("unknown scenario {other}"),
    }
}

/// The child-process entry point. A no-op unless `SD_SCENARIO` is set.
#[test]
fn scenario_runner() {
    let Ok(name) = std::env::var("SD_SCENARIO") else {
        return;
    };
    let imp = std::env::var("SD_IMPL").expect("SD_IMPL");
    let p = pair();
    let api: &Api = if imp == "c" { &p.c } else { &p.rs };
    unsafe { run_scenario(&name, api) }
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exit(i32),
    Signal(i32),
}

fn spawn(scenario: &str, imp: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let st = Command::new(exe)
        .args(["--exact", "scenario_runner", "--test-threads=1", "--nocapture"])
        .env("SD_SCENARIO", scenario)
        .env("SD_IMPL", imp)
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");
    if let Some(sig) = st.signal() {
        Outcome::Signal(sig)
    } else {
        Outcome::Exit(st.code().unwrap_or(-1))
    }
}

/// Every crashing/aborting scenario must terminate identically in both
/// implementations.
#[test]
fn crash_equivalence_all_scenarios() {
    // Skip when we *are* the child (SD_SCENARIO is set): the child only runs
    // `scenario_runner`, so this is belt-and-braces.
    if std::env::var("SD_SCENARIO").is_ok() {
        return;
    }
    let mut report = Vec::new();
    for s in SCENARIOS {
        let c = spawn(s, "c");
        let r = spawn(s, "rs");
        report.push(format!("{s}: C={c:?} Rust={r:?}"));
        assert_eq!(c, r, "scenario `{s}` terminated differently: C={c:?} Rust={r:?}");
    }
    for line in &report {
        println!("{line}");
    }
    // `str_dups_ok` must be the one scenario that exits cleanly.
    assert_eq!(
        spawn("str_dups_ok", "c"),
        Outcome::Exit(0),
        "str_dups asserts must not fire in C"
    );
    assert_eq!(
        spawn("str_dups_ok", "rs"),
        Outcome::Exit(0),
        "str_dups asserts must not fire in Rust"
    );
}
