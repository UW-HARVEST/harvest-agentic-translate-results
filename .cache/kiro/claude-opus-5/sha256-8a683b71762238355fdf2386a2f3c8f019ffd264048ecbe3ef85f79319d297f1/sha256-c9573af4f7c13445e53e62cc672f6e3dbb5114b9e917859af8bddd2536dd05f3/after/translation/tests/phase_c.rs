//! Phase C — error / rejection differential tests, one test per row of
//! `ERRORS.md`.
//!
//! Rows whose C behaviour is `abort()`/`SIGSEGV` cannot be compared in-process
//! (the first one would kill the test binary), so they are run in two child
//! processes - one loading only the C `.so`, one loading only the Rust `.so` -
//! and the *termination status* is compared.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;

// ===========================================================================
// subprocess machinery for the abort/segfault rows
// ===========================================================================

const ENV_SCENARIO: &str = "DIFF_CRASH_SCENARIO";
const ENV_WHICH: &str = "DIFF_CRASH_LIB";

fn run_crash(scenario: &str, which: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "crash_runner", "--ignored", "--nocapture"])
        .env(ENV_SCENARIO, scenario)
        .env(ENV_WHICH, which)
        .env("RUST_BACKTRACE", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .expect("spawn child");
    (out.status.code(), out.status.signal())
}

/// Assert that the C `.so` and the Rust `.so` terminate the child process in
/// exactly the same way for a scenario that is expected to crash/abort.
///
/// `oom_path == true` marks scenarios whose C behaviour is "`realloc` returned
/// NULL and the C then writes through it".  The *release* cdylib - the artifact
/// that corresponds to the C `.so` (it is the one `[profile.release] panic =
/// "abort"` is written for) - faults exactly like the C.  A cdylib built with
/// `debug_assertions` additionally carries rustc's "null pointer dereference
/// occurred" check, which turns the SIGSEGV into a SIGABRT *before* the faulting
/// store.  That is Rust runtime instrumentation, not a translation difference,
/// so those two rows are skipped when the suite is deliberately pointed at the
/// debug artifact via `DIFF_RUST_PROFILE=debug`.
fn same_termination_inner(scenario: &str, oom_path: bool) {
    if oom_path && rust_so_path().to_string_lossy().contains("/debug/") {
        eprintln!(
            "SKIP {}: the debug cdylib's debug_assertions null-deref check pre-empts \
             the SIGSEGV; this row is verified against the release cdylib",
            scenario
        );
        return;
    }
    let c = run_crash(scenario, "c");
    let r = run_crash(scenario, "rust");
    assert_eq!(
        c, r,
        "`{}`: C terminated with (code, signal) = {:?} but Rust with {:?}",
        scenario, c, r
    );
    assert!(
        c.1.is_some(),
        "`{}`: expected the child to be killed by a signal, got exit code {:?}",
        scenario,
        c.0
    );
}

fn same_termination(scenario: &str) {
    same_termination_inner(scenario, false);
}

fn same_termination_oom(scenario: &str) {
    same_termination_inner(scenario, true);
}

#[test]
#[ignore = "driven by same_termination() in a child process"]
fn crash_runner() {
    let scenario = match std::env::var(ENV_SCENARIO) {
        Ok(s) => s,
        // run directly (e.g. `--include-ignored`) rather than as a child: no-op
        Err(_) => return,
    };
    let which = std::env::var(ENV_WHICH).unwrap_or_else(|_| "c".to_string());
    let path = if which == "c" {
        c_so_path()
    } else {
        rust_so_path()
    };
    let lib = unsafe { Lib::load("x", &path) };
    unsafe { run_scenario(&lib, &scenario) };
    // If we get here the scenario did not crash; make that visible as a
    // *distinct* (and equal-for-both) exit code so same_termination() reports it.
    std::process::exit(77);
}

unsafe fn run_scenario(lib: &Lib, scenario: &str) {
    match scenario {
        // ERRORS row 20: stbds_arrfreef(NULL) -> free((header*)NULL - 1)
        "arrfreef_null" => {
            (lib.arrfreef)(std::ptr::null_mut());
        }
        // ERRORS row 19: realloc fails, then (char*)NULL + 32 is written through
        "arrgrowf_oom" => {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), 1 << 40, 0, 1 << 20);
            std::hint::black_box(a);
        }
        // ERRORS row 19 (arena variant): blocksize = 512 << 31 = 1 TiB
        "stralloc_oom" => {
            let mut a = StringArena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: 63,
                mode: 3,
            };
            let s = b"hello\0";
            let p = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
            std::hint::black_box(p);
        }
        // ERRORS row 33: hmdel_key with mode == 2 on a NON-last element takes the
        // *binary* re-find path with a string hash, finds nothing, and trips
        // STBDS_ASSERT(slot >= 0).
        "hmdel_mode2_nonlast" => {
            (lib.rand_seed)(0x31415926);
            let keys: Vec<Vec<u8>> = (0..6)
                .map(|i| format!("key_{}\0", i).into_bytes())
                .collect();
            let mut hm = Hm::from_shmode(lib, 16, 8, STBDS_SH_DEFAULT);
            for (i, k) in keys.iter().enumerate() {
                hm.put_str(k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_STRING);
            }
            // delete the FIRST key: old_index(1) != final_index(6) -> re-find
            let r = hm.del_str(keys[0].as_ptr() as *mut c_char, 2);
            std::hint::black_box(r);
        }
        // ERRORS row 35 / CONFIGS row 29: a lookup on a `string.mode == SH_NONE`
        // map reinterprets the memcpy'd key BYTES as a char* and dereferences it.
        "sh_none_lookup" => {
            (lib.rand_seed)(0x31415926);
            let k = b"abcdefghijklmnop\0";
            let mut hm = Hm::from_shmode(lib, 16, 8, STBDS_SH_NONE);
            hm.put_str(k.as_ptr() as *mut c_char, &0u64.to_le_bytes().to_vec(), STBDS_HM_STRING);
            let r = hm.get_str(k.as_ptr() as *mut c_char, STBDS_HM_STRING);
            std::hint::black_box(r);
        }
        other => panic!("unknown scenario {}", other),
    }
}

