//! Phase C — error-path differential tests, one per row of ERRORS.md.
//!
//! Rows whose C behaviour is a *crash* (NULL dereference or a live `assert`)
//! are compared by re-executing this very test binary in a child process and
//! asserting that BOTH libraries terminate with the SAME signal / exit code.
mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::process::Command;

const ES: usize = 8; // `struct { int key; int value; }`

fn i32k(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
fn int_map() -> ElemDesc {
    ElemDesc::all_raw(8)
}
fn str_map() -> ElemDesc {
    ElemDesc::ptr_key(16)
}

// ===========================================================================
// subprocess plumbing
// ===========================================================================

fn child_status(case: &str, which: &str) -> (Option<i32>, Option<i32>, String) {
    use std::os::unix::process::ExitStatusExt;
    // The Rust child always loads the RELEASE cdylib: that is the shipped
    // artifact, and unlike a debug build it contains no `debug_assertions`
    // instrumentation, so a NULL / misaligned raw dereference faults exactly
    // like the C code instead of turning into a Rust panic + abort().
    let rel = release_so_path();
    assert!(
        rel.exists(),
        "crash-parity tests need the release cdylib.\n\
         Run `cargo build --release` (or ./run_tests.sh) first; expected {}",
        rel.display()
    );
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "err_child_runner", "--nocapture", "--test-threads=1"])
        .env("DIFF_CRASH_CASE", case)
        .env("DIFF_CRASH_LIB", which)
        .env("DIFF_RUST_SO", &rel)
        .output()
        .expect("spawn child");
    (
        out.status.code(),
        out.status.signal(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// Run `case` against the C `.so` and against the Rust `.so` in separate
/// processes and require identical termination.
fn assert_same_crash(case: &str) {
    let (cc, cs, cerr) = child_status(case, "c");
    let (rc, rs, rerr) = child_status(case, "rust");
    assert_eq!(
        (cs, cc),
        (rs, rc),
        "case `{case}`: C exited (signal={cs:?} code={cc:?}) but RUST exited \
         (signal={rs:?} code={rc:?})\n--- C stderr ---\n{}\n--- RUST stderr ---\n{}",
        cerr.lines().rev().take(6).collect::<Vec<_>>().join("\n"),
        rerr.lines().rev().take(6).collect::<Vec<_>>().join("\n"),
    );
    assert!(
        cs.is_some() || cc == Some(101),
        "case `{case}` was expected to terminate abnormally, got code {cc:?}"
    );
    eprintln!("case `{case}`: both terminated with signal={cs:?} code={cc:?}");
}

/// The child entry point. Not a real test: it does nothing unless
/// `DIFF_CRASH_CASE` is set, so it is a harmless no-op in normal runs.
#[test]
fn err_child_runner() {
    let case = match std::env::var("DIFF_CRASH_CASE") {
        Ok(c) => c,
        Err(_) => return,
    };
    let which = std::env::var("DIFF_CRASH_LIB").unwrap();
    let l = libs();
    let lib = if which == "c" { &l.c } else { &l.r };
    unsafe {
        (lib.rand_seed)(0x3141_5926);
        match case.as_str() {
            // ERRORS.md row 4
            "arrfreef_null" => (lib.arrfreef)(std::ptr::null_mut()),
            // row 6
            "hash_string_null" => {
                let h = (lib.hash_string)(std::ptr::null_mut(), 1);
                println!("unexpectedly returned {h:#x}");
            }
            // row 8
            "hash_bytes_null_len1" => {
                let h = (lib.hash_bytes)(std::ptr::null_mut(), 1, 1);
                println!("unexpectedly returned {h:#x}");
            }
            "hash_bytes_null_len8" => {
                let h = (lib.hash_bytes)(std::ptr::null_mut(), 8, 1);
                println!("unexpectedly returned {h:#x}");
            }
            // row 17
            "hmget_ts_null_temp" => {
                let k = i32k(1);
                let t = (lib.hmget_key_ts)(
                    std::ptr::null_mut(),
                    ES,
                    k.as_ptr() as *mut c_void,
                    4,
                    std::ptr::null_mut(),
                    HM_BINARY,
                );
                println!("unexpectedly returned {t:p}");
            }
            // row 36
            "stralloc_null_str" => {
                let mut a = Arena::zeroed();
                let p = (lib.stralloc)(&mut a, std::ptr::null_mut());
                println!("unexpectedly returned {p:p}");
            }
            // row 41
            "strreset_null" => (lib.strreset)(std::ptr::null_mut()),
            // row 38 (extreme): a->block = 200 => 512 << (100 & 63) = 1<<45
            // bytes; realloc fails and the unchecked result is dereferenced.
            "stralloc_huge_block" => {
                let mut a = Arena::zeroed();
                a.block = 200;
                let s = b"hello\0";
                let p = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                println!("unexpectedly returned {p:p}");
            }
            // row 33: assert(slot >= 0) in the swap-with-last re-find
            "hmdel_keyoffset_assert_slot" => {
                let mut t: *mut u8 = std::ptr::null_mut();
                for &(k, v) in &[(1i32, 1i32), (2, 2), (3, 99)] {
                    let kb = i32k(k);
                    t = (lib.hmput_key)(t as *mut c_void, ES, kb.as_ptr() as *mut c_void, 4, 0)
                        as *mut u8;
                    let idx = temp_of(t, ES);
                    *(t.add(ES * idx as usize) as *mut i32) = k;
                    *(t.add(ES * idx as usize + 4) as *mut i32) = v;
                }
                let kb = i32k(1);
                t = (lib.hmdel_key)(t as *mut c_void, ES, kb.as_ptr() as *mut c_void, 4, 4, 0)
                    as *mut u8;
                println!("unexpectedly survived, t={t:p}");
            }
            // row 34: assert(b->index[i] == final_index)
            "hmdel_keyoffset_assert_index" => {
                let mut t: *mut u8 = std::ptr::null_mut();
                for &(k, v) in &[(1i32, 1i32), (2, 2), (3, 2)] {
                    let kb = i32k(k);
                    t = (lib.hmput_key)(t as *mut c_void, ES, kb.as_ptr() as *mut c_void, 4, 0)
                        as *mut u8;
                    let idx = temp_of(t, ES);
                    *(t.add(ES * idx as usize) as *mut i32) = k;
                    *(t.add(ES * idx as usize + 4) as *mut i32) = v;
                }
                let kb = i32k(1);
                t = (lib.hmdel_key)(t as *mut c_void, ES, kb.as_ptr() as *mut c_void, 4, 4, 0)
                    as *mut u8;
                println!("unexpectedly survived, t={t:p}");
            }
            other => panic!("unknown crash case `{other}`"),
        }
    }
    println!("child for `{case}` finished normally");
}

// ===========================================================================
// row 1 — arrgrowf: min_cap <= arrcap  =>  returns `a` unchanged, no alloc
// ===========================================================================
#[test]
fn err_01_arrgrowf_no_growth() {
    let l = libs();
    unsafe {
        // from NULL with min_cap 0 and addlen 0: 0 <= 0 => returns NULL
        for es in [0usize, 1, 8, 64, usize::MAX] {
            let c = (l.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let r = (l.r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(c.is_null(), "C must return NULL for es={es}");
            assert!(r.is_null(), "RUST must return NULL for es={es}");
        }
        // from an existing array: any min_cap <= capacity is a no-op
        let c0 = (l.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 1) as *mut u8;
        let r0 = (l.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 1) as *mut u8;
        let ccap = (*(c0.sub(HEADER_SIZE) as *const ArrHeader)).capacity;
        let rcap = (*(r0.sub(HEADER_SIZE) as *const ArrHeader)).capacity;
        assert_eq!(ccap, 4);
        assert_eq!(rcap, 4);
        for cap in 0..=4usize {
            let c = (l.c.arrgrowf)(c0 as *mut c_void, 8, 0, cap) as *mut u8;
            let r = (l.r.arrgrowf)(r0 as *mut c_void, 8, 0, cap) as *mut u8;
            assert_eq!(c, c0, "C must return the same pointer for min_cap={cap}");
            assert_eq!(r, r0, "RUST must return the same pointer for min_cap={cap}");
        }
        (l.c.arrfreef)(c0 as *mut c_void);
        (l.r.arrfreef)(r0 as *mut c_void);
    }
}

// ===========================================================================
// row 2 — arrgrowf capacity floor of 4
// ===========================================================================
#[test]
fn err_02_arrgrowf_cap_floor() {
    let l = libs();
    unsafe {
        for (addlen, min_cap) in [(0usize, 1usize), (0, 2), (0, 3), (1, 0), (2, 0), (3, 0)] {
            let c = (l.c.arrgrowf)(std::ptr::null_mut(), 8, addlen, min_cap) as *mut u8;
            let r = (l.r.arrgrowf)(std::ptr::null_mut(), 8, addlen, min_cap) as *mut u8;
            let ch = *(c.sub(HEADER_SIZE) as *const ArrHeader);
            let rh = *(r.sub(HEADER_SIZE) as *const ArrHeader);
            assert_eq!((ch.length, ch.capacity, ch.temp), (0, 4, 0));
            assert_eq!((rh.length, rh.capacity, rh.temp), (0, 4, 0));
            assert!(ch.hash_table.is_null() && rh.hash_table.is_null());
            (l.c.arrfreef)(c as *mut c_void);
            (l.r.arrfreef)(r as *mut c_void);
        }
        // and 4 or more is taken verbatim
        for min_cap in [4usize, 5, 9, 40] {
            let c = (l.c.arrgrowf)(std::ptr::null_mut(), 8, 0, min_cap) as *mut u8;
            let r = (l.r.arrgrowf)(std::ptr::null_mut(), 8, 0, min_cap) as *mut u8;
            assert_eq!((*(c.sub(HEADER_SIZE) as *const ArrHeader)).capacity, min_cap);
            assert_eq!((*(r.sub(HEADER_SIZE) as *const ArrHeader)).capacity, min_cap);
            (l.c.arrfreef)(c as *mut c_void);
            (l.r.arrfreef)(r as *mut c_void);
        }
    }
}

// ===========================================================================
// row 3 — `elemsize * min_cap + sizeof(header)` wraps size_t: the allocation
//         SUCCEEDS with a tiny block and a bogus `capacity` is reported.
// ===========================================================================
#[test]
fn err_03_arrgrowf_size_wrap() {
    let l = libs();
    unsafe {
        // (1<<62)*8 == 1<<65 == 0 (mod 2^64); +32 => a 32-byte allocation
        for &(es, cap) in &[
            (1usize << 62, 8usize),
            (1 << 63, 4),
            (1 << 61, 16),
            (usize::MAX, 0),
        ] {
            let min_cap = if cap == 0 { 4 } else { cap };
            let c = (l.c.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap) as *mut u8;
            let r = (l.r.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap) as *mut u8;
            assert_eq!(c.is_null(), r.is_null(), "es={es} cap={min_cap}");
            if c.is_null() {
                continue;
            }
            let ch = *(c.sub(HEADER_SIZE) as *const ArrHeader);
            let rh = *(r.sub(HEADER_SIZE) as *const ArrHeader);
            assert_eq!(
                (ch.length, ch.capacity, ch.temp),
                (rh.length, rh.capacity, rh.temp),
                "es={es} cap={min_cap}: header mismatch"
            );
            assert_eq!(ch.capacity, min_cap, "the bogus capacity is reported as-is");
            (l.c.arrfreef)(c as *mut c_void);
            (l.r.arrfreef)(r as *mut c_void);
        }
    }
}

// ===========================================================================
// row 4 / 6 / 8 / 17 / 36 / 41 / 38-extreme / 33 / 34 — crash parity
// ===========================================================================
#[test]
fn err_04_arrfreef_null() {
    assert_same_crash("arrfreef_null");
}
#[test]
fn err_06_hash_string_null() {
    assert_same_crash("hash_string_null");
}
#[test]
fn err_08_hash_bytes_null_nonzero_len() {
    assert_same_crash("hash_bytes_null_len1");
    assert_same_crash("hash_bytes_null_len8");
}
#[test]
fn err_17_hmget_ts_null_temp() {
    assert_same_crash("hmget_ts_null_temp");
}
#[test]
fn err_36_stralloc_null_str() {
    assert_same_crash("stralloc_null_str");
}
#[test]
fn err_41_strreset_null() {
    assert_same_crash("strreset_null");
}
#[test]
fn err_38b_stralloc_huge_block_alloc_failure() {
    assert_same_crash("stralloc_huge_block");
}
#[test]
fn err_33_hmdel_assert_slot_ge_zero() {
    assert_same_crash("hmdel_keyoffset_assert_slot");
}
#[test]
fn err_34_hmdel_assert_index_eq_final() {
    assert_same_crash("hmdel_keyoffset_assert_index");
}

// ===========================================================================
// row 7 — hash_bytes(NULL, 0, seed) does NOT crash and returns a real value
// ===========================================================================
#[test]
fn err_07_hash_bytes_null_len0() {
    let l = libs();
    unsafe {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, 12345] {
            let ch = (l.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let rh = (l.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(ch, rh, "hash_bytes(NULL,0,{seed:#x})");
            // and it equals hashing a real zero-length buffer
            let empty: [u8; 0] = [];
            assert_eq!(ch, (l.c.hash_bytes)(empty.as_ptr() as *mut c_void, 0, seed));
            assert_eq!(rh, (l.r.hash_bytes)(empty.as_ptr() as *mut c_void, 0, seed));
        }
        // empty *string* also works (the loop body never runs)
        let z = b"\0";
        for seed in [0usize, 1, usize::MAX] {
            assert_eq!(
                (l.c.hash_string)(z.as_ptr() as *mut c_char, seed),
                (l.r.hash_string)(z.as_ptr() as *mut c_char, seed)
            );
        }
    }
}

// ===========================================================================
// rows 9 / 10 / 45 — `mode` is an unvalidated C `int`
// ===========================================================================
#[test]
fn err_09_10_45_mode_enum_sweep() {
    let l = libs();
    let modes: [c_int; 14] = [
        i32::MIN,
        -1000,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        7,
        255,
        256,
        1000,
        i32::MAX,
    ];
    // (a) BINARY-vs-STRING dispatch: for a *binary* int map only mode <= 0 can
    //     find the key; mode >= 1 hashes the key bytes as a C string instead.
    for &mode in &modes {
        let mut m = MapPair::new(int_map(), 4, &format!("mode-sweep put mode={mode}"));
        m.seed(0x3141_5926);
        // build with mode 0 so the table is SH_NONE
        for i in 1..20i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
        }
        // now probe with the exotic mode; only the C/RUST agreement matters
        for i in 1..20i32 {
            let k = i32k(i);
            m.get_ts(k.as_ptr() as *mut c_void, 4, mode);
            m.get(k.as_ptr() as *mut c_void, 4, mode);
        }
        m.free();
    }
    // (b) `stbds_hmdel_key`'s `mode == 1` exact test vs `mode >= 1`:
    //     LIFO deletes only, so the address-dependent re-find never runs.
    for &mode in &modes {
        let mut m = MapPair::new(int_map(), 4, &format!("mode-sweep del mode={mode}"));
        m.seed(0x3141_5926);
        for i in 1..10i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
        }
        for i in (1..10i32).rev() {
            let k = i32k(i);
            m.del(k.as_ptr() as *mut c_void, 4, 0, mode);
        }
        m.free();
    }
    // (c) shmode_func with every mode, then a put in binary mode
    unsafe {
        for &mode in &modes {
            (l.c.rand_seed)(0x3141_5926);
            (l.r.rand_seed)(0x3141_5926);
            let ct = (l.c.shmode_func)(ES, mode) as *mut u8;
            let rt = (l.r.shmode_func)(ES, mode) as *mut u8;
            let ci = *((*header_of(ct, ES)).hash_table as *const HashIndex);
            let ri = *((*header_of(rt, ES)).hash_table as *const HashIndex);
            assert_eq!(
                ci.string.mode, ri.string.mode,
                "shmode_func({mode}) string.mode mismatch"
            );
            assert_eq!(
                ci.string.mode,
                (mode as u32 & 0xff) as u8,
                "shmode_func({mode}) must truncate to unsigned char"
            );
            (l.c.hmfree_func)(ct.sub(ES) as *mut c_void, ES);
            (l.r.hmfree_func)(rt.sub(ES) as *mut c_void, ES);
        }
    }
}

// ===========================================================================
// row 11 — hmfree_func(NULL, es) is the one NULL guard that just returns
// ===========================================================================
#[test]
fn err_11_hmfree_null() {
    let l = libs();
    unsafe {
        for es in [0usize, 1, 8, 16, usize::MAX] {
            (l.c.hmfree_func)(std::ptr::null_mut(), es);
            (l.r.hmfree_func)(std::ptr::null_mut(), es);
        }
    }
    // if either had crashed we would not get here
}

// ===========================================================================
// rows 12 / 16 / 18 — key-absent lookups report -1 everywhere
// ===========================================================================
#[test]
fn err_12_16_18_absent_key_reports_minus_one() {
    let mut rng = Rng::new(0xC0_0012);
    // binary
    {
        let mut m = MapPair::new(int_map(), 4, "absent binary");
        m.seed(0x3141_5926);
        for i in 0..64i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
        }
        for _ in 0..4000 {
            let k = 1_000_000 + (rng.next_u32() % 1_000_000) as i32;
            let kb = i32k(k);
            assert_eq!(m.get_ts(kb.as_ptr() as *mut c_void, 4, HM_BINARY), -1);
            assert_eq!(m.get(kb.as_ptr() as *mut c_void, 4, HM_BINARY), -1);
            // hmget_key must also record -1 in the header
            assert_eq!(unsafe { temp_of(m.ct, 8) }, -1);
            assert_eq!(unsafe { temp_of(m.rt, 8) }, -1);
        }
        m.free();
    }
    // string
    {
        let mut m = MapPair::new(str_map(), 8, "absent string");
        m.seed(0x3141_5926);
        let mut keys = Vec::new();
        for i in 0..40i64 {
            let k = leak_cstr(format!("present-{i}").as_bytes());
            m.put_string(k, &i.to_le_bytes(), HM_STRING);
            keys.push(k);
        }
        for i in 0..1000 {
            let k = leak_cstr(format!("missing-{i}").as_bytes());
            assert_eq!(m.get_ts(k as *mut c_void, 8, HM_STRING), -1);
            assert_eq!(m.get(k as *mut c_void, 8, HM_STRING), -1);
            free_raw(k);
        }
        m.free();
        for k in keys {
            free_raw(k);
        }
    }
}

// ===========================================================================
// row 14 — hmget_key_ts(NULL, ...) allocates and reports -1
// ===========================================================================
#[test]
fn err_14_hmget_ts_null_map() {
    let l = libs();
    unsafe {
        for es in [1usize, 4, 8, 16, 20] {
            for mode in [HM_BINARY, HM_STRING, -5, 9] {
                (l.c.rand_seed)(0x3141_5926);
                (l.r.rand_seed)(0x3141_5926);
                let key = b"abcdefghijkl\0";
                let mut ctemp: isize = 0x1234;
                let mut rtemp: isize = 0x1234;
                let ct = (l.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    es,
                    key.as_ptr() as *mut c_void,
                    4,
                    &mut ctemp,
                    mode,
                ) as *mut u8;
                let rt = (l.r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    es,
                    key.as_ptr() as *mut c_void,
                    4,
                    &mut rtemp,
                    mode,
                ) as *mut u8;
                assert_eq!(ctemp, -1, "es={es} mode={mode}: C *temp");
                assert_eq!(rtemp, -1, "es={es} mode={mode}: RUST *temp");
                assert!(!ct.is_null() && !rt.is_null());
                let d = ElemDesc::all_raw(es);
                assert_eq!(
                    snapshot_map(ct, &d),
                    snapshot_map(rt, &d),
                    "es={es} mode={mode}"
                );
                // a fresh map has length 1, no hash table
                let h = *header_of(ct, es);
                assert_eq!(h.length, 1);
                assert!(h.hash_table.is_null());
                (l.c.hmfree_func)(ct.sub(es) as *mut c_void, es);
                (l.r.hmfree_func)(rt.sub(es) as *mut c_void, es);
            }
        }
    }
}

// ===========================================================================
// row 15 — hmget_key_ts on a table-less map reports -1 and returns `a` as-is
// ===========================================================================
#[test]
fn err_15_hmget_ts_no_table() {
    let mut m = MapPair::new(int_map(), 4, "no-table get_ts");
    m.seed(0x3141_5926);
    m.hmput_default_raw(); // creates a map with hash_table == NULL
    let before_c = m.ct;
    let before_r = m.rt;
    for i in 0..50i32 {
        let k = i32k(i);
        assert_eq!(m.get_ts(k.as_ptr() as *mut c_void, 4, HM_BINARY), -1);
        assert_eq!(m.get(k.as_ptr() as *mut c_void, 4, HM_BINARY), -1);
    }
    assert_eq!(m.ct, before_c, "C must return `a` untouched");
    assert_eq!(m.rt, before_r, "RUST must return `a` untouched");
    unsafe {
        assert!((*header_of(m.ct, 8)).hash_table.is_null());
        assert!((*header_of(m.rt, 8)).hash_table.is_null());
    }
    m.free();
}

// ===========================================================================
// rows 19 / 20 / 21 — hmput_default: NULL, length==0, and the no-op path
// ===========================================================================
#[test]
fn err_19_20_21_hmput_default_paths() {
    let l = libs();
    unsafe {
        // row 19: NULL
        for es in [1usize, 8, 16, 24] {
            (l.c.rand_seed)(0x3141_5926);
            (l.r.rand_seed)(0x3141_5926);
            let ct = (l.c.hmput_default)(std::ptr::null_mut(), es) as *mut u8;
            let rt = (l.r.hmput_default)(std::ptr::null_mut(), es) as *mut u8;
            let d = ElemDesc::all_raw(es);
            assert_eq!(snapshot_map(ct, &d), snapshot_map(rt, &d), "es={es}");
            assert_eq!((*header_of(ct, es)).length, 1);
            assert!((*header_of(ct, es)).hash_table.is_null());

            // row 21: second call is a pure no-op
            let cs = snapshot_map(ct, &d);
            let ct2 = (l.c.hmput_default)(ct as *mut c_void, es) as *mut u8;
            let rt2 = (l.r.hmput_default)(rt as *mut c_void, es) as *mut u8;
            assert_eq!(ct2, ct, "C: no-op must return the same pointer");
            assert_eq!(rt2, rt, "RUST: no-op must return the same pointer");
            assert_eq!(cs, snapshot_map(ct2, &d), "C: no-op must not change state");
            assert_eq!(
                snapshot_map(ct2, &d),
                snapshot_map(rt2, &d),
                "es={es} after no-op"
            );

            (l.c.hmfree_func)(ct.sub(es) as *mut c_void, es);
            (l.r.hmfree_func)(rt.sub(es) as *mut c_void, es);
        }
        // row 20: length == 0
        for es in [1usize, 8, 20] {
            let ca = (l.c.arrgrowf)(std::ptr::null_mut(), es, 0, 1) as *mut u8;
            let ra = (l.r.arrgrowf)(std::ptr::null_mut(), es, 0, 1) as *mut u8;
            assert_eq!((*(ca.sub(HEADER_SIZE) as *const ArrHeader)).length, 0);
            // poison the element so the re-zero is observable
            std::ptr::write_bytes(ca, 0xAB, es);
            std::ptr::write_bytes(ra, 0xAB, es);
            let ct = (l.c.hmput_default)(ca.add(es) as *mut c_void, es) as *mut u8;
            let rt = (l.r.hmput_default)(ra.add(es) as *mut c_void, es) as *mut u8;
            let d = ElemDesc::all_raw(es);
            assert_eq!(snapshot_map(ct, &d), snapshot_map(rt, &d), "len0 es={es}");
            assert_eq!((*header_of(ct, es)).length, 1);
            let bytes = std::slice::from_raw_parts(ct.sub(es), es);
            assert!(bytes.iter().all(|&b| b == 0), "element must be re-zeroed");
            (l.c.hmfree_func)(ct.sub(es) as *mut c_void, es);
            (l.r.hmfree_func)(rt.sub(es) as *mut c_void, es);
        }
    }
}

// ===========================================================================
// row 22 — hmput_key(NULL, ...) never returns NULL
// ===========================================================================
#[test]
fn err_22_hmput_key_null_map() {
    let l = libs();
    unsafe {
        for es in [1usize, 4, 8, 16, 20] {
            for mode in [HM_BINARY, HM_STRING, -3, 5] {
                // A string-mode table stores an 8-byte `char *` in the element,
                // so `elemsize < 8` would overflow the heap block in BOTH
                // libraries (glibc aborts on the next free). Skip those.
                if mode >= 1 && es < 8 {
                    continue;
                }
                (l.c.rand_seed)(0x3141_5926);
                (l.r.rand_seed)(0x3141_5926);
                let key = b"abcdefghijklmnop\0";
                let ks = if mode >= 1 { 8 } else { es.min(8) };
                let ct = (l.c.hmput_key)(
                    std::ptr::null_mut(),
                    es,
                    key.as_ptr() as *mut c_void,
                    ks,
                    mode,
                ) as *mut u8;
                let rt = (l.r.hmput_key)(
                    std::ptr::null_mut(),
                    es,
                    key.as_ptr() as *mut c_void,
                    ks,
                    mode,
                ) as *mut u8;
                assert!(!ct.is_null(), "C returned NULL (es={es} mode={mode})");
                assert!(!rt.is_null(), "RUST returned NULL (es={es} mode={mode})");
                assert_eq!(temp_of(ct, es), temp_of(rt, es));
                assert_eq!(temp_of(ct, es), 0, "first insert lands at index 0");
                (l.c.hmfree_func)(ct.sub(es) as *mut c_void, es);
                (l.r.hmfree_func)(rt.sub(es) as *mut c_void, es);
            }
        }
    }
}

// ===========================================================================
// rows 23 / 24 — duplicate key found in the FIRST vs the SECOND probe loop.
// The second loop deliberately does NOT update `temp_key`; both loops must be
// reached and both libraries must agree.
// ===========================================================================
#[test]
fn err_23_24_duplicate_key_both_probe_loops() {
    let l = libs();
    let mut first_loop = 0usize;
    let mut second_loop = 0usize;
    unsafe {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, 0xa5a5_a5a5] {
            let mut m = MapPair::new(int_map(), 4, &format!("dup-loops seed={seed:#x}"));
            m.seed(seed);
            // 6 entries in an 8-slot table => a very dense bucket
            for i in 0..6i32 {
                m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
            }
            let ti = *((*header_of(m.ct, 8)).hash_table as *const HashIndex);
            assert_eq!(ti.slot_count, 8);
            let tbl_seed = ti.seed;
            for i in 0..6i32 {
                let kb = i32k(i);
                // work out which probe loop the C code will match in
                let mut h = (l.c.hash_bytes)(kb.as_ptr() as *mut c_void, 4, tbl_seed);
                if h < 2 {
                    h += 2;
                }
                let start = h & (ti.slot_count - 1) & BUCKET_MASK;
                let bk = *ti.storage;
                let mut at = usize::MAX;
                for s in 0..BUCKET_LENGTH {
                    if bk.hash[s] == h {
                        at = s;
                        break;
                    }
                }
                assert_ne!(at, usize::MAX, "key {i} not in the bucket");
                if at >= start {
                    first_loop += 1;
                } else {
                    second_loop += 1;
                }
                m.put_binary(&kb, &i32k(i * 31), HM_BINARY);
                assert_eq!(temp_of(m.ct, 8), temp_of(m.rt, 8));
            }
            assert_eq!(hmlen(m.ct, 8), 6, "duplicates must not grow the map");
            m.free();
        }
    }
    assert!(first_loop > 0, "the first probe loop was never exercised");
    assert!(
        second_loop > 0,
        "the SECOND probe loop (the one that skips temp_key) was never exercised"
    );
    eprintln!("dup found in first loop {first_loop}x, second loop {second_loop}x");
}

// ===========================================================================
// row 24 (targeted) — on a STRING map, a duplicate key found in the SECOND
// probe loop must leave `table->temp_key` UNCHANGED, while one found in the
// FIRST loop must set it to the ALREADY-STORED pointer (not the caller's).
//
// Duplicate puts deliberately use a *different* `char*` with identical content,
// so "temp_key = stored pointer" and "temp_key = caller's pointer" are
// distinguishable.
// ===========================================================================
#[test]
fn err_24b_temp_key_first_vs_second_probe_loop() {
    let l = libs();
    let mut rng = Rng::new(0xC0_0024);
    let mut first_loop = 0usize;
    let mut second_loop = 0usize;
    let mut trials = 0usize;

    // Search over hash seeds / key counts / key spellings until BOTH probe loops
    // have been observed. The interesting shape is a bucket dense enough that
    // the `pos&MASK .. 8` scan finds no empty slot, so the entry is only reached
    // by the `0 .. limit` wrap-around scan.
    //
    // Counts are kept strictly below `used_count_threshold` (8 slots -> 6,
    // 16 -> 12, 32 -> 24) so that a duplicate put cannot trigger the grow at the
    // top of `stbds_hmput_key`, which would rehash into a table whose layout is
    // not the one we measured (and whose `temp_key` is uninitialised).
    'outer: for round in 0..3000u64 {
        for &count in &[5usize, 11, 23] {
            trials += 1;
            let seed = if round == 0 {
                0x3141_5926
            } else {
                rng.next_u64() as usize
            };
            let mut m = MapPair::new(str_map(), 8, &format!("temp_key seed={seed:#x} n={count}"));
            m.seed(seed);
            let mut orig: Vec<(*mut c_char, Vec<u8>)> = Vec::new();
            for i in 0..count {
                let kb = format!("k{round}-{i}").into_bytes();
                let k = leak_cstr(&kb);
                m.put_string(k, &(i as i64).to_le_bytes(), HM_STRING);
                orig.push((k, kb));
            }
            unsafe {
                for (k, kb) in &orig {
                    // Re-read the live table for EVERY probe.
                    let tp = (*header_of(m.ct, 16)).hash_table as *const HashIndex;
                    let ti = *tp;
                    assert_eq!(ti.string.mode, 1, "expected SH_DEFAULT");
                    if ti.used_count >= ti.used_count_threshold {
                        continue; // a grow is imminent; layout would change
                    }
                    let mut h = (l.c.hash_string)(*k, ti.seed);
                    if h < 2 {
                        h += 2;
                    }
                    let pos = h & (ti.slot_count - 1);
                    let bidx = pos >> BUCKET_SHIFT;
                    let start = pos & BUCKET_MASK;
                    let bk = *ti.storage.add(bidx);
                    // Only classify when the entry is in the FIRST probed bucket
                    // and the scan from `start` cannot bail out on an empty slot
                    // before reaching it; otherwise the multi-bucket
                    // `pos += step` walk decides and attribution is ambiguous.
                    let at = match (0..BUCKET_LENGTH).find(|&s| bk.hash[s] == h) {
                        Some(s) => s,
                        None => continue,
                    };
                    let empty_before_entry = if at >= start {
                        (start..at).any(|s| bk.hash[s] == 0)
                    } else {
                        (start..BUCKET_LENGTH).any(|s| bk.hash[s] == 0)
                            || (0..at).any(|s| bk.hash[s] == 0)
                    };
                    if empty_before_entry {
                        continue;
                    }

                    let before_c = temp_key_of(m.ct, 16);
                    let before_r = temp_key_of(m.rt, 16);
                    assert_eq!(before_c, before_r, "temp_key must already agree");

                    // a *different* pointer with the same contents, so
                    // "stored pointer" and "caller pointer" are distinguishable
                    let dup = leak_cstr(kb);
                    assert_ne!(dup, *k);
                    let len_before = hmlen(m.ct, 16);
                    m.put_string(dup, &(-1i64).to_le_bytes(), HM_STRING);
                    assert_eq!(hmlen(m.ct, 16), len_before, "duplicate must not insert");
                    assert_eq!(
                        (*header_of(m.ct, 16)).hash_table as *const HashIndex,
                        tp,
                        "the table must not have been reallocated"
                    );

                    let after_c = temp_key_of(m.ct, 16);
                    let after_r = temp_key_of(m.rt, 16);
                    assert_eq!(
                        after_c, after_r,
                        "C and RUST temp_key must agree (start={start} at={at})"
                    );
                    assert_ne!(
                        after_c, dup,
                        "temp_key must never become the caller's new pointer \
                         (start={start} at={at})"
                    );
                    if at >= start {
                        first_loop += 1;
                        assert_eq!(
                            after_c, *k,
                            "first probe loop must report the ALREADY-STORED pointer \
                             (start={start} at={at})"
                        );
                    } else {
                        second_loop += 1;
                        assert_eq!(
                            after_c, before_c,
                            "second probe loop must leave temp_key UNTOUCHED \
                             (start={start} at={at})"
                        );
                    }
                    free_raw(dup);
                }
            }
            m.free();
            for (k, _) in orig {
                free_raw(k);
            }
            if first_loop >= 50 && second_loop >= 20 {
                break 'outer;
            }
        }
    }
    assert!(first_loop > 0, "the FIRST probe loop was never exercised");
    assert!(
        second_loop > 0,
        "the SECOND probe loop (which must NOT touch temp_key) was never \
         exercised in {trials} trials"
    );
    eprintln!(
        "temp_key: first loop {first_loop}x, second loop {second_loop}x ({trials} trials)"
    );
}

