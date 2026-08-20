//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Non-fatal rejections are compared in-process (same sentinel / same state).
//! Fatal rejections (`assert` ⇒ `SIGABRT`, null deref ⇒ `SIGSEGV`) are compared
//! by re-executing this very test binary as a child process against exactly one
//! of the two libraries and comparing the resulting wait status, so "the same
//! error" means the same signal number, not merely "both failed somehow".

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// ===========================================================================
// Fatal-path plumbing
// ===========================================================================

const CRASH_ENV_SCENARIO: &str = "DIFF_CRASH_SCENARIO";
const CRASH_ENV_LIB: &str = "DIFF_CRASH_LIB";

/// The child entry point.  Does nothing during a normal test run.
#[test]
fn crash_child() {
    let scenario = match std::env::var(CRASH_ENV_SCENARIO) {
        Ok(s) => s,
        Err(_) => return,
    };
    let which = std::env::var(CRASH_ENV_LIB).unwrap_or_default();
    let (c, r) = load_both();
    let api = if which == "C" { c } else { r };
    eprintln!("child: lib={} scenario={}", api.name, scenario);
    unsafe { run_fatal_scenario(&api, &scenario) };
    // Reaching this point means the scenario did NOT crash.
    eprintln!("child: scenario {scenario} returned normally");
    std::process::exit(77);
}

unsafe fn run_fatal_scenario(api: &Api, scenario: &str) {
    let mut key = [1u8, 2, 3, 4, 5, 6, 7, 8];
    match scenario {
        // ERRORS.md row 5
        "arrfreef_null" => (api.arrfreef)(std::ptr::null_mut()),
        // ERRORS.md row 8
        "hash_bytes_null" => {
            let v = (api.hash_bytes)(std::ptr::null_mut(), 16, 1);
            eprintln!("unexpected {v}");
        }
        // ERRORS.md row 9
        "hash_string_null" => {
            let v = (api.hash_string)(std::ptr::null_mut(), 1);
            eprintln!("unexpected {v}");
        }
        // ERRORS.md row 19
        "hmget_key_ts_null_temp" => {
            let p = (api.hmget_key_ts)(
                std::ptr::null_mut(),
                8,
                key.as_mut_ptr() as *mut c_void,
                4,
                std::ptr::null_mut(),
                STBDS_HM_BINARY,
            );
            eprintln!("unexpected {p:?}");
        }
        // ERRORS.md row 52
        "stralloc_null_arena" => {
            let mut s = *b"abc\0";
            let p = (api.stralloc)(std::ptr::null_mut(), s.as_mut_ptr() as *mut c_char);
            eprintln!("unexpected {p:?}");
        }
        "stralloc_null_str" => {
            let mut a = StringArena::zeroed();
            let p = (api.stralloc)(&mut a, std::ptr::null_mut());
            eprintln!("unexpected {p:?}");
        }
        // ERRORS.md row 54
        "strreset_null" => (api.strreset)(std::ptr::null_mut()),
        // ERRORS.md rows 1/3 taken to their oversized extreme: the allocation
        // fails, `realloc` returns NULL and the header write lands on address 0.
        "arrgrowf_oom" => {
            let p = (api.arrgrowf)(std::ptr::null_mut(), 8, 0, usize::MAX / 16);
            eprintln!("unexpected {p:?}");
        }
        // ERRORS.md rows 40/41: `mode > STBDS_HM_STRING` hashes as a string but
        // the back-fill re-find passes the *address* of the key field, so
        // `STBDS_ASSERT(slot >= 0)` fires.
        "hmdel_backfill_mode_gt_1" => {
            let es = 8usize;
            let cfg = MapCfg::string(es, 2);
            let mut keys: Vec<Vec<u8>> = (0..6)
                .map(|i| {
                    let mut v = format!("backfill_key_{i}").into_bytes();
                    v.push(0);
                    v
                })
                .collect();
            let mut t: *mut c_void = std::ptr::null_mut();
            for k in keys.iter_mut() {
                t = map_put_string(api, t, &cfg, k.as_mut_ptr() as *mut c_char, &[]);
            }
            // delete the FIRST inserted key -> old_index != final_index -> back-fill
            let p = (api.hmdel_key)(
                t,
                es,
                keys[0].as_mut_ptr() as *mut c_void,
                8,
                0,
                2,
            );
            eprintln!("unexpected {p:?}");
        }
        other => panic!("unknown scenario {other}"),
    }
}

fn run_child(scenario: &str, lib: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let st = Command::new(exe)
        .args(["--exact", "crash_child", "--nocapture", "--test-threads=1"])
        .env(CRASH_ENV_SCENARIO, scenario)
        .env(CRASH_ENV_LIB, lib)
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");
    (st.code(), st.signal())
}

#[track_caller]
fn assert_same_fatal(scenario: &str) {
    let c = run_child(scenario, "C");
    let r = run_child(scenario, "RUST");
    assert_eq!(
        c, r,
        "scenario `{scenario}`: C exited with (code,signal)={c:?} but RUST with {r:?}"
    );
    assert!(
        c.1.is_some(),
        "scenario `{scenario}`: expected the C library to die from a signal, got code={:?}",
        c.0
    );
    println!("fatal scenario `{scenario}`: both died with signal {:?}", c.1);
}

// ===========================================================================
// ERRORS.md rows 1..4 — stbds_arrgrowf
// ===========================================================================