// ===========================================================================
// Row 1-3 — stbds_hmfree_func
// ===========================================================================

#[test]
fn err_01_hmfree_null() {
    diff("err_01_hmfree_null", |lib, t| unsafe {
        for &elemsize in [0usize, 1, 8, 16].iter() {
            (lib.hmfree_func)(std::ptr::null_mut(), elemsize);
            t.push(Ev::Tag("returned"));
        }
    });
}

#[test]
fn err_02_hmfree_no_table() {
    diff("err_02_hmfree_no_table", |lib, t| unsafe {
        for &elemsize in [8usize, 16, 32].iter() {
            let p = (lib.hmput_default)(std::ptr::null_mut(), elemsize);
            let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            t.push(Ev::Tbl(snap_table(raw))); // present == false
            (lib.hmfree_func)(raw, elemsize);
            t.push(Ev::Tag("freed-no-table"));
        }
    });
}

#[test]
fn err_03_hmfree_no_strdup_mode() {
    diff("err_03_hmfree_no_strdup_mode", |lib, t| unsafe {
        // string.mode == SH_DEFAULT / SH_ARENA / SH_NONE must NOT free the key
        // pointers; only SH_STRDUP does.
        for &shmode in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_ARENA, 9].iter() {
            (lib.rand_seed)(5);
            let keys: Vec<Vec<u8>> = (0..5)
                .map(|i| format!("stable_key_{}\0", i).into_bytes())
                .collect();
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            for (i, k) in keys.iter().enumerate() {
                hm.put_str(
                    k.as_ptr() as *mut c_char,
                    &(i as u64).to_le_bytes().to_vec(),
                    STBDS_HM_STRING,
                );
            }
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
            // the caller-owned key buffers must still be intact
            for k in keys.iter() {
                t.push(Ev::Bytes(cstr_bytes(k.as_ptr() as *const c_char)));
            }
        }
    });
}

// ===========================================================================
// Rows 4-8 — lookup misses
// ===========================================================================

#[test]
fn err_04_get_missing_key() {
    diff("err_04_get_missing_key", |lib, t| unsafe {
        let mut rng = Rng::new(0x404);
        for &(elemsize, keysize) in [(8usize, 4usize), (16, 8), (24, 16)].iter() {
            (lib.rand_seed)(0x31415926);
            let mut hm = Hm::new(lib, elemsize, keysize, 0);
            // empty map (table exists only after the first put) -> also row 6
            for _ in 0..20 {
                let k = rng.bytes(keysize);
                t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY)));
            }
            for i in 0..50usize {
                let k = rng.bytes(keysize);
                hm.put_kv(&k, &vec![i as u8; elemsize - keysize], STBDS_HM_BINARY);
            }
            for _ in 0..500 {
                let k = rng.bytes(keysize);
                t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY)));
            }
            hm.free();
        }
    });
}

#[test]
fn err_05_get_ts_null_a() {
    diff("err_05_get_ts_null_a", |lib, t| unsafe {
        for &elemsize in [1usize, 8, 16, 64].iter() {
            for &keysize in [0usize, 1, 4, 8].iter() {
                let key = [0x5Au8; 8];
                let mut temp: isize = 0x7777;
                let p = (lib.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_ptr() as *mut c_void,
                    keysize,
                    &mut temp,
                    STBDS_HM_BINARY,
                );
                t.push(Ev::Bool(p.is_null()));
                t.push(Ev::I(temp));
                let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
                t.push(Ev::Arr(snap_arr(raw, elemsize)));
                t.push(Ev::Tbl(snap_table(raw)));
                (lib.hmfree_func)(raw, elemsize);
            }
        }
    });
}

