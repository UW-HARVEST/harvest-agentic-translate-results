//! Phase C — error paths that terminate the process.
//!
//! ERRORS.md rows 6, 7, 16, 35, 36, 49, 51 are cases where the C dereferences
//! or frees an invalid pointer, or where a live `assert()` fires.  They cannot
//! be observed in-process, so each scenario is run out of process against ONE
//! library at a time and the two exit statuses (exit code AND terminating
//! signal) are compared.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const ENV_SCENARIO: &str = "HARVEST_CRASH_SCENARIO";
const ENV_LIB: &str = "HARVEST_CRASH_LIB";

/// The child driver.  A normal run (no env vars set) is a no-op so that this
/// test also passes as part of the ordinary suite.
#[test]
fn crash_child_driver() {
    let scenario = match std::env::var(ENV_SCENARIO) {
        Ok(s) => s,
        Err(_) => return,
    };
    let which = std::env::var(ENV_LIB).expect("HARVEST_CRASH_LIB");
    let (p, _g) = libs();
    let l = if which == "c" { &p.c } else { &p.rs };
    eprintln!("child: scenario={scenario} lib={which}");

    unsafe {
        match scenario.as_str() {
            // ERRORS.md row 7: free((header *) NULL - 1)
            "arrfreef_null" => (l.arrfreef)(std::ptr::null_mut()),

            // ERRORS.md row 16: the `*temp` store is unconditional
            "getkeyts_null_temp" => {
                let k = CKey::new(b"key");
                (l.hmget_key_ts)(
                    std::ptr::null_mut(),
                    16,
                    k.ptr() as *mut c_void,
                    8,
                    std::ptr::null_mut(),
                    HM_BINARY,
                );
            }

            // ERRORS.md row 49: `a->remaining` read with a == NULL
            "stralloc_null_arena" => {
                let k = CKey::new(b"payload");
                (l.stralloc)(std::ptr::null_mut(), k.ptr());
            }

            // ERRORS.md row 51: `a->storage` read with a == NULL
            "strreset_null" => (l.strreset)(std::ptr::null_mut()),

            // ERRORS.md row 6: `elemsize * min_cap + sizeof(header)` wraps.
            // es=4  -> 4*SIZE_MAX+32  == 28  (realloc succeeds, header write
            //          overruns the request but stays inside the glibc chunk)
            // es=16 -> 16*SIZE_MAX+32 == 16  (header write corrupts the heap;
            //          glibc detects it on free)
            "overflow_wrap_es4" => {
                let a = (l.arrgrowf)(std::ptr::null_mut(), 4, usize::MAX, 0);
                assert!(!a.is_null());
                let s = snap_arr(a, 4, ElemFmt::Raw, false);
                eprintln!("child: capacity={} length={}", s.capacity, s.length);
                (l.arrfreef)(a);
                eprintln!("child: survived");
            }
            "overflow_wrap_es16" => {
                let a = (l.arrgrowf)(std::ptr::null_mut(), 16, usize::MAX, 0);
                assert!(!a.is_null());
                let s = snap_arr(a, 16, ElemFmt::Raw, false);
                eprintln!("child: capacity={} length={}", s.capacity, s.length);
                (l.arrfreef)(a);
                eprintln!("child: survived");
            }
            // realloc of ~2^63 bytes fails -> b = NULL + 32 -> header store at 0
            "overflow_realloc_fail" => {
                let a = (l.arrgrowf)(std::ptr::null_mut(), 1, 1usize << 63, 0);
                eprintln!("child: returned {a:?}");
            }

            // ERRORS.md rows 35/36: deleting a NON-tail entry with a `mode`
            // that is >= STBDS_HM_STRING but != STBDS_HM_STRING makes the
            // post-memmove re-find hash the element's raw bytes, so
            // `assert(slot >= 0)` (or `assert(b->index[i] == final_index)`)
            // fires.
            "hmdel_mode2_middle" | "hmdel_mode1000_middle" => {
                let mode = if scenario == "hmdel_mode2_middle" {
                    2
                } else {
                    1000
                };
                (l.rand_seed)(DEFAULT_SEED);
                let mut h: *mut c_void = std::ptr::null_mut();
                let keys: Vec<CKey> = (0..6)
                    .map(|i| CKey::new(format!("middle-key-{i}").as_bytes()))
                    .collect();
                for k in &keys {
                    h = (l.hmput_key)(h, 16, k.ptr() as *mut c_void, 8, mode);
                }
                eprintln!("child: inserted, deleting a middle entry");
                (l.hmdel_key)(h, 16, keys[0].ptr() as *mut c_void, 8, 0, mode);
                eprintln!("child: survived");
            }

            other => panic!("unknown scenario {other}"),
        }
    }
}

fn run_child(scenario: &str, which: &str) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "crash_child_driver", "--nocapture", "--test-threads=1"])
        .env(ENV_SCENARIO, scenario)
        .env(ENV_LIB, which)
        .output()
        .expect("spawn child");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (out.status.code(), out.status.signal(), stderr)
}

#[track_caller]
fn assert_same_termination(scenario: &str) {
    let (cc, cs, cerr) = run_child(scenario, "c");
    let (rc, rs, rerr) = run_child(scenario, "rust");
    let survived = |code: Option<i32>, err: &str| code == Some(0) && err.contains("survived");
    assert_eq!(
        cs, rs,
        "[{scenario}] terminating signal differs: C={cs:?} RUST={rs:?}\n\
         --- C stderr ---\n{cerr}\n--- RUST stderr ---\n{rerr}"
    );
    if cs.is_none() {
        assert_eq!(
            cc, rc,
            "[{scenario}] exit code differs: C={cc:?} RUST={rc:?}\n\
             --- C stderr ---\n{cerr}\n--- RUST stderr ---\n{rerr}"
        );
        assert_eq!(
            survived(cc, &cerr),
            survived(rc, &rerr),
            "[{scenario}] one library survived and the other did not"
        );
        // when the call survives, the observable header state must match too
        let pick = |s: &str| -> Vec<String> {
            s.lines()
                .filter(|l| l.starts_with("child: capacity="))
                .map(|l| l.to_string())
                .collect()
        };
        assert_eq!(
            pick(&cerr),
            pick(&rerr),
            "[{scenario}] surviving-call header state differs"
        );
        assert!(
            !pick(&cerr).is_empty() || !survived(cc, &cerr),
            "[{scenario}] expected a header report from a surviving call"
        );
    }
    eprintln!("[{scenario}] both terminated with code={cc:?} signal={cs:?}");
}

#[test]
fn err07_arrfreef_null_same_abort() {
    assert_same_termination("arrfreef_null");
}

#[test]
fn err16_hmget_key_ts_null_temp_same_fault() {
    assert_same_termination("getkeyts_null_temp");
}

#[test]
fn err49_stralloc_null_arena_same_fault() {
    assert_same_termination("stralloc_null_arena");
}

#[test]
fn err51_strreset_null_same_fault() {
    assert_same_termination("strreset_null");
}

#[test]
fn err06_arrgrowf_size_overflow_same_outcome() {
    assert_same_termination("overflow_wrap_es4");
    assert_same_termination("overflow_wrap_es16");
    assert_same_termination("overflow_realloc_fail");
}

#[test]
fn err35_36_hmdel_wrong_string_mode_same_abort() {
    assert_same_termination("hmdel_mode2_middle");
    assert_same_termination("hmdel_mode1000_middle");
}