#[test]
fn e01_04_arrgrowf_boundaries() {
    let (c, r) = load_both();
    unsafe {
        for &es in &[1usize, 4, 8, 16, 64] {
            // row 1: no-growth on a NULL array -> NULL, no allocation
            let cv = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let rv = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            diff_eq_val(&format!("e01 es={es}"), cv.is_null(), rv.is_null());
            assert!(cv.is_null());

            // row 3: fresh allocation; row 4: the min_cap<4 clamp
            let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            diff_eq(
                &format!("e03 es={es}"),
                &snapshot_array(ca, es, 0),
                &snapshot_array(ra, es, 0),
            );
            assert_eq!(arr_cap(ca), 4, "min_cap<4 must clamp to 4");
            assert_eq!((*header(ca)).length, 0);
            assert!((*header(ca)).hash_table.is_null());
            assert_eq!((*header(ca)).temp, 0);

            // row 2: min_cap <= cap -> identical pointer
            for &mc in &[0usize, 1, 2, 3, 4] {
                let cb = (c.arrgrowf)(ca, es, 0, mc);
                let rb = (r.arrgrowf)(ra, es, 0, mc);
                diff_eq_val(&format!("e02 es={es} mc={mc}"), cb == ca, rb == ra);
                assert_eq!(cb, ca);
            }
            // row 4 boundaries around 2*cap
            let mut ca = ca;
            let mut ra = ra;
            for &mc in &[5usize, 7, 8, 9, 16, 17] {
                ca = (c.arrgrowf)(ca, es, 0, mc);
                ra = (r.arrgrowf)(ra, es, 0, mc);
                diff_eq(
                    &format!("e04 es={es} mc={mc}"),
                    &snapshot_array(ca, es, 0),
                    &snapshot_array(ra, es, 0),
                );
            }
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

// ERRORS.md row 5
#[test]
fn e05_arrfreef_null_is_fatal_in_both() {
    assert_same_fatal("arrfreef_null");
}

// ERRORS.md rows 1/3 with an oversized min_cap (allocation failure)
#[test]
fn e01b_arrgrowf_oversized_is_fatal_in_both() {
    assert_same_fatal("arrgrowf_oom");
}

// ===========================================================================
// ERRORS.md rows 7..10 — hash functions
// ===========================================================================

#[test]
fn e07_hash_bytes_len_zero_accepts_null() {
    let (c, r) = load_both();
    unsafe {
        for &seed in &[0usize, 1, 0x31415926, usize::MAX] {
            let cv = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let rv = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            diff_eq_val(&format!("e07 seed={seed:#x}"), cv, rv);
        }
    }
}

#[test]
fn e08_hash_bytes_null_with_len_is_fatal_in_both() {
    assert_same_fatal("hash_bytes_null");
}

#[test]
fn e09_hash_string_null_is_fatal_in_both() {
    assert_same_fatal("hash_string_null");
}

#[test]
fn e10_hash_string_empty() {
    let (c, r) = load_both();
    unsafe {
        let mut empty = [0u8; 1];
        for &seed in &[0usize, 1, 2, 0x31415926, usize::MAX, usize::MAX - 1] {
            let cv = (c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            let rv = (r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            diff_eq_val(&format!("e10 seed={seed:#x}"), cv, rv);
        }
    }
}

// ===========================================================================
// ERRORS.md row 11 — out-of-range `mode` values across the FFI boundary
// ===========================================================================

#[test]
fn e11_out_of_range_mode_classification() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 16usize;
    unsafe {
        // Every `mode <= 0` must behave like STBDS_HM_BINARY and every
        // `mode >= 1` like a string mode.  Both classes are compared C-vs-Rust,
        // and the binary class is additionally compared against `mode == 0`.
        let mut reference: Vec<String> = Vec::new();
        for (round, &mode) in [0 as c_int, -1, -7, -1000, c_int::MIN].iter().enumerate() {
            let cfg = MapCfg {
                elemsize: es,
                keysize: 8,
                mode,
                del_keyoffset: 0,
                kind: KeyKind::Raw,
            };
            pin_seed(&c, &r, 0x1111);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            let mut trace = Vec::new();
            for k in 0..20u64 {
                ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[0x11u8; 16]);
                rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[0x11u8; 16]);
                diff_eq(
                    &format!("e11 mode={mode} put({k})"),
                    &snapshot_map(ct, es, KeyKind::Raw),
                    &snapshot_map(rt, es, KeyKind::Raw),
                );
                trace.push(snapshot_map(ct, es, KeyKind::Raw));
            }
            for k in 0..20u64 {
                let mut key = k.to_le_bytes();
                let (nct, ci) = map_geti(&c, ct, &cfg, &mut key);
                let mut key = k.to_le_bytes();
                let (nrt, ri) = map_geti(&r, rt, &cfg, &mut key);
                ct = nct;
                rt = nrt;
                diff_eq_val(&format!("e11 mode={mode} get({k})"), ci, ri);
                trace.push(format!("get({k})={ci}"));
            }
            for k in 0..20u64 {
                let mut key = k.to_le_bytes();
                let (nct, cd) = map_del(&c, ct, &cfg, &mut key);
                let mut key = k.to_le_bytes();
                let (nrt, rd) = map_del(&r, rt, &cfg, &mut key);
                ct = nct;
                rt = nrt;
                diff_eq_val(&format!("e11 mode={mode} del({k})"), cd, rd);
                trace.push(snapshot_map(ct, es, KeyKind::Raw));
            }
            map_free(&c, ct, es);
            map_free(&r, rt, es);
            if round == 0 {
                reference = trace;
            } else {
                assert_eq!(trace.len(), reference.len());
                for (i, (a, b)) in reference.iter().zip(trace.iter()).enumerate() {
                    diff_eq(&format!("e11 mode={mode} step {i} vs mode=0"), a, b);
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 12..13 — stbds_hmfree_func
// ===========================================================================

#[test]
fn e12_hmfree_func_null_is_noop() {
    let (c, r) = load_both();
    unsafe {
        for &es in &[1usize, 8, 16] {
            // must simply return; nothing to compare but "does not crash"
            (c.hmfree_func)(std::ptr::null_mut(), es);
            (r.hmfree_func)(std::ptr::null_mut(), es);
        }
    }
    println!("e12: hmfree_func(NULL) is a no-op in both libraries");
}

#[test]
fn e13_hmfree_func_without_hash_table() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &es in &[1usize, 8, 16, 24] {
            let ct = (c.hmput_default)(std::ptr::null_mut(), es);
            let rt = (r.hmput_default)(std::ptr::null_mut(), es);
            let cs = snapshot_map(ct, es, KeyKind::Raw);
            let rs = snapshot_map(rt, es, KeyKind::Raw);
            diff_eq(&format!("e13 es={es}"), &cs, &rs);
            assert!(cs.contains("table=NULL"), "expected a table-less map");
            (c.hmfree_func)(hash_to_arr(ct, es), es);
            (r.hmfree_func)(hash_to_arr(rt, es), es);
        }
    }
}

/// ERRORS.md row 13 (second half): when the map *does* have a hash table,
/// `stbds_hmfree_func` must also run `stbds_strreset` on the table's arena, i.e.
/// release the whole `stbds_string_block` chain.  Skipping it is invisible in the
/// data structures (they are freed right after), so it is detected with the same
/// glibc tcache LIFO probe used by `e38`: after the free, a `malloc` of the
/// block's size class must hand the block back.
#[test]
fn e13b_hmfree_func_releases_the_arena_chain() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 8usize;
    unsafe {
        let mut released = Vec::new();
        for api in [&c, &r] {
            let cfg = MapCfg::string(es, STBDS_HM_STRING);
            pin_seed(&c, &r, 0xcccc);
            // Short keys and only a handful of them -> exactly ONE arena block of
            // 512 payload bytes (malloc'd as 8 + 512), so the probe is unambiguous.
            let mut keys: Vec<Vec<u8>> = (0..4)
                .map(|i| {
                    let mut v = format!("k{i}").into_bytes();
                    v.push(0);
                    v
                })
                .collect();
            let mut t: *mut c_void = (api.shmode_func)(es, STBDS_SH_ARENA);
            for k in keys.iter_mut() {
                t = map_put_string(api, t, &cfg, k.as_mut_ptr() as *mut c_char, &[]);
            }
            let ti = map_table(t, es).unwrap();
            let head = ti.string.storage as *mut c_void;
            assert!(!head.is_null(), "expected one arena block");
            assert_eq!(ti.string.block, 1, "expected exactly one 512-byte block");
            // the chain must be a single block
            assert!((*(head as *mut *mut c_void)).is_null(), "expected a 1-block chain");

            (api.hmfree_func)(hash_to_arr(t, es), es);

            let probe = malloc(8 + 512);
            let got = probe == head;
            free(probe);
            released.push(got);
        }
        println!(
            "e13b: C released the arena block={} RUST released={}",
            released[0], released[1]
        );
        diff_eq_val("e13b arena released", released[0], released[1]);
        assert!(
            released[0],
            "probe broken: the C hmfree_func must release the arena block"
        );
    }
}

// ===========================================================================
// ERRORS.md rows 14..18, 20 — key-absent reporting
// ===========================================================================

#[test]
fn e14_20_key_absent_paths() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 8usize;
    let cfg = MapCfg::binary(es, 4);
    unsafe {
        // row 16 / 20: get on a NULL map -> allocates, temp == -1
        pin_seed(&c, &r, 0x2222);
        let mut key = 1u32.to_le_bytes();
        let (ct, ci) = map_geti(&c, std::ptr::null_mut(), &cfg, &mut key);
        let mut key = 1u32.to_le_bytes();
        let (rt, ri) = map_geti(&r, std::ptr::null_mut(), &cfg, &mut key);
        diff_eq_val("e16 get(NULL) temp", ci, ri);
        assert_eq!(ci, -1, "STBDS_INDEX_EMPTY expected");
        diff_eq(
            "e16 get(NULL) state",
            &snapshot_map(ct, es, KeyKind::Raw),
            &snapshot_map(rt, es, KeyKind::Raw),
        );

        // row 17: array exists but hash_table == 0
        let mut key = 5u32.to_le_bytes();
        let (ct, ci) = map_geti(&c, ct, &cfg, &mut key);
        let mut key = 5u32.to_le_bytes();
        let (rt, ri) = map_geti(&r, rt, &cfg, &mut key);
        diff_eq_val("e17 no-table temp", ci, ri);
        assert_eq!(ci, -1);
        let mut key = 5u32.to_le_bytes();
        let (ct, ci) = map_geti_ts(&c, ct, &cfg, &mut key);
        let mut key = 5u32.to_le_bytes();
        let (rt, ri) = map_geti_ts(&r, rt, &cfg, &mut key);
        diff_eq_val("e17 no-table temp (ts)", ci, ri);
        assert_eq!(ci, -1);
        map_free(&c, ct, es);
        map_free(&r, rt, es);

        // rows 14/15/18: hash table present, key absent -> find_slot returns -1
        // from BOTH inner scans.  Using many random probes on a heavily loaded
        // table guarantees both scans are taken.
        pin_seed(&c, &r, 0x2223);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        for k in 0..200u32 {
            ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[9, 9, 9, 9]);
            rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[9, 9, 9, 9]);
        }
        let mut rng = Rng::new(1420);
        for _ in 0..5000 {
            let k = 1_000_000u32.wrapping_add(rng.next_u64() as u32);
            let mut key = k.to_le_bytes();
            let (nct, ci) = map_geti(&c, ct, &cfg, &mut key);
            let mut key = k.to_le_bytes();
            let (nrt, ri) = map_geti(&r, rt, &cfg, &mut key);
            ct = nct;
            rt = nrt;
            diff_eq_val(&format!("e14/15/18 miss({k})"), ci, ri);
            assert_eq!(ci, -1);
            let mut key = k.to_le_bytes();
            let (nct, ci) = map_geti_ts(&c, ct, &cfg, &mut key);
            let mut key = k.to_le_bytes();
            let (nrt, ri) = map_geti_ts(&r, rt, &cfg, &mut key);
            ct = nct;
            rt = nrt;
            diff_eq_val(&format!("e18 miss_ts({k})"), ci, ri);
            assert_eq!(ci, -1);
        }
        map_free(&c, ct, es);
        map_free(&r, rt, es);
    }
}

#[test]
fn e19_hmget_key_ts_null_temp_is_fatal_in_both() {
    assert_same_fatal("hmget_key_ts_null_temp");
}

// ===========================================================================
// ERRORS.md rows 21..23 — stbds_hmput_default
// ===========================================================================

#[test]
fn e21_23_hmput_default_paths() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &es in &[1usize, 8, 16, 32] {
            // row 21: NULL -> create
            let ct = (c.hmput_default)(std::ptr::null_mut(), es);
            let rt = (r.hmput_default)(std::ptr::null_mut(), es);
            diff_eq(
                &format!("e21 es={es}"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
            assert_eq!((*header(hash_to_arr(ct, es))).length, 1);

            // row 23: non-NULL and length != 0 -> unchanged
            let before = snapshot_map(ct, es, KeyKind::Raw);
            let ct2 = (c.hmput_default)(ct, es);
            let rt2 = (r.hmput_default)(rt, es);
            assert_eq!(ct2, ct, "must be the identical pointer");
            assert_eq!(rt2, rt);
            diff_eq_val(&format!("e23 es={es} unchanged"), before, snapshot_map(ct2, es, KeyKind::Raw));
            diff_eq(
                &format!("e23 es={es}"),
                &snapshot_map(ct2, es, KeyKind::Raw),
                &snapshot_map(rt2, es, KeyKind::Raw),
            );
            (c.hmfree_func)(hash_to_arr(ct2, es), es);
            (r.hmfree_func)(hash_to_arr(rt2, es), es);

            // row 22: array with length == 0
            let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            assert_eq!((*header(ca)).length, 0);
            let ct = (c.hmput_default)(arr_to_hash(ca, es), es);
            let rt = (r.hmput_default)(arr_to_hash(ra, es), es);
            diff_eq(
                &format!("e22 es={es}"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
            assert_eq!((*header(hash_to_arr(ct, es))).length, 1);
            (c.hmfree_func)(hash_to_arr(ct, es), es);
            (r.hmfree_func)(hash_to_arr(rt, es), es);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 24..29, 31, 32 — stbds_hmput_key
// ===========================================================================

#[test]
fn e24_29_hmput_key_paths() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 8usize;
    let cfg = MapCfg::binary(es, 4);
    unsafe {
        pin_seed(&c, &r, 0x3333);
        // row 24: NULL map -> raw array + zeroed default element first
        let ct = map_put_binary(&c, std::ptr::null_mut(), &cfg, &1u32.to_le_bytes(), &[1, 1, 1, 1]);
        let rt = map_put_binary(&r, std::ptr::null_mut(), &cfg, &1u32.to_le_bytes(), &[1, 1, 1, 1]);
        diff_eq("e24", &snapshot_map(ct, es, KeyKind::Raw), &snapshot_map(rt, es, KeyKind::Raw));
        // row 25: fresh table has exactly STBDS_BUCKET_LENGTH slots
        let ti = map_table(ct, es).expect("table");
        assert_eq!(ti.slot_count, STBDS_BUCKET_LENGTH);
        assert_eq!(ti.used_count_threshold, 6);
        assert_eq!(ti.tombstone_count_threshold, 1);
        assert_eq!(ti.used_count_shrink_threshold, 0, "8-slot tables never shrink");
        diff_eq_val("e25 rust table", format!("{:?}", (ti.slot_count, ti.used_count_threshold, ti.tombstone_count_threshold, ti.used_count_shrink_threshold)),
                    { let t2 = map_table(rt, es).unwrap(); format!("{:?}", (t2.slot_count, t2.used_count_threshold, t2.tombstone_count_threshold, t2.used_count_shrink_threshold)) });
        map_free(&c, ct, es);
        map_free(&r, rt, es);

        // row 26: the rehash boundary, checked slot-count by slot-count
        pin_seed(&c, &r, 0x3334);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        let mut expected_slots = 8usize;
        for k in 0..100u32 {
            let used_before = map_table(ct, es).map(|t| t.used_count).unwrap_or(0);
            let thr_before = map_table(ct, es).map(|t| t.used_count_threshold).unwrap_or(0);
            if !ct.is_null() && map_table(ct, es).is_some() && used_before >= thr_before {
                expected_slots *= 2;
            }
            ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[2, 2, 2, 2]);
            rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[2, 2, 2, 2]);
            let ti = map_table(ct, es).unwrap();
            assert_eq!(ti.slot_count, expected_slots, "unexpected slot count after put({k})");
            diff_eq(
                &format!("e26 put({k})"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
        }

        // rows 28/29: `hash < 2` fixup and tombstone reuse are exercised by
        // repeatedly emptying and refilling the table.
        for k in 0..100u32 {
            let mut key = k.to_le_bytes();
            let (nct, cd) = map_del(&c, ct, &cfg, &mut key);
            let mut key = k.to_le_bytes();
            let (nrt, rd) = map_del(&r, rt, &cfg, &mut key);
            ct = nct;
            rt = nrt;
            diff_eq_val(&format!("e29 del({k})"), cd, rd);
        }
        let mut max_reuse = 0usize;
        for round in 0..8u32 {
            for k in 0..40u32 {
                ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[3, 3, 3, 3]);
                rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[3, 3, 3, 3]);
                diff_eq(
                    &format!("e29 round={round} put({k})"),
                    &snapshot_map(ct, es, KeyKind::Raw),
                    &snapshot_map(rt, es, KeyKind::Raw),
                );
                max_reuse = max_reuse.max(map_table(ct, es).unwrap().tombstone_count);
            }
            for k in 0..40u32 {
                let mut key = k.to_le_bytes();
                let (nct, _) = map_del(&c, ct, &cfg, &mut key);
                let mut key = k.to_le_bytes();
                let (nrt, _) = map_del(&r, rt, &cfg, &mut key);
                ct = nct;
                rt = nrt;
                diff_eq(
                    &format!("e29 round={round} del({k})"),
                    &snapshot_map(ct, es, KeyKind::Raw),
                    &snapshot_map(rt, es, KeyKind::Raw),
                );
                max_reuse = max_reuse.max(map_table(ct, es).unwrap().tombstone_count);
            }
        }
        assert!(max_reuse > 0, "tombstones were never created");
        map_free(&c, ct, es);
        map_free(&r, rt, es);
    }
}

// ERRORS.md rows 27, 31, 32 (mode/string.mode classification) are covered
// exhaustively by tests/strmap.rs rows 48/52/53/54; this test re-checks the
// two *classification* boundaries directly.
#[test]
fn e27_32_string_mode_boundaries() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &es in &[8usize, 16] {
            for &(mode, expect) in &[
                (c_int::MIN, 0u8),
                (-1, 0),
                (0, 0),
                (1, 1),
                (2, 1),
                (3, 1),
                (1000, 1),
                (c_int::MAX, 1),
            ] {
                pin_seed(&c, &r, 0x4444);
                let mut kbuf = *b"a_key_for_mode\0\0\0\0\0\0\0\0\0\0";
                let ct = (c.hmput_key)(
                    std::ptr::null_mut(),
                    es,
                    kbuf.as_mut_ptr() as *mut c_void,
                    8,
                    mode,
                );
                let rt = (r.hmput_key)(
                    std::ptr::null_mut(),
                    es,
                    kbuf.as_mut_ptr() as *mut c_void,
                    8,
                    mode,
                );
                let cm = map_table(ct, es).unwrap().string.mode;
                let rm = map_table(rt, es).unwrap().string.mode;
                diff_eq_val(&format!("e27 mode={mode} implicit string.mode"), cm, rm);
                assert_eq!(cm, expect, "mode={mode} must classify to string.mode {expect}");
                (c.hmfree_func)(hash_to_arr(ct, es), es);
                (r.hmfree_func)(hash_to_arr(rt, es), es);
            }
            // row 32: (unsigned char) truncation in shmode_func
            for &(mode, expect) in &[
                (0 as c_int, 0u8),
                (1, 1),
                (2, 2),
                (3, 3),
                (4, 4),
                (255, 255),
                (256, 0),
                (257, 1),
                (300, 44),
                (-1, 255),
                (c_int::MAX, 255),
                (c_int::MIN, 0),
            ] {
                pin_seed(&c, &r, 0x4445);
                let ct = (c.shmode_func)(es, mode);
                let rt = (r.shmode_func)(es, mode);
                let cm = map_table(ct, es).unwrap().string.mode;
                let rm = map_table(rt, es).unwrap().string.mode;
                diff_eq_val(&format!("e32 shmode_func mode={mode}"), cm, rm);
                assert_eq!(cm, expect);
                (c.hmfree_func)(hash_to_arr(ct, es), es);
                (r.hmfree_func)(hash_to_arr(rt, es), es);
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 33..35, 39, 43..45 — stbds_hmdel_key
// ===========================================================================

#[test]
fn e33_35_hmdel_key_rejections() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 8usize;
    let cfg = MapCfg::binary(es, 4);
    unsafe {
        // row 33: a == NULL -> returns NULL
        let mut key = 1u32.to_le_bytes();
        let cv = (c.hmdel_key)(std::ptr::null_mut(), es, key.as_mut_ptr() as *mut c_void, 4, 0, 0);
        let rv = (r.hmdel_key)(std::ptr::null_mut(), es, key.as_mut_ptr() as *mut c_void, 4, 0, 0);
        diff_eq_val("e33 null-ness", cv.is_null(), rv.is_null());
        assert!(cv.is_null(), "C must return NULL");

        // row 34: hash_table == 0 -> temp = 0, returned unchanged
        pin_seed(&c, &r, 0x5555);
        let ct = (c.hmput_default)(std::ptr::null_mut(), es);
        let rt = (r.hmput_default)(std::ptr::null_mut(), es);
        // poison temp so we can see it being set to 0
        (*header(hash_to_arr(ct, es))).temp = 1234;
        (*header(hash_to_arr(rt, es))).temp = 1234;
        let mut key = 7u32.to_le_bytes();
        let (ct2, cd) = map_del(&c, ct, &cfg, &mut key);
        let mut key = 7u32.to_le_bytes();
        let (rt2, rd) = map_del(&r, rt, &cfg, &mut key);
        assert_eq!(ct2, ct, "returned unchanged");
        assert_eq!(rt2, rt);
        diff_eq_val("e34 temp", cd, rd);
        assert_eq!(cd, 0, "temp must be reset to 0");
        map_free(&c, ct2, es);
        map_free(&r, rt2, es);

        // row 35: key absent -> temp = 0, length unchanged
        pin_seed(&c, &r, 0x5556);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        for k in 0..20u32 {
            ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[4, 4, 4, 4]);
            rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[4, 4, 4, 4]);
        }
        let len = hm_len(ct, es);
        for k in 1000..1200u32 {
            let mut key = k.to_le_bytes();
            let (nct, cd) = map_del(&c, ct, &cfg, &mut key);
            let mut key = k.to_le_bytes();
            let (nrt, rd) = map_del(&r, rt, &cfg, &mut key);
            ct = nct;
            rt = nrt;
            diff_eq_val(&format!("e35 absent({k})"), cd, rd);
            assert_eq!(cd, 0);
            diff_eq(
                &format!("e35 absent({k}) state"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
        }
        assert_eq!(hm_len(ct, es), len, "length must be untouched");
        map_free(&c, ct, es);
        map_free(&r, rt, es);
    }
}

/// ERRORS.md row 37: `STBDS_ASSERT(table->used_count >= 0)` on an unsigned
/// `size_t` is vacuous — it must NEVER fire, not even when `--used_count` wraps
/// around to `SIZE_MAX`.  The condition is forced by zeroing `used_count` on
/// both tables before deleting an existing key.
#[test]
fn e37_used_count_underflow_is_not_an_error() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 8usize;
    let cfg = MapCfg::binary(es, 4);
    unsafe {
        pin_seed(&c, &r, 0x6666);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        for k in 0..5u32 {
            ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[5, 5, 5, 5]);
            rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[5, 5, 5, 5]);
        }
        // force used_count == 0 in both tables (identically)
        let cti = (*header(hash_to_arr(ct, es))).hash_table as *mut HashIndex;
        let rti = (*header(hash_to_arr(rt, es))).hash_table as *mut HashIndex;
        (*cti).used_count = 0;
        (*rti).used_count = 0;
        diff_eq(
            "e37 pre",
            &snapshot_map(ct, es, KeyKind::Raw),
            &snapshot_map(rt, es, KeyKind::Raw),
        );
        // delete the last element (no back-fill): --used_count wraps to SIZE_MAX
        let mut key = 4u32.to_le_bytes();
        let (ct, cd) = map_del(&c, ct, &cfg, &mut key);
        let mut key = 4u32.to_le_bytes();
        let (rt, rd) = map_del(&r, rt, &cfg, &mut key);
        diff_eq_val("e37 del result", cd, rd);
        assert_eq!(cd, 1, "the delete must succeed");
        let cti = map_table(ct, es).unwrap();
        assert_eq!(cti.used_count, usize::MAX, "used_count must wrap, not be clamped");
        diff_eq(
            "e37 post",
            &snapshot_map(ct, es, KeyKind::Raw),
            &snapshot_map(rt, es, KeyKind::Raw),
        );
        map_free(&c, ct, es);
        map_free(&r, rt, es);
    }
}

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// ERRORS.md row 38: the `strdup`'d key is freed **only** when `mode` is exactly
/// `STBDS_HM_STRING`; `mode = 2/3/999` hash as strings but skip the free.
///
/// Detected via glibc's LIFO tcache: right after the delete, a `malloc` of the
/// key's size class returns the *same* address iff that address was just freed.
/// The probe is self-validating — it must report "freed" for `mode == 1` and
/// "not freed" for `mode > 1` in the C reference, otherwise the test fails.
#[test]
fn e38_strdup_key_freed_only_for_mode_eq_1() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 8usize;
    unsafe {
        for &mode in &[1 as c_int, 2, 3, 999] {
            let mut freed_flags = Vec::new();
            for api in [&c, &r] {
                let cfg = MapCfg::string(es, mode);
                pin_seed(&c, &r, 0x7777);
                let mut keys: Vec<Vec<u8>> = (0..6)
                    .map(|i| {
                        let mut v = format!("a_reasonably_long_strdup_key_{i}").into_bytes();
                        v.push(0);
                        v
                    })
                    .collect();
                let klen = keys[5].len(); // == strlen + 1, the strdup size
                let mut t: *mut c_void = (api.shmode_func)(es, STBDS_SH_STRDUP);
                for k in keys.iter_mut() {
                    t = map_put_string(api, t, &cfg, k.as_mut_ptr() as *mut c_char, &[]);
                }
                // The element that will be deleted: the LAST inserted one, so
                // `old_index == final_index` and the *only* allocator activity of
                // the call is the conditional key free.
                let last = hm_len(t, es) - 1;
                let stored = *((t as *const u8).wrapping_offset(last * es as isize)
                    as *const *mut c_void);
                assert_eq!(
                    cstr(stored as *const c_char),
                    "a_reasonably_long_strdup_key_5"
                );
                let t = (api.hmdel_key)(t, es, keys[5].as_mut_ptr() as *mut c_void, 8, 0, mode);
                // tcache probe
                let probe = malloc(klen);
                let freed = probe == stored;
                free(probe);
                freed_flags.push(freed);
                (api.hmfree_func)(hash_to_arr(t, es), es);
            }
            println!(
                "e38 mode={mode}: C freed_key={} RUST freed_key={}",
                freed_flags[0], freed_flags[1]
            );
            diff_eq_val(
                &format!("e38 mode={mode} key-freed"),
                freed_flags[0],
                freed_flags[1],
            );
            if mode == STBDS_HM_STRING {
                assert!(
                    freed_flags[0],
                    "probe broken: mode==STBDS_HM_STRING must free the strdup'd key"
                );
            } else {
                assert!(
                    !freed_flags[0],
                    "mode>STBDS_HM_STRING must NOT free the strdup'd key"
                );
            }
        }
    }
}