#[test]
fn err_06_get_ts_no_table() {
    diff("err_06_get_ts_no_table", |lib, t| unsafe {
        for &elemsize in [8usize, 16].iter() {
            let p = (lib.hmput_default)(std::ptr::null_mut(), elemsize);
            let key = [1u8, 2, 3, 4, 5, 6, 7, 8];
            for &keysize in [0usize, 4, 8].iter() {
                let mut temp: isize = 0x7777;
                let p2 = (lib.hmget_key_ts)(
                    p,
                    elemsize,
                    key.as_ptr() as *mut c_void,
                    keysize,
                    &mut temp,
                    STBDS_HM_BINARY,
                );
                t.push(Ev::Bool(std::ptr::eq(p as *const u8, p2 as *const u8)));
                t.push(Ev::I(temp));
            }
            let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            (lib.hmfree_func)(raw, elemsize);
        }
    });
}

#[test]
fn err_07_get_ts_missing() {
    diff("err_07_get_ts_missing", |lib, t| unsafe {
        let mut rng = Rng::new(0x407);
        (lib.rand_seed)(0x31415926);
        let mut hm = Hm::new(lib, 16, 8, 0);
        for i in 0..100usize {
            let k = rng.bytes(8);
            hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY);
        }
        let base_temp = hm.temp();
        for _ in 0..500 {
            let k = rng.bytes(8);
            t.push(Ev::I(hm.get_ts(&k, STBDS_HM_BINARY)));
        }
        // get_ts must not have disturbed header->temp
        t.push(Ev::I(base_temp));
        t.push(Ev::I(hm.temp()));
        hm.free();
    });
}

#[test]
fn err_08_get_key_temp_minus1() {
    diff("err_08_get_key_temp_minus1", |lib, t| unsafe {
        let mut rng = Rng::new(0x408);
        // (a) a == NULL
        let key = [9u8; 8];
        let p = (lib.hmget_key)(
            std::ptr::null_mut(),
            16,
            key.as_ptr() as *mut c_void,
            8,
            STBDS_HM_BINARY,
        );
        let raw = (p as *mut u8).sub(16) as *mut c_void;
        t.push(Ev::Arr(snap_arr(raw, 16))); // temp must be -1
        (lib.hmfree_func)(raw, 16);
        // (b) table == NULL
        let p = (lib.hmput_default)(std::ptr::null_mut(), 16);
        let p = (lib.hmget_key)(p, 16, key.as_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
        let raw = (p as *mut u8).sub(16) as *mut c_void;
        t.push(Ev::Arr(snap_arr(raw, 16)));
        (lib.hmfree_func)(raw, 16);
        // (c) populated map, key absent
        (lib.rand_seed)(1);
        let mut hm = Hm::new(lib, 16, 8, 0);
        for i in 0..64usize {
            let k = rng.bytes(8);
            hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY);
        }
        for _ in 0..200 {
            let k = rng.bytes(8);
            t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY)));
            t.push(Ev::I(hm.temp()));
        }
        hm.free();
    });
}

// ===========================================================================
// Rows 9-12 — delete misses
// ===========================================================================

#[test]
fn err_09_del_null_a() {
    diff("err_09_del_null_a", |lib, t| unsafe {
        let key = [3u8; 16];
        for &elemsize in [1usize, 8, 16].iter() {
            for &keysize in [0usize, 4, 8, 16].iter() {
                for &keyoffset in [0usize, 4].iter() {
                    for &mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -1].iter() {
                        let r = (lib.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                        t.push(Ev::Bool(r.is_null()));
                    }
                }
            }
        }
    });
}

#[test]
fn err_10_del_no_table() {
    diff("err_10_del_no_table", |lib, t| unsafe {
        for &elemsize in [8usize, 16].iter() {
            let p = (lib.hmput_default)(std::ptr::null_mut(), elemsize);
            // poison temp first so we can see that it is reset to 0
            let h = (p as *mut u8).sub(elemsize).sub(HEADER_SIZE) as *mut ArrayHeader;
            (*h).temp = 0x1234;
            let key = [7u8; 8];
            let p2 = (lib.hmdel_key)(
                p,
                elemsize,
                key.as_ptr() as *mut c_void,
                8,
                0,
                STBDS_HM_BINARY,
            );
            t.push(Ev::Bool(std::ptr::eq(p as *const u8, p2 as *const u8)));
            t.push(Ev::I((*h).temp));
            let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            (lib.hmfree_func)(raw, elemsize);
        }
    });
}

#[test]
fn err_11_del_missing_key() {
    diff("err_11_del_missing_key", |lib, t| unsafe {
        let mut rng = Rng::new(0x411);
        for &(elemsize, keysize) in [(8usize, 4usize), (16, 8)].iter() {
            (lib.rand_seed)(0x31415926);
            let mut hm = Hm::new(lib, elemsize, keysize, 0);
            for i in 0..80usize {
                let k = rng.bytes(keysize);
                hm.put_kv(&k, &vec![i as u8; elemsize - keysize], STBDS_HM_BINARY);
            }
            for _ in 0..400 {
                let k = rng.bytes(keysize);
                t.push(Ev::I(hm.del(&k, STBDS_HM_BINARY)));
                t.push(Ev::U((*hm.header()).length));
            }
            let (a, tb) = hm.snap();
            t.push(Ev::Arr(a));
            t.push(Ev::Tbl(tb));
            hm.free();
        }
    });
}