// ===========================================================================
// rows 26 / 27 — shmode_func truncation + the `default:` memcpy branch
// ===========================================================================
#[test]
fn err_26_27_shmode_truncation_and_default_branch() {
    let l = libs();
    // row 27: `(unsigned char) mode` truncation, no validation
    unsafe {
        for &(mode, want) in &[
            (0i32, 0u8),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (255, 255),
            (256, 0),
            (257, 1),
            (259, 3),
            (-1, 255),
            (-256, 0),
            (i32::MIN, 0),
            (i32::MAX, 255),
        ] {
            (l.c.rand_seed)(0x3141_5926);
            (l.r.rand_seed)(0x3141_5926);
            let ct = (l.c.shmode_func)(16, mode) as *mut u8;
            let rt = (l.r.shmode_func)(16, mode) as *mut u8;
            let ci = *((*header_of(ct, 16)).hash_table as *const HashIndex);
            let ri = *((*header_of(rt, 16)).hash_table as *const HashIndex);
            assert_eq!(ci.string.mode, want, "C shmode_func({mode})");
            assert_eq!(ri.string.mode, want, "RUST shmode_func({mode})");
            (l.c.hmfree_func)(ct.sub(16) as *mut c_void, 16);
            (l.r.hmfree_func)(rt.sub(16) as *mut c_void, 16);
        }
    }
    // row 26: a table whose string.mode is not 1/2/3 takes the `default:`
    // memcpy branch even for mode >= STBDS_HM_STRING.
    let mut rng = Rng::new(0xC0_0026);
    for &tmode in &[0i32, 4, 5, 255, 256, 260] {
        let mut m = MapPair::new(
            ElemDesc::all_raw(16),
            8,
            &format!("default-branch table_mode={tmode}"),
        );
        m.seed(0x3141_5926);
        m.shmode(tmode);
        for i in 0..30i64 {
            let n = 8 + rng.below(20);
            let kb = rng.cstr_bytes(n, false);
            let k = leak_cstr(&kb);
            m.put_raw_keysize(k, 8, &i.to_le_bytes(), HM_STRING);
            // the element must literally contain the first 8 key bytes
            unsafe {
                let idx = temp_of(m.ct, 16) as usize;
                let ce = std::slice::from_raw_parts(m.ct.add(16 * idx), 8);
                let re = std::slice::from_raw_parts(m.rt.add(16 * idx), 8);
                assert_eq!(ce, &kb[..8], "C default: branch must memcpy the key bytes");
                assert_eq!(re, &kb[..8], "RUST default: branch must memcpy the key bytes");
            }
            free_raw(k);
        }
        m.free();
    }
}