/// ERRORS.md rows 40/41: for `mode > STBDS_HM_STRING` the back-fill re-find
/// hashes the *address* of the key field instead of the key, so
/// `STBDS_ASSERT(slot >= 0)` fires.  Both libraries must die the same way.
#[test]
fn e40_41_hmdel_backfill_with_mode_gt_1_aborts_in_both() {
    assert_same_fatal("hmdel_backfill_mode_gt_1");
}

/// ERRORS.md rows 39, 43, 44 — the three post-delete table transitions.
#[test]
fn e39_43_44_delete_transitions() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 8usize;
    let cfg = MapCfg::binary(es, 4);
    unsafe {
        pin_seed(&c, &r, 0x8888);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        for k in 0..40u32 {
            ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[6, 6, 6, 6]);
            rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[6, 6, 6, 6]);
        }
        let mut saw_shrink = false;
        let mut saw_rebuild = false;
        let mut saw_last = false;
        let mut saw_middle = false;
        // delete from the front: the first ones are back-fills (row 39 false),
        // the very last one is the last element (row 39 true)
        for k in 0..40u32 {
            let pre = map_table(ct, es).unwrap();
            let pre_slots = pre.slot_count;
            let pre_tombs = pre.tombstone_count;
            let final_index = hm_len(ct, es) - 1;
            let mut key = k.to_le_bytes();
            let (nct, cd) = map_geti(&c, ct, &cfg, &mut key);
            ct = nct;
            let old_index = cd;
            if old_index == final_index {
                saw_last = true;
            } else {
                saw_middle = true;
            }
            let mut key = k.to_le_bytes();
            let (nct, cd) = map_del(&c, ct, &cfg, &mut key);
            let mut key = k.to_le_bytes();
            let (nrt, rd) = map_del(&r, rt, &cfg, &mut key);
            ct = nct;
            rt = nrt;
            diff_eq_val(&format!("e39 del({k})"), cd, rd);
            let post = map_table(ct, es).unwrap();
            if post.slot_count < pre_slots {
                saw_shrink = true;
            } else if post.tombstone_count < pre_tombs {
                saw_rebuild = true;
            }
            diff_eq(
                &format!("e39 del({k}) state"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
        }
        assert!(saw_last, "row 39: the `old_index == final_index` path was never taken");
        assert!(saw_middle, "row 39: the back-fill path was never taken");
        assert!(saw_shrink, "row 43: the shrink path was never taken");
        assert!(saw_rebuild, "row 44: the tombstone rebuild path was never taken");
        map_free(&c, ct, es);
        map_free(&r, rt, es);
    }
}