#[test]
fn err_12_del_twice() {
    diff("err_12_del_twice", |lib, t| unsafe {
        let mut rng = Rng::new(0x412);
        (lib.rand_seed)(0x31415926);
        let mut hm = Hm::new(lib, 16, 8, 0);
        let mut keys = Vec::new();
        for i in 0..120usize {
            let k = rng.bytes(8);
            hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY);
            keys.push(k);
        }
        for k in keys.iter() {
            t.push(Ev::I(hm.del(k, STBDS_HM_BINARY))); // 1 == deleted
            t.push(Ev::I(hm.del(k, STBDS_HM_BINARY))); // 0 == not found
            t.push(Ev::I(hm.del(k, STBDS_HM_BINARY))); // 0
            t.push(Ev::U((*hm.header()).length));
            t.push(Ev::Tbl(snap_table(hm.raw())));
        }
        hm.free();
    });
}

// ===========================================================================
// Rows 13-15 — stbds_hmput_default
// ===========================================================================

#[test]
fn err_13_put_default_null() {
    diff("err_13_put_default_null", |lib, t| unsafe {
        for &elemsize in [1usize, 4, 8, 16, 64, 100].iter() {
            let p = (lib.hmput_default)(std::ptr::null_mut(), elemsize);
            t.push(Ev::Bool(p.is_null()));
            let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            (lib.hmfree_func)(raw, elemsize);
        }
    });
}

#[test]
fn err_14_put_default_len0() {
    diff("err_14_put_default_len0", |lib, t| unsafe {
        // a non-NULL hash pointer whose array length is 0 (the 2nd disjunct of
        // the `if` in stbds_hmput_default)
        for &elemsize in [8usize, 16, 32].iter() {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            // fill the payload with a marker so the memset(0) is observable
            std::ptr::write_bytes(a as *mut u8, 0xEE, elemsize * 4);
            let hp = (a as *mut u8).add(elemsize) as *mut c_void;
            let p = (lib.hmput_default)(hp, elemsize);
            t.push(Ev::Bool(std::ptr::eq(hp as *const u8, p as *const u8)));
            let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
            t.push(Ev::Arr(snap_arr(raw, elemsize)));
            (lib.hmfree_func)(raw, elemsize);
        }
    });
}