// ===========================================================================
// rows 28 / 29 / 30 — hmdel_key rejections
// ===========================================================================
#[test]
fn err_28_29_30_hmdel_rejections() {
    let l = libs();
    // row 28: NULL map => returns NULL
    unsafe {
        for es in [1usize, 8, 16] {
            for mode in [HM_BINARY, HM_STRING, -1, 4] {
                for keyoffset in [0usize, 4, 8] {
                    let key = b"abcdefgh\0";
                    let c = (l.c.hmdel_key)(
                        std::ptr::null_mut(),
                        es,
                        key.as_ptr() as *mut c_void,
                        4,
                        keyoffset,
                        mode,
                    );
                    let r = (l.r.hmdel_key)(
                        std::ptr::null_mut(),
                        es,
                        key.as_ptr() as *mut c_void,
                        4,
                        keyoffset,
                        mode,
                    );
                    assert!(c.is_null(), "C hmdel_key(NULL) must return NULL");
                    assert!(r.is_null(), "RUST hmdel_key(NULL) must return NULL");
                }
            }
        }
    }
    // row 29: no hash table => temp = 0, `a` returned unchanged
    {
        let mut m = MapPair::new(int_map(), 4, "hmdel no table");
        m.seed(0x3141_5926);
        m.hmput_default_raw();
        let (bc, br) = (m.ct, m.rt);
        for i in 0..20i32 {
            let k = i32k(i);
            assert_eq!(m.del(k.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 0);
            assert_eq!(unsafe { temp_of(m.ct, 8) }, 0);
            assert_eq!(unsafe { temp_of(m.rt, 8) }, 0);
        }
        assert_eq!(m.ct, bc);
        assert_eq!(m.rt, br);
        m.free();
    }
    // row 30: key absent => temp = 0, nothing changes
    {
        let mut rng = Rng::new(0xC0_0030);
        let mut m = MapPair::new(int_map(), 4, "hmdel missing key");
        m.seed(0x3141_5926);
        for i in 0..40i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
        }
        // NOTE: a failed delete *does* write `stbds_temp((t)-1) = 0` (lib.c:815)
        // before it bails out, so take the reference snapshot after one failed
        // delete has already normalised `temp`.
        let k0 = i32k(999_999);
        assert_eq!(m.del(k0.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 0);
        let (bc, _br) = m.snapshots();
        for _ in 0..3000 {
            let k = 500_000 + (rng.next_u32() % 500_000) as i32;
            let kb = i32k(k);
            assert_eq!(m.del(kb.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 0);
            assert_eq!(unsafe { temp_of(m.ct, 8) }, 0);
            assert_eq!(unsafe { temp_of(m.rt, 8) }, 0);
        }
        let (ac, _ar) = m.snapshots();
        assert_eq!(
            bc, ac,
            "apart from `temp`, a failed delete must not modify the map"
        );
        assert_eq!(unsafe { hmlen(m.ct, 8) }, 40);
        m.free();
    }
}

// ===========================================================================
// row 32 — `assert(table->used_count >= 0)` is DEAD (size_t): forcing
//          `used_count` to 0 makes `--used_count` wrap to SIZE_MAX and the
//          assert still does not fire, in BOTH libraries.
// ===========================================================================
#[test]
fn err_32_used_count_wraps_without_assert() {
    let mut m = MapPair::new(int_map(), 4, "used_count wrap");
    m.seed(0x3141_5926);
    for i in 0..5i32 {
        m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
    }
    unsafe {
        let ct = (*header_of(m.ct, 8)).hash_table as *mut HashIndex;
        let rt = (*header_of(m.rt, 8)).hash_table as *mut HashIndex;
        (*ct).used_count = 0;
        (*rt).used_count = 0;
    }
    // LIFO delete: no swap-with-last, so nothing address-dependent happens
    let k = i32k(4);
    assert_eq!(m.del(k.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 1);
    unsafe {
        let ct = *((*header_of(m.ct, 8)).hash_table as *const HashIndex);
        let rt = *((*header_of(m.rt, 8)).hash_table as *const HashIndex);
        assert_eq!(ct.used_count, usize::MAX, "C must wrap used_count");
        assert_eq!(rt.used_count, usize::MAX, "RUST must wrap used_count");
        assert_eq!(ct.slot_count, rt.slot_count);
        assert_eq!(ct.tombstone_count, rt.tombstone_count);
    }
    m.check("after used_count wrap");
    // do not free: the table bookkeeping is deliberately corrupt
    std::mem::forget(m);
}

// ===========================================================================
// row 35 — hmdel_key's `mode == STBDS_HM_STRING` EXACT test.
// With mode = 2 on an SH_STRDUP table the key is NOT freed and the re-find
// uses the binary key expression. Only LIFO deletes are used so the
// (address-dependent) re-find branch never runs.
// ===========================================================================
#[test]
fn err_35_hmdel_mode2_on_string_table() {
    for &tmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &mode in &[2i32, 3, 7, 1000, i32::MAX] {
            let mut m = MapPair::new(
                str_map(),
                8,
                &format!("hmdel mode={mode} table={tmode}"),
            );
            m.seed(0x3141_5926);
            m.shmode(tmode);
            let mut keys = Vec::new();
            for i in 0..12i64 {
                let k = leak_cstr(format!("key-number-{i:04}").as_bytes());
                m.put_string(k, &i.to_le_bytes(), mode);
                keys.push(k);
            }
            // LIFO => old_index == final_index => no re-find
            while let Some(k) = keys.pop() {
                assert_eq!(
                    m.del(k as *mut c_void, 8, 0, mode),
                    1,
                    "delete with mode={mode} on table={tmode}"
                );
                free_raw(k);
            }
            assert_eq!(unsafe { hmlen(m.ct, 16) }, 0);
            m.free();
        }
    }
}

// ===========================================================================
// rows 38 / 39 / 40 — stralloc block-size branches (non-crashing part)
// ===========================================================================
#[test]
fn err_38_39_40_stralloc_block_branches() {
    let l = libs();
    unsafe {
        // row 39: len > blocksize with storage == NULL  =>  remaining = 0
        for &len in &[512usize, 600, 5000] {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            let mut s = vec![b'x'; len];
            s.push(0);
            let cp = (l.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            let rp = (l.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            assert_eq!(ca.remaining, 0, "C: oversized first block => remaining 0");
            assert_eq!(ra.remaining, 0, "RUST: oversized first block => remaining 0");
            assert_eq!(ca.block, ra.block);
            assert_eq!(ca.block, 1, "the block counter is still bumped");
            assert_eq!(CStr::from_ptr(cp).to_bytes().len(), len);
            assert_eq!(CStr::from_ptr(rp).to_bytes().len(), len);
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
        }
        // row 40: len > blocksize with storage != NULL => remaining UNTOUCHED
        {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            let small = b"small\0";
            (l.c.stralloc)(&mut ca, small.as_ptr() as *mut c_char);
            (l.r.stralloc)(&mut ra, small.as_ptr() as *mut c_char);
            let crem = ca.remaining;
            let rrem = ra.remaining;
            assert_eq!(crem, rrem);
            let mut big = vec![b'B'; 4000];
            big.push(0);
            (l.c.stralloc)(&mut ca, big.as_ptr() as *mut c_char);
            (l.r.stralloc)(&mut ra, big.as_ptr() as *mut c_char);
            assert_eq!(ca.remaining, crem, "C: remaining must be untouched");
            assert_eq!(ra.remaining, rrem, "RUST: remaining must be untouched");
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
        }
        // row 38: `512 << (block>>1)` wraps to 0 for block>>1 >= 55, so every
        // string takes the dedicated-block path (and `block` keeps wrapping).
        // `block>>1` in 55..=63 (i.e. block 110..=127) => `512 << n` is 0, so
        // every string gets its own block and `remaining` stays 0. Five
        // allocations keep `block` inside that window for start <= 122.
        for &start in &[110u8, 112, 118, 122] {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            ca.block = start;
            ra.block = start;
            for i in 0..5usize {
                let mut s = vec![b'q'; 1 + i];
                s.push(0);
                let cp = (l.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
                assert_eq!(ca.remaining, ra.remaining, "start={start} i={i}");
                assert_eq!(ca.block, ra.block, "start={start} i={i}");
                assert_eq!(ca.remaining, 0, "blocksize 0 => dedicated block");
                assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            }
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
        }
        // `block == 127` bumps to 128, where `block>>1 == 64` is C UB. x86-64
        // `shlq` masks the count to 6 bits, so the blocksize jumps back to 512
        // and the arena starts carving again. Both libraries must do this.
        for &start in &[127u8, 254, 255] {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            ca.block = start;
            ra.block = start;
            let mut states_c = Vec::new();
            let mut states_r = Vec::new();
            for i in 0..6usize {
                let mut s = vec![b'q'; 1 + i];
                s.push(0);
                let cp = (l.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
                states_c.push((ca.remaining, ca.block));
                states_r.push((ra.remaining, ra.block));
                assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            }
            assert_eq!(states_c, states_r, "shift-mask walk from block={start}");
            assert_eq!(states_c[0], (0, start.wrapping_add(1)), "first call: blocksize 0");
            assert!(
                states_c[1..].iter().any(|&(rem, _)| rem != 0),
                "after the wrap to block>>1 == 64 the masked shift must give a \
                 non-zero blocksize again (states={states_c:?})"
            );
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
        }
    }
}

// ===========================================================================
// rows 42 / 43 / 44 — strkey / str_put edge inputs
// ===========================================================================
#[test]
fn err_42_43_44_strkey_and_str_put_edges() {
    let l = libs();
    unsafe {
        // row 42: extremes never overflow the 256-byte static buffer
        for n in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            let cp = (l.c.strkey)(n);
            let rp = (l.r.strkey)(n);
            let cs = CStr::from_ptr(cp).to_bytes().to_vec();
            let rs = CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(cs, rs, "strkey({n})");
            assert_eq!(cs, format!("test_{n}").into_bytes());
            assert!(cs.len() < 256);
        }
    }
    // rows 43 / 44: non-positive `num` skips the stralloc loop entirely
    for num in [0i32, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        let cout = capture_stdout(|| unsafe {
            (l.c.rand_seed)(0x3141_5926);
            (l.c.str_put)(num);
        });
        let rout = capture_stdout(|| unsafe {
            (l.r.rand_seed)(0x3141_5926);
            (l.r.str_put)(num);
        });
        assert_eq!(
            cout,
            rout,
            "str_put({num})\nC   : {:?}\nRUST: {:?}",
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
        // the three live asserts at lib.c:958-960 must hold => normal output
        assert_eq!(cout, format!("a {num}\n").into_bytes());
    }
}

// ===========================================================================
// row 46 — generic zero / degenerate sizes
// ===========================================================================
#[test]
fn err_46_zero_sizes() {
    let l = libs();
    // elemsize == 0: every element aliases the same address
    unsafe {
        (l.c.rand_seed)(0x3141_5926);
        (l.r.rand_seed)(0x3141_5926);
        let d = ElemDesc {
            elemsize: 0,
            raw: vec![],
            cstr: vec![],
        };
        let mut ct: *mut u8 = std::ptr::null_mut();
        let mut rt: *mut u8 = std::ptr::null_mut();
        for i in 0..10i32 {
            let k = i32k(i);
            ct = (l.c.hmput_key)(ct as *mut c_void, 0, k.as_ptr() as *mut c_void, 0, 0) as *mut u8;
            rt = (l.r.hmput_key)(rt as *mut c_void, 0, k.as_ptr() as *mut c_void, 0, 0) as *mut u8;
            assert_eq!(temp_of(ct, 0), temp_of(rt, 0), "elemsize 0, i={i}");
            assert_eq!(
                snapshot_map(ct, &d),
                snapshot_map(rt, &d),
                "elemsize 0, i={i}"
            );
        }
        (l.c.hmfree_func)(ct as *mut c_void, 0);
        (l.r.hmfree_func)(rt as *mut c_void, 0);
    }
    // keysize == 0 on a normal map: all keys compare equal
    {
        let mut m = MapPair::new(int_map(), 8, "keysize 0");
        m.seed(0x3141_5926);
        for i in 0..25i32 {
            let k = i32k(i);
            unsafe {
                let l2 = libs();
                m.ct = (l2.c.hmput_key)(m.ct as *mut c_void, 8, k.as_ptr() as *mut c_void, 0, 0)
                    as *mut u8;
                m.rt = (l2.r.hmput_key)(m.rt as *mut c_void, 8, k.as_ptr() as *mut c_void, 0, 0)
                    as *mut u8;
                let ci = temp_of(m.ct, 8);
                assert_eq!(ci, temp_of(m.rt, 8));
                // write the whole element so the snapshot is well defined
                std::ptr::write_bytes(m.ct.add(8 * ci as usize), i as u8, 8);
                std::ptr::write_bytes(m.rt.add(8 * ci as usize), i as u8, 8);
            }
            m.check("keysize 0 put");
        }
        assert_eq!(unsafe { hmlen(m.ct, 8) }, 1);
        m.free();
    }
    // min_cap == 0 / addlen == 0 on arrgrowf is row 1; huge elemsize is row 3.
}