/// ERRORS.md row 45 — `keyoffset` that does not match how the key was stored.
#[test]
fn e45_hmdel_key_mismatched_keyoffset() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 16usize;
    unsafe {
        for &ko in &[0usize, 1, 2, 4, 8, 12, 15] {
            let mut cfg = MapCfg::binary(es, 4);
            cfg.del_keyoffset = ko;
            pin_seed(&c, &r, 0x9999);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            for k in 0..20u32 {
                ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[7u8; 16]);
                rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[7u8; 16]);
            }
            for k in 0..20u32 {
                let mut key = k.to_le_bytes();
                let (nct, cd) = map_del(&c, ct, &cfg, &mut key);
                let mut key = k.to_le_bytes();
                let (nrt, rd) = map_del(&r, rt, &cfg, &mut key);
                ct = nct;
                rt = nrt;
                diff_eq_val(&format!("e45 ko={ko} del({k})"), cd, rd);
                diff_eq(
                    &format!("e45 ko={ko} del({k}) state"),
                    &snapshot_map(ct, es, KeyKind::Raw),
                    &snapshot_map(rt, es, KeyKind::Raw),
                );
            }
            map_free(&c, ct, es);
            map_free(&r, rt, es);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 46..50, 53 — string arena
// ===========================================================================

#[test]
fn e46_50_stralloc_branches() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        // row 46: len > remaining -> allocate; row 47/48: block ladder + clamp
        for &blk in &[0u8, 1, 2, 21, 22, 23, 254, 255] {
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            ca.block = blk;
            ra.block = blk;
            for &l in &[1usize, 100, 512, 513, 2000] {
                let mut s: Vec<u8> = (0..l).map(|i| b'x'.wrapping_add((i % 7) as u8)).collect();
                s.push(0);
                let cp = (c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let rp = (r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                diff_eq_val(&format!("e46 blk={blk} l={l} content"), cstr(cp), cstr(rp));
                diff_eq_val(&format!("e47 blk={blk} l={l} block"), ca.block, ra.block);
                diff_eq_val(&format!("e46 blk={blk} l={l} remaining"), ca.remaining, ra.remaining);
                diff_eq_val(
                    &format!("e46 blk={blk} l={l} storage"),
                    ca.storage.is_null(),
                    ra.storage.is_null(),
                );
            }
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
        }

        // row 50: oversize with storage == NULL -> remaining = 0
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut s: Vec<u8> = vec![b'z'; 5000];
        s.push(0);
        let cp = (c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
        let rp = (r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
        diff_eq_val("e50 content", cstr(cp), cstr(rp));
        diff_eq_val("e50 remaining", ca.remaining, ra.remaining);
        assert_eq!(ca.remaining, 0);
        diff_eq_val("e50 block", ca.block, ra.block);
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);

        // row 49: oversize with storage != NULL -> remaining preserved
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut small = *b"seed\0";
        (c.stralloc)(&mut ca, small.as_mut_ptr() as *mut c_char);
        (r.stralloc)(&mut ra, small.as_mut_ptr() as *mut c_char);
        let rem_c = ca.remaining;
        let rem_r = ra.remaining;
        diff_eq_val("e49 seed remaining", rem_c, rem_r);
        let cp = (c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
        let rp = (r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
        diff_eq_val("e49 content", cstr(cp), cstr(rp));
        diff_eq_val("e49 remaining preserved", ca.remaining, ra.remaining);
        assert_eq!(ca.remaining, rem_c);
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }
}

#[test]
fn e52_stralloc_null_args_are_fatal_in_both() {
    assert_same_fatal("stralloc_null_arena");
    assert_same_fatal("stralloc_null_str");
}

#[test]
fn e53_strreset_on_empty_arena() {
    let (c, r) = load_both();
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        ca.block = 5;
        ra.block = 5;
        ca.mode = 3;
        ra.mode = 3;
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        let cs = format!("{} {} {} {}", ca.remaining, ca.block, ca.mode, ca.storage.is_null());
        let rs = format!("{} {} {} {}", ra.remaining, ra.block, ra.mode, ra.storage.is_null());
        diff_eq_val("e53", cs.clone(), rs);
        assert_eq!(cs, "0 0 0 true");
        // idempotent
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }
}

#[test]
fn e54_strreset_null_is_fatal_in_both() {
    assert_same_fatal("strreset_null");
}

// ===========================================================================
// ERRORS.md row 56 — strkey
// ===========================================================================

#[test]
fn e56_strkey_extremes() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &n in &[0 as c_int, -1, c_int::MIN, c_int::MAX, -2147483647] {
            let cp = (c.strkey)(n);
            let rp = (r.strkey)(n);
            diff_eq_val(&format!("e56 strkey({n})"), cstr(cp), cstr(rp));
            assert!(cstr(cp).len() < 256, "no overflow of the 256 byte buffer");
        }
        // the previous result is clobbered by the next call (same static buffer)
        let p1 = (c.strkey)(1);
        let s1 = cstr(p1);
        let _ = (c.strkey)(2);
        let s1b = cstr(p1);
        let q1 = (r.strkey)(1);
        let t1 = cstr(q1);
        let _ = (r.strkey)(2);
        let t1b = cstr(q1);
        diff_eq_val("e56 clobber before", s1, t1);
        diff_eq_val("e56 clobber after", s1b.clone(), t1b);
        assert_eq!(s1b, "test_2", "the static buffer must be reused");
    }
}