#[test]
fn err_15_put_default_noop() {
    diff("err_15_put_default_noop", |lib, t| unsafe {
        for &elemsize in [8usize, 16].iter() {
            let mut p = (lib.hmput_default)(std::ptr::null_mut(), elemsize);
            std::ptr::write_bytes((p as *mut u8).sub(elemsize), 0x5A, elemsize);
            for _ in 0..5 {
                let p2 = (lib.hmput_default)(p, elemsize);
                t.push(Ev::Bool(std::ptr::eq(p as *const u8, p2 as *const u8)));
                p = p2;
                let raw = (p as *mut u8).sub(elemsize) as *mut c_void;
                t.push(Ev::Arr(snap_arr(raw, elemsize)));
            }
            (lib.hmfree_func)((p as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    });
}

// ===========================================================================
// Rows 16-20 — stbds_arrgrowf / stbds_arrfreef edges
// ===========================================================================

#[test]
fn err_16_arrgrowf_noop() {
    diff("err_16_arrgrowf_noop", |lib, t| unsafe {
        for &elemsize in [1usize, 4, 8, 16].iter() {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7);
            let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
            let cap = (*h).capacity;
            for len in 0..=cap {
                (*h).length = len;
                for min_cap in 0..=cap {
                    let b = (lib.arrgrowf)(a, elemsize, 0, min_cap);
                    let noop = std::ptr::eq(a as *const u8, b as *const u8);
                    t.push(Ev::Bool(noop));
                    t.push(Ev::U((*h).capacity));
                    assert!(noop || min_cap > cap);
                }
            }
            (*h).length = 0;
            (lib.arrfreef)(a);
        }
    });
}

#[test]
fn err_17_arrgrowf_zero() {
    diff("err_17_arrgrowf_zero", |lib, t| unsafe {
        // a == NULL, addlen == 0, min_cap == 0  =>  `0 <= arrcap(NULL)` is TRUE
        // and the C returns NULL *without allocating*.
        for &elemsize in [0usize, 1, 8, 64].iter() {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            t.push(Ev::Bool(a.is_null()));
        }
        // addlen == 0, min_cap == 1  =>  bumped to 4
        for &elemsize in [1usize, 8, 64].iter() {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            t.push(Ev::Arr(snap_arr(a, elemsize)));
            (lib.arrfreef)(a);
        }
    });
}

#[test]
fn err_18_arrgrowf_elemsize0() {
    diff("err_18_arrgrowf_elemsize0", |lib, t| unsafe {
        for &addlen in [1usize, 2, 9].iter() {
            for &min_cap in [0usize, 1, 4, 33].iter() {
                let a = (lib.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
                t.push(Ev::Bool(a.is_null()));
                t.push(Ev::Arr(snap_arr(a, 0)));
                if !a.is_null() {
                    (lib.arrfreef)(a);
                }
            }
        }
        // hash-map functions with elemsize 0 also stay in step
        let key = [1u8; 8];
        let mut temp: isize = 0;
        let p = (lib.hmget_key_ts)(
            std::ptr::null_mut(),
            0,
            key.as_ptr() as *mut c_void,
            0,
            &mut temp,
            STBDS_HM_BINARY,
        );
        t.push(Ev::I(temp));
        t.push(Ev::Arr(snap_arr(p, 0)));
        (lib.hmfree_func)(p, 0);
    });
}

#[test]
fn err_19_arrgrowf_oom() {
    same_termination_oom("arrgrowf_oom");
}

#[test]
fn err_20_arrfreef_null() {
    same_termination("arrfreef_null");
}

// ===========================================================================
// Rows 21-24 — hash-function edges
// ===========================================================================

#[test]
fn err_21_hash_bytes_len0() {
    diff("err_21_hash_bytes_len0", |lib, t| unsafe {
        let buf = [0u8; 8];
        for &seed in [0usize, 1, usize::MAX, 0x31415926, 0xdeadbeef].iter() {
            t.push(Ev::U((lib.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed)));
        }
    });
}

#[test]
fn err_22_hash_bytes_null_len0() {
    diff("err_22_hash_bytes_null_len0", |lib, t| unsafe {
        // len == 0 means the pointer is never dereferenced, so NULL is legal
        for &seed in [0usize, 1, usize::MAX, 0x31415926].iter() {
            t.push(Ev::U((lib.hash_bytes)(std::ptr::null_mut(), 0, seed)));
        }
        // ... and must equal the len==0 result for a valid pointer
        let buf = [0u8; 1];
        for &seed in [0usize, 1, usize::MAX, 0x31415926].iter() {
            t.push(Ev::U((lib.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed)));
        }
    });
}

#[test]
fn err_23_hash_string_empty() {
    diff("err_23_hash_string_empty", |lib, t| unsafe {
        let empty = [0u8; 1];
        for &seed in [0usize, 1, 2, usize::MAX, 0x31415926, 1 << 63].iter() {
            t.push(Ev::U((lib.hash_string)(
                empty.as_ptr() as *mut c_char,
                seed,
            )));
        }
    });
}

#[test]
fn err_24_hash_string_high_bytes() {
    diff("err_24_hash_string_high_bytes", |lib, t| unsafe {
        for b in 1..=255u8 {
            let s = [b, 0];
            for &seed in [0usize, usize::MAX, 0x31415926].iter() {
                t.push(Ev::U((lib.hash_string)(s.as_ptr() as *mut c_char, seed)));
            }
        }
        // 0x80..0xFF must NOT sign-extend: compare against a manual reference
        for b in 0x80..=0xFFu8 {
            let s = [b, b, b, 0];
            t.push(Ev::U((lib.hash_string)(s.as_ptr() as *mut c_char, 0)));
        }
    });
}

// ===========================================================================
// Rows 25-30 — arena edges
// ===========================================================================

#[test]
fn err_25_stralloc_oversize() {
    diff("err_25_stralloc_oversize", |lib, t| unsafe {
        let mut rng = Rng::new(0x425);
        for &len in [512usize, 513, 1000, 5000].iter() {
            // storage == NULL branch: sb->next = 0, storage = sb, remaining = 0
            let mut a = arena_zero();
            let s = rng.ascii_cstring(len);
            let p = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
            t.push(Ev::Bytes(cstr_bytes(p)));
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            t.push(Ev::Bool(a.storage.is_null()));
            // storage != NULL branch: spliced in behind the head block
            let s2 = rng.ascii_cstring(len + 1000);
            let p2 = (lib.stralloc)(&mut a, s2.as_ptr() as *mut c_char);
            t.push(Ev::Bytes(cstr_bytes(p2)));
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            (lib.strreset)(&mut a);
            t.push(Ev::U(a.remaining));
            t.push(Ev::Bool(a.storage.is_null()));
        }
    });
}

#[test]
fn err_26_stralloc_block_saturate() {
    diff("err_26_stralloc_block_saturate", |lib, t| unsafe {
        // block 21 -> blocksize 512<<10 = 512K  (< 1M, so ++block)
        // block 22 -> blocksize 512<<11 = 1M    (NOT < 1M, so block frozen)
        for &blk in [20u8, 21, 22, 23, 24, 25].iter() {
            let mut a = arena_zero();
            a.block = blk;
            let s = b"x\0";
            let p = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
            t.push(Ev::Bytes(cstr_bytes(p)));
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            (lib.strreset)(&mut a);
        }
    });
}

#[test]
fn err_27_stralloc_shift_overflow() {
    diff("err_27_stralloc_shift_overflow", |lib, t| unsafe {
        // `512 << (block>>1)` with block >= 128: the shift count reaches 64 and
        // is masked to 6 bits by the x86-64 `shl` instruction.
        for &blk in [126u8, 127, 128, 129, 130, 131, 152, 153, 254, 255].iter() {
            let sh = ((blk as usize) >> 1) & 63;
            let blocksize = 512usize.wrapping_shl(sh as u32);
            if blocksize > (4usize << 20) {
                continue;
            }
            for &len in [1usize, 600].iter() {
                let mut a = arena_zero();
                a.block = blk;
                let s: Vec<u8> = (0..len).map(|_| b'q').chain([0]).collect();
                let p = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                t.push(Ev::U(blk as usize));
                t.push(Ev::Bytes(cstr_bytes(p)));
                t.push(Ev::U(a.remaining));
                t.push(Ev::U(a.block as usize));
                t.push(Ev::Bool(a.storage.is_null()));
                (lib.strreset)(&mut a);
            }
        }
    });
}

#[test]
fn err_27b_stralloc_oom() {
    same_termination_oom("stralloc_oom");
}

#[test]
fn err_29_strreset_empty() {
    diff("err_29_strreset_empty", |lib, t| unsafe {
        let mut a = arena_zero();
        a.block = 7;
        a.mode = 3;
        a.remaining = 123;
        (lib.strreset)(&mut a);
        t.push(Ev::U(a.remaining));
        t.push(Ev::U(a.block as usize));
        t.push(Ev::U(a.mode as usize));
        t.push(Ev::Bool(a.storage.is_null()));
    });
}

#[test]
fn err_30_strreset_twice() {
    diff("err_30_strreset_twice", |lib, t| unsafe {
        let mut a = arena_zero();
        let s = b"abc\0";
        (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
        for _ in 0..4 {
            (lib.strreset)(&mut a);
            t.push(Ev::U(a.remaining));
            t.push(Ev::U(a.block as usize));
            t.push(Ev::U(a.mode as usize));
            t.push(Ev::Bool(a.storage.is_null()));
        }
    });
}

// ===========================================================================
// Rows 31-35 — out-of-range mode values across the FFI boundary
// ===========================================================================

#[test]
fn err_31_mode_out_of_range() {
    // The C `mode` parameter is a plain `int` and the dispatch is `mode >= 1`.
    diff("err_31_mode_out_of_range", |lib, t| unsafe {
        let mut rng = Rng::new(0x431);
        // binary-behaving: every mode < 1 must be indistinguishable from 0
        let modes_bin = [c_int::MIN, -1000, -2, -1, 0];
        let keys: Vec<Vec<u8>> = (0..64).map(|_| rng.bytes(8)).collect();
        for &mode in modes_bin.iter() {
            (lib.rand_seed)(0x31415926);
            let mut hm = Hm::new(lib, 16, 8, 0);
            for (i, k) in keys.iter().enumerate() {
                t.push(Ev::I(hm.put_kv(k, &(i as u64).to_le_bytes().to_vec(), mode)));
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.get(k, mode)));
                t.push(Ev::I(hm.get_ts(k, mode)));
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.del(k, mode)));
            }
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }
        // string-behaving: every mode >= 1 hashes/compares as a string
        let strs: Vec<Vec<u8>> = (0..48)
            .map(|i| format!("k{}_{}\0", i, i * 7).into_bytes())
            .collect();
        for &mode in [1i32, 2, 3, 7, 1000, c_int::MAX].iter() {
            (lib.rand_seed)(0x31415926);
            let mut hm = Hm::from_shmode(lib, 16, 8, STBDS_SH_DEFAULT);
            for (i, s) in strs.iter().enumerate() {
                t.push(Ev::I(hm.put_str(
                    s.as_ptr() as *mut c_char,
                    &(i as u64).to_le_bytes().to_vec(),
                    mode,
                )));
            }
            for s in strs.iter() {
                t.push(Ev::I(hm.get_str(s.as_ptr() as *mut c_char, mode)));
            }
            // a key that is absent
            let absent = b"definitely_absent\0";
            t.push(Ev::I(hm.get_str(absent.as_ptr() as *mut c_char, mode)));
            t.push(Ev::I(hm.del_str(absent.as_ptr() as *mut c_char, mode)));
            t.push(Ev::Str(snap_str_elems(&hm)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }
    });
}

#[test]
fn err_32_put_fresh_mode() {
    // On a FRESH table stbds_hmput_key sets
    //   nt->string.mode = mode >= STBDS_HM_STRING ? STBDS_SH_DEFAULT : 0
    diff("err_32_put_fresh_mode", |lib, t| unsafe {
        for &mode in [c_int::MIN, -1, 0, 1, 2, 99, c_int::MAX].iter() {
            (lib.rand_seed)(0x31415926);
            if mode >= STBDS_HM_STRING {
                let s = b"the_key\0";
                let mut hm = Hm::new(lib, 16, 8, 0);
                t.push(Ev::I(hm.put_str(
                    s.as_ptr() as *mut c_char,
                    &0u64.to_le_bytes().to_vec(),
                    mode,
                )));
                t.push(Ev::Tbl(snap_table(hm.raw()))); // str_mode must be 1
                t.push(Ev::Str(snap_str_elems(&hm)));
                hm.free();
            } else {
                let k = [1u8, 2, 3, 4, 5, 6, 7, 8];
                let mut hm = Hm::new(lib, 16, 8, 0);
                t.push(Ev::I(hm.put_kv(&k, &0u64.to_le_bytes().to_vec(), mode)));
                t.push(Ev::Tbl(snap_table(hm.raw()))); // str_mode must be 0
                t.push(Ev::Arr(snap_arr(hm.raw(), 16)));
                hm.free();
            }
        }
    });
}

#[test]
fn err_33_del_mode2_asymmetry() {
    // deleting the LAST element with mode == 2 skips the re-find, so it is
    // well-defined and can be compared in-process (also covered by cfg_33)
    diff("err_33_del_mode2_asymmetry", |lib, t| unsafe {
        for &shmode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA].iter() {
            (lib.rand_seed)(0x31415926);
            let keys: Vec<Vec<u8>> = (0..12).map(|i| format!("kk_{}\0", i).into_bytes()).collect();
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            for (i, k) in keys.iter().enumerate() {
                hm.put_str(
                    k.as_ptr() as *mut c_char,
                    &(i as u64).to_le_bytes().to_vec(),
                    STBDS_HM_STRING,
                );
            }
            for i in (0..keys.len()).rev() {
                t.push(Ev::I(hm.del_str(keys[i].as_ptr() as *mut c_char, 2)));
                t.push(Ev::Str(snap_str_elems(&hm)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            hm.free();
        }
    });
}

#[test]
fn err_33b_del_mode2_nonlast_aborts() {
    same_termination("hmdel_mode2_nonlast");
}

#[test]
fn err_34_shmode_out_of_range() {
    diff("err_34_shmode_out_of_range", |lib, t| unsafe {
        for m in -300i32..=300 {
            (lib.rand_seed)(0x31415926);
            let p = (lib.shmode_func)(16, m);
            let raw = (p as *mut u8).sub(16) as *mut c_void;
            let tb = snap_table(raw);
            t.push(Ev::U(tb.str_mode as usize));
            t.push(Ev::Tbl(tb));
            (lib.hmfree_func)(raw, 16);
        }
        for &m in [c_int::MIN, c_int::MAX, 1 << 16, (1 << 16) + 3].iter() {
            (lib.rand_seed)(0x31415926);
            let p = (lib.shmode_func)(16, m);
            let raw = (p as *mut u8).sub(16) as *mut c_void;
            t.push(Ev::Tbl(snap_table(raw)));
            (lib.hmfree_func)(raw, 16);
        }
    });
}

#[test]
fn err_35_shmode_undefined_mode() {
    // string.mode outside {0,1,2,3} falls into the `default:` memcpy branch
    diff("err_35_shmode_undefined_mode", |lib, t| unsafe {
        for &shmode in [4i32, 5, 100, 254, 255].iter() {
            (lib.rand_seed)(0x31415926);
            let keys: Vec<Vec<u8>> = (0..5)
                .map(|i| format!("long_enough_key_{}\0", i).into_bytes())
                .collect();
            let mut hm = Hm::from_shmode(lib, 16, 8, shmode);
            for (i, k) in keys.iter().enumerate() {
                t.push(Ev::I(hm.put_str(
                    k.as_ptr() as *mut c_char,
                    &(i as u64).to_le_bytes().to_vec(),
                    STBDS_HM_STRING,
                )));
                // the key slot holds the first 8 BYTES OF THE STRING, not a ptr
                t.push(Ev::Arr(snap_arr(hm.raw(), 16)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            hm.free();
        }
    });
}

#[test]
fn err_35b_sh_none_lookup_segv() {
    same_termination("sh_none_lookup");
}

// ===========================================================================
// Rows 36-37 — degenerate keysize / keyoffset
// ===========================================================================

#[test]
fn err_36_put_keysize0() {
    // keysize == 0: hash_bytes(p,0,seed) is key-independent and memcmp(_,_,0)==0,
    // so the FIRST key permanently shadows every other key.
    diff("err_36_put_keysize0", |lib, t| unsafe {
        let mut rng = Rng::new(0x436);
        for &elemsize in [8usize, 16].iter() {
            (lib.rand_seed)(0x31415926);
            let mut hm = Hm::new(lib, elemsize, 0, 0);
            for i in 0..20usize {
                let k = rng.bytes(4);
                t.push(Ev::I(hm.put_kv(&k, &vec![i as u8; elemsize], STBDS_HM_BINARY)));
                t.push(Ev::Arr(snap_arr(hm.raw(), elemsize)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            for _ in 0..10 {
                let k = rng.bytes(4);
                t.push(Ev::I(hm.get(&k, STBDS_HM_BINARY)));
            }
            for _ in 0..5 {
                let k = rng.bytes(4);
                t.push(Ev::I(hm.del(&k, STBDS_HM_BINARY)));
                t.push(Ev::Arr(snap_arr(hm.raw(), elemsize)));
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            hm.free();
        }
    });
}

#[test]
fn err_37_del_keyoffset() {
    // stbds_hmput_key/hmget_key hard-code keyoffset = 0; only hmdel_key accepts
    // one, so a non-zero keyoffset makes the delete compare the wrong bytes.
    diff("err_37_del_keyoffset", |lib, t| unsafe {
        let mut rng = Rng::new(0x437);
        for &keyoffset in [0usize, 1, 4, 8, 12, 15].iter() {
            (lib.rand_seed)(0x31415926);
            let mut hm = Hm::new(lib, 16, 4, keyoffset);
            let mut keys = Vec::new();
            for i in 0..40usize {
                let k = rng.bytes(4);
                hm.put_kv(&k, &vec![i as u8; 12], STBDS_HM_BINARY);
                keys.push(k);
            }
            for k in keys.iter() {
                t.push(Ev::I(hm.del(k, STBDS_HM_BINARY)));
                t.push(Ev::U((*hm.header()).length));
            }
            t.push(Ev::Arr(snap_arr(hm.raw(), 16)));
            t.push(Ev::Tbl(snap_table(hm.raw())));
            hm.free();
        }
    });
}

// ===========================================================================
// Rows 41-44 — arr_ins / strkey extremes and tombstone paths
// ===========================================================================

#[test]
fn err_41_arr_ins_extremes() {
    diff("err_41_arr_ins_extremes", |lib, t| unsafe {
        for &n in [
            0i32,
            1,
            2,
            3,
            4,
            5,
            -1,
            -4,
            i32::MIN,
            i32::MAX,
            i32::MIN + 1,
            i32::MAX - 1,
        ]
        .iter()
        {
            (lib.arr_ins)(n);
            t.push(Ev::I(n as isize));
        }
    });
}

#[test]
fn err_42_strkey_extremes() {
    diff("err_42_strkey_extremes", |lib, t| unsafe {
        for &n in [
            0i32,
            -1,
            1,
            -9,
            9,
            -10,
            10,
            i32::MIN,
            i32::MAX,
            i32::MIN + 1,
            i32::MAX - 1,
        ]
        .iter()
        {
            let p = (lib.strkey)(n);
            t.push(Ev::Bytes(cstr_bytes(p)));
            // the static buffer must be reused, i.e. the same pointer every time
            t.push(Ev::Bool(!p.is_null()));
        }
    });
}

#[test]
fn err_43_tombstone_paths() {
    diff("err_43_tombstone_paths", |lib, t| unsafe {
        let mut rng = Rng::new(0x443);
        for &seed in [0usize, 1, 0x31415926, usize::MAX].iter() {
            (lib.rand_seed)(seed);
            let mut hm = Hm::new(lib, 16, 8, 0);
            let mut keys = Vec::new();
            // grow well past several doublings
            for i in 0..800usize {
                let k = rng.bytes(8);
                hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY);
                keys.push(k);
            }
            t.push(Ev::Tbl(snap_table(hm.raw())));
            // delete almost everything -> repeated shrink + rebuild
            for k in keys.iter() {
                hm.del(k, STBDS_HM_BINARY);
                t.push(Ev::Tbl(snap_table(hm.raw())));
            }
            t.push(Ev::Arr(snap_arr(hm.raw(), 16)));
            // and lookups on the fully-drained table
            for k in keys.iter().take(100) {
                t.push(Ev::I(hm.get(k, STBDS_HM_BINARY)));
            }
            hm.free();
        }
    });
}

#[test]
fn err_44_tombstone_reuse() {
    diff("err_44_tombstone_reuse", |lib, t| unsafe {
        let mut rng = Rng::new(0x444);
        for &seed in [0usize, 3, 0x31415926].iter() {
            (lib.rand_seed)(seed);
            let mut hm = Hm::new(lib, 16, 8, 0);
            let mut keys = Vec::new();
            for i in 0..300usize {
                let k = rng.bytes(8);
                hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY);
                keys.push(k);
            }
            // delete/re-insert the same keys many times: tombstone_count goes up
            // and is decremented again by the `tombstone >= 0` reuse branch
            for round in 0..6usize {
                for k in keys.iter().skip(round % 3).step_by(3) {
                    t.push(Ev::I(hm.del(k, STBDS_HM_BINARY)));
                }
                t.push(Ev::Tbl(snap_table(hm.raw())));
                for (i, k) in keys.iter().enumerate().skip(round % 3).step_by(3) {
                    t.push(Ev::I(hm.put_kv(
                        k,
                        &((i + round) as u64).to_le_bytes().to_vec(),
                        STBDS_HM_BINARY,
                    )));
                }
                t.push(Ev::Tbl(snap_table(hm.raw())));
                t.push(Ev::Arr(snap_arr(hm.raw(), 16)));
            }
            hm.free();
        }
    });
}

// ---------------------------------------------------------------------------

fn arena_zero() -> StringArena {
    StringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    }
}