// ===========================================================================
// Extra generic boundaries (zero / degenerate sizes)
// ===========================================================================

/// `keysize == 0`: `memcmp(..., 0) == 0` makes *every* key compare equal and
/// `hash_bytes(k, 0, seed)` makes every key hash the same, so the map collapses
/// to a single entry.  Fully defined, and a genuine zero-length boundary.
#[test]
fn e57_keysize_zero() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &es in &[8usize, 16] {
            let cfg = MapCfg {
                elemsize: es,
                keysize: 0,
                mode: STBDS_HM_BINARY,
                del_keyoffset: 0,
                kind: KeyKind::Raw,
            };
            pin_seed(&c, &r, 0xaaaa);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            for k in 0..10u64 {
                let pl = vec![k as u8; es];
                ct = map_put_binary(&c, ct, &cfg, &[], &pl);
                rt = map_put_binary(&r, rt, &cfg, &[], &pl);
                diff_eq(
                    &format!("e57 es={es} put({k})"),
                    &snapshot_map(ct, es, KeyKind::Raw),
                    &snapshot_map(rt, es, KeyKind::Raw),
                );
            }
            assert_eq!(hm_len(ct, es), 1, "all zero-length keys collapse into one entry");
            let (nct, ci) = map_geti(&c, ct, &cfg, &mut []);
            let (nrt, ri) = map_geti(&r, rt, &cfg, &mut []);
            ct = nct;
            rt = nrt;
            diff_eq_val(&format!("e57 es={es} get"), ci, ri);
            let (nct, cd) = map_del(&c, ct, &cfg, &mut []);
            let (nrt, rd) = map_del(&r, rt, &cfg, &mut []);
            ct = nct;
            rt = nrt;
            diff_eq_val(&format!("e57 es={es} del"), cd, rd);
            diff_eq(
                &format!("e57 es={es} after del"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
            map_free(&c, ct, es);
            map_free(&r, rt, es);
        }
    }
}

/// `elemsize == 0` together with `keysize == 0`: every element lives at the same
/// (zero-sized) address and nothing is ever written to it, so the whole
/// lifecycle is well defined and must still match bit for bit.
#[test]
fn e58_elemsize_zero() {
    let _g = global_lock();
    let (c, r) = load_both();
    let es = 0usize;
    let cfg = MapCfg {
        elemsize: es,
        keysize: 0,
        mode: STBDS_HM_BINARY,
        del_keyoffset: 0,
        kind: KeyKind::Raw,
    };
    unsafe {
        pin_seed(&c, &r, 0xbbbb);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        for k in 0..6u64 {
            ct = (c.hmput_key)(ct, es, std::ptr::null_mut(), 0, 0);
            rt = (r.hmput_key)(rt, es, std::ptr::null_mut(), 0, 0);
            diff_eq(
                &format!("e58 put({k})"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
        }
        let (nct, ci) = map_geti(&c, ct, &cfg, &mut []);
        let (nrt, ri) = map_geti(&r, rt, &cfg, &mut []);
        ct = nct;
        rt = nrt;
        diff_eq_val("e58 get", ci, ri);
        let (nct, cd) = map_del(&c, ct, &cfg, &mut []);
        let (nrt, rd) = map_del(&r, rt, &cfg, &mut []);
        ct = nct;
        rt = nrt;
        diff_eq_val("e58 del", cd, rd);
        diff_eq(
            "e58 after del",
            &snapshot_map(ct, es, KeyKind::Raw),
            &snapshot_map(rt, es, KeyKind::Raw),
        );
        map_free(&c, ct, es);
        map_free(&r, rt, es);
    }
}
