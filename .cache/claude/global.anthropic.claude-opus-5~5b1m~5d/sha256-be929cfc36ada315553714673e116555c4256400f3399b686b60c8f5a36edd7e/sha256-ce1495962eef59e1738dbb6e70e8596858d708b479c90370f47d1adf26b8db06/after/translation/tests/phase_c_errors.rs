//! Phase C: one differential test per row of `ERRORS.md`.
//!
//! Rows whose C behaviour is `assert()`+`SIGABRT` or `SIGSEGV` are run in a
//! re-executed child process so the *signal* and the *stderr diagnostic* can be
//! compared between the two libraries.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

fn k32(v: i32) -> [u8; 4] {
    v.to_ne_bytes()
}

fn mk_elem(key: &[u8], elemsize: usize, fill: &[u8]) -> Vec<u8> {
    let mut e = vec![0u8; elemsize];
    e[..key.len()].copy_from_slice(key);
    let n = (elemsize - key.len()).min(fill.len());
    e[key.len()..key.len() + n].copy_from_slice(&fill[..n]);
    e
}

// ===========================================================================
// E1 / E2 — stbds_arrgrowf sentinel returns
// ===========================================================================

#[test]
fn err_e1_arrgrowf_nogrow() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[1usize, 4, 8, 16, 64] {
            let mut a = ArrPair::new(c, r, es);
            a.grow(0, 4);
            let (pc, pr) = (a.c, a.r);
            for &(addlen, mc) in &[(0usize, 0usize), (0, 1), (0, 4), (4, 0), (4, 4), (1, 3)] {
                let (same_c, same_r) = a.grow(addlen, mc);
                assert!(
                    same_c,
                    "E1 C: expected unchanged pointer for es={es} addlen={addlen} mc={mc}"
                );
                assert!(
                    same_r,
                    "E1 RUST: expected unchanged pointer for es={es} addlen={addlen} mc={mc}"
                );
                a.assert_same(&format!("E1 es={es} addlen={addlen} mc={mc}"));
            }
            assert_eq!((a.c, a.r), (pc, pr));
            a.free();
        }
    });
}

#[test]
fn err_e2_arrgrowf_null_zero() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[0usize, 1, 4, 8, 16, 64, 4096] {
            let pc = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let pr = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(pc.is_null(), "E2 C es={es}: expected NULL, got {pc:?}");
            assert!(pr.is_null(), "E2 RUST es={es}: expected NULL, got {pr:?}");
        }
    });
}

// ===========================================================================
// E4 / E5 — stbds_hmfree_func
// ===========================================================================

#[test]
fn err_e4_hmfree_null() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[0usize, 1, 8, 16, 1024] {
            (c.hmfree_func)(std::ptr::null_mut(), es);
            (r.hmfree_func)(std::ptr::null_mut(), es);
        }
    });
}

#[test]
fn err_e5_hmfree_no_table() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[1usize, 8, 16, 64] {
            // array created by arrgrowf: header->hash_table == NULL
            let pc = (c.arrgrowf)(std::ptr::null_mut(), es, 4, 0);
            let pr = (r.arrgrowf)(std::ptr::null_mut(), es, 4, 0);
            assert!((*(pc as *mut u8).wrapping_sub(HDR_SIZE).cast::<ArrHeader>())
                .hash_table
                .is_null());
            assert!((*(pr as *mut u8).wrapping_sub(HDR_SIZE).cast::<ArrHeader>())
                .hash_table
                .is_null());
            (c.hmfree_func)(pc, es);
            (r.hmfree_func)(pr, es);
        }
    });
}

// ===========================================================================
// E6 / E7 / E8 / E9 / E46 — get sentinels
// ===========================================================================

#[test]
fn err_e6_hmget_ts_null() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &(es, ks) in &[(8usize, 4usize), (16, 8), (64, 16)] {
            for &mode in &[HM_BINARY, HM_STRING, 2, -1] {
                let mut key = vec![b'k'; ks.max(1)];
                key[ks.max(1) - 1] = 0;
                let mut tc: isize = 0x1234;
                let mut tr: isize = 0x1234;
                let pc = (c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    es,
                    key.as_mut_ptr() as *mut c_void,
                    ks,
                    &mut tc,
                    mode,
                );
                let pr = (r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    es,
                    key.as_mut_ptr() as *mut c_void,
                    ks,
                    &mut tr,
                    mode,
                );
                assert_eq!(tc, -1, "E6 C es={es} mode={mode}");
                assert_eq!(tr, -1, "E6 RUST es={es} mode={mode}");
                assert!(!pc.is_null() && !pr.is_null(), "E6 allocated a fresh map");
                // fresh map: length 1, element zeroed, no hash table
                let hc = *(pc as *mut u8).wrapping_sub(es + HDR_SIZE).cast::<ArrHeader>();
                let hr = *(pr as *mut u8).wrapping_sub(es + HDR_SIZE).cast::<ArrHeader>();
                assert_eq!(hc.length, 1);
                assert_eq!(hr.length, 1);
                assert_eq!(hc.capacity, hr.capacity);
                assert!(hc.hash_table.is_null() && hr.hash_table.is_null());
                let ec = std::slice::from_raw_parts((pc as *const u8).wrapping_sub(es), es);
                let er = std::slice::from_raw_parts((pr as *const u8).wrapping_sub(es), es);
                assert!(ec.iter().all(|&b| b == 0), "E6 C default element zeroed");
                assert_eq!(ec, er, "E6 default element");
                (c.hmfree_func)((pc as *mut u8).wrapping_sub(es) as *mut c_void, es);
                (r.hmfree_func)((pr as *mut u8).wrapping_sub(es) as *mut c_void, es);
            }
        }
    });
}

#[test]
fn err_e7_hmget_ts_no_table() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        p.put_default("E7 hmput_default");
        assert!(p.c.table().is_null(), "E7 no hash table yet");
        assert!(p.r.table().is_null(), "E7 no hash table yet");
        for k in 0i32..5 {
            let got = p.geti_ts(&k32(k), HM_BINARY, &format!("E7 ts {k}"));
            assert_eq!(got, -1, "E7 *temp must be -1 (table == 0)");
        }
        p.free("E7 free");
    });
}

#[test]
fn err_e8_hmget_ts_missing() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(8);
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..40 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("E8 put {k}"));
        }
        for _ in 0..500 {
            let k = 100_000 + rng.i32().rem_euclid(1_000_000);
            let got = p.geti_ts(&k32(k), HM_BINARY, &format!("E8 ts missing {k}"));
            assert_eq!(got, -1, "E8 missing key {k}");
        }
        p.free("E8 free");
    });
}

#[test]
fn err_e9_hmget_key_missing() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        // (a) a == NULL
        for &es in &[8usize, 16] {
            let mut key = k32(1);
            let pc = (c.hmget_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                HM_BINARY,
            );
            let pr = (r.hmget_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                HM_BINARY,
            );
            let hc = *(pc as *mut u8).wrapping_sub(es + HDR_SIZE).cast::<ArrHeader>();
            let hr = *(pr as *mut u8).wrapping_sub(es + HDR_SIZE).cast::<ArrHeader>();
            assert_eq!(hc.temp, -1, "E9a C header->temp");
            assert_eq!(hr.temp, -1, "E9a RUST header->temp");
            (c.hmfree_func)((pc as *mut u8).wrapping_sub(es) as *mut c_void, es);
            (r.hmfree_func)((pr as *mut u8).wrapping_sub(es) as *mut c_void, es);
        }
        // (b) table == 0  and  (c) key absent
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        p.put_default("E9 default");
        for k in 0i32..3 {
            assert_eq!(
                p.geti(&k32(k), HM_BINARY, &format!("E9b get {k}")),
                -1,
                "E9b table==0"
            );
        }
        for k in 0i32..20 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("E9 put {k}"));
        }
        for k in 1000i32..1050 {
            assert_eq!(
                p.geti(&k32(k), HM_BINARY, &format!("E9c get {k}")),
                -1,
                "E9c absent"
            );
        }
        p.free("E9 free");
    });
}

#[test]
fn err_e46_hmget_ts_leaves_header_temp() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..20 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("E46 put {k}"));
        }
        // header->temp currently holds the last inserted index
        let before_c = p.c.temp();
        let before_r = p.r.temp();
        assert_eq!(before_c, before_r);
        // a _ts lookup of a *missing* key must not touch header->temp
        let mut key = k32(999_999);
        let mut tc: isize = 7;
        let mut tr: isize = 7;
        p.c.t = (c.hmget_key_ts)(
            p.c.t as *mut c_void,
            8,
            key.as_mut_ptr() as *mut c_void,
            4,
            &mut tc,
            HM_BINARY,
        ) as *mut u8;
        p.r.t = (r.hmget_key_ts)(
            p.r.t as *mut c_void,
            8,
            key.as_mut_ptr() as *mut c_void,
            4,
            &mut tr,
            HM_BINARY,
        ) as *mut u8;
        assert_eq!(tc, -1);
        assert_eq!(tr, -1);
        assert_eq!(p.c.temp(), before_c, "E46 C header->temp untouched");
        assert_eq!(p.r.temp(), before_r, "E46 RUST header->temp untouched");
        p.assert_same("E46");
        p.free("E46 free");
    });
}

// ===========================================================================
// E10 — keysize == 0
// ===========================================================================

#[test]
fn err_e10_keysize_zero() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[1usize, 4, 8, 16] {
            let mut rng = Rng::new(10 + es as u64);
            let mut p = Pair::new(c, r, Shape::binary(es, 0));
            for i in 0..15 {
                let e = rng.bytes(es);
                let idx = p.put_struct(&[], &e, HM_BINARY, &format!("E10 es={es} put {i}"));
                assert_eq!(idx, 0, "E10 es={es}: keysize 0 collapses to one entry");
            }
            assert_eq!(p.c.hmlen(), 1);
            assert_eq!(p.geti(&[], HM_BINARY, "E10 get"), 0);
            assert_eq!(p.del(&[], HM_BINARY, "E10 del"), 1);
            assert_eq!(p.del(&[], HM_BINARY, "E10 del again"), 0);
            p.free(&format!("E10 es={es} free"));
        }
    });
}

// ===========================================================================
// E11 / E12 / E13 / E14 / E15 — hmdel_key sentinels
// ===========================================================================

#[test]
fn err_e11_hmdel_null() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[1usize, 8, 16] {
            for &ks in &[0usize, 4, 8] {
                for &ko in &[0usize, 4] {
                    for &mode in &[HM_BINARY, HM_STRING, 2, -1, c_int::MAX, c_int::MIN] {
                        let mut key = vec![0u8; 16];
                        let pc = (c.hmdel_key)(
                            std::ptr::null_mut(),
                            es,
                            key.as_mut_ptr() as *mut c_void,
                            ks,
                            ko,
                            mode,
                        );
                        let pr = (r.hmdel_key)(
                            std::ptr::null_mut(),
                            es,
                            key.as_mut_ptr() as *mut c_void,
                            ks,
                            ko,
                            mode,
                        );
                        assert!(pc.is_null(), "E11 C es={es} mode={mode}");
                        assert!(pr.is_null(), "E11 RUST es={es} mode={mode}");
                    }
                }
            }
        }
    });
}

#[test]
fn err_e12_hmdel_no_table() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        p.put_default("E12 default");
        assert!(p.c.table().is_null());
        let before = p.c.snapshot();
        for k in 0i32..5 {
            let d = p.del(&k32(k), HM_BINARY, &format!("E12 del {k}"));
            assert_eq!(d, 0, "E12 temp must be 0");
        }
        let after = p.c.snapshot();
        assert_eq!(before.length, after.length, "E12 length unchanged");
        p.free("E12 free");
    });
}

#[test]
fn err_e13_hmdel_missing() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..30 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("E13 put {k}"));
        }
        let before = p.c.snapshot();
        for k in 5000i32..5050 {
            let d = p.del(&k32(k), HM_BINARY, &format!("E13 del missing {k}"));
            assert_eq!(d, 0, "E13 missing key → temp 0");
        }
        let mut after = p.c.snapshot();
        after.temp = before.temp; // temp is legitimately clobbered to 0
        let mut b = before;
        b.temp = after.temp;
        assert_eq!(b, after, "E13 nothing else changed");
        p.free("E13 free");
    });
}

#[test]
fn err_e14_hmdel_twice() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..12 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("E14 put {k}"));
        }
        for k in 0i32..12 {
            assert_eq!(p.del(&k32(k), HM_BINARY, &format!("E14 del1 {k}")), 1);
            let len = p.c.hmlen();
            assert_eq!(p.del(&k32(k), HM_BINARY, &format!("E14 del2 {k}")), 0);
            assert_eq!(p.c.hmlen(), len, "E14 second delete must not shrink");
            assert_eq!(p.del(&k32(k), HM_BINARY, &format!("E14 del3 {k}")), 0);
        }
        p.free("E14 free");
    });
}

#[test]
fn err_e15_hmdel_last() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &n in &[1usize, 2, 3, 8] {
            let mut p = Pair::new(c, r, Shape::binary(8, 4));
            for k in 0..n as i32 {
                let key = k32(k);
                let e = mk_elem(&key, 8, &k32(k));
                p.put_struct(&key, &e, HM_BINARY, &format!("E15 n={n} put {k}"));
            }
            // deleting in reverse insertion order always hits old_index == final_index
            for k in (0..n as i32).rev() {
                assert_eq!(
                    p.del(&k32(k), HM_BINARY, &format!("E15 n={n} del {k}")),
                    1
                );
            }
            assert_eq!(p.c.hmlen(), 0);
            p.free(&format!("E15 n={n} free"));
        }
    });
}

// ===========================================================================
// E17 / E18 — out-of-range `mode` (C enums / ints accept any value)
// ===========================================================================

#[test]
fn err_e17_negative_mode() {
    let modes: &[c_int] = &[-1, -2, -7, -1000, c_int::MIN, c_int::MIN + 1];
    let mut baseline: Option<MapSnap> = None;
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        // establish mode-0 baseline first, then every negative mode
        for &mode in std::iter::once(&0).chain(modes.iter()) {
            (c.rand_seed)(DEFAULT_SEED);
            (r.rand_seed)(DEFAULT_SEED);
            let mut p = Pair::new(c, r, Shape::binary(16, 8));
            for k in 0u64..25 {
                let key = k.to_ne_bytes();
                let e = mk_elem(&key, 16, &(k * 3).to_ne_bytes());
                p.put_struct(&key, &e, mode, &format!("E17 mode={mode} put {k}"));
            }
            for k in 0u64..30 {
                p.geti(&k.to_ne_bytes(), mode, &format!("E17 mode={mode} get {k}"));
                p.geti_ts(&k.to_ne_bytes(), mode, &format!("E17 mode={mode} ts {k}"));
            }
            for k in (0u64..25).step_by(2) {
                p.del(&k.to_ne_bytes(), mode, &format!("E17 mode={mode} del {k}"));
            }
            let s = p.c.snapshot();
            match &baseline {
                None => baseline = Some(s),
                Some(b) => assert_eq!(*b, s, "E17 mode={mode} must equal mode=0"),
            }
            p.free(&format!("E17 mode={mode} free"));
        }
    });
}

#[test]
fn err_e18_large_mode() {
    let modes: &[c_int] = &[1, 2, 3, 7, 1000, 0x7fff, c_int::MAX, c_int::MAX - 1];
    let mut baseline: Option<MapSnap> = None;
    for &mode in modes {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(18);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..25u32 {
                let mut s = format!("key_{i}_").into_bytes();
                s.extend_from_slice(&rng.ascii(6));
                s.push(0);
                keys.push(s);
            }
            let mut p = Pair::new(c, r, Shape::string(16));
            for (i, k) in keys.iter_mut().enumerate() {
                let kp = k.as_mut_ptr() as *mut c_char;
                p.shput_value(
                    kp,
                    8,
                    &(i as u64).to_ne_bytes(),
                    mode,
                    &format!("E18 mode={mode} put {i}"),
                );
            }
            for (i, k) in keys.iter_mut().enumerate() {
                let kp = k.as_mut_ptr() as *mut c_char;
                let a = p.sgeti(kp, mode, &format!("E18 mode={mode} get {i}"));
                assert!(a >= 0, "E18 mode={mode} key {i} present");
            }
            let s = p.c.snapshot();
            match &baseline {
                None => baseline = Some(s),
                Some(b) => assert_eq!(
                    *b, s,
                    "E18 mode={mode} put/get must equal mode=1"
                ),
            }
            p.free(&format!("E18 mode={mode} free"));
        });
    }
}

// ===========================================================================
// E19 / E20 — stbds_shmode_func with out-of-range / SH_NONE modes
// ===========================================================================

#[test]
fn err_e19_shmode_out_of_range() {
    // `(unsigned char) mode` truncation
    let cases: &[(c_int, u8)] = &[
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (5, 5),
        (255, 255),
        (256, 0),
        (257, 1),
        (258, 2),
        (-1, 255),
        (-2, 254),
        (-256, 0),
        (c_int::MAX, 255),
        (c_int::MIN, 0),
    ];
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &(mode, want) in cases {
            let pc = (c.shmode_func)(16, mode);
            let pr = (r.shmode_func)(16, mode);
            let tc = (*(pc as *mut u8).wrapping_sub(16 + HDR_SIZE).cast::<ArrHeader>()).hash_table
                as *mut HashIndex;
            let tr = (*(pr as *mut u8).wrapping_sub(16 + HDR_SIZE).cast::<ArrHeader>()).hash_table
                as *mut HashIndex;
            assert_eq!((*tc).string.mode, want, "E19 C mode={mode}");
            assert_eq!((*tr).string.mode, want, "E19 RUST mode={mode}");
            assert_eq!((*tc).slot_count, (*tr).slot_count);
            assert_eq!((*tc).seed, (*tr).seed);
            (c.hmfree_func)((pc as *mut u8).wrapping_sub(16) as *mut c_void, 16);
            (r.hmfree_func)((pr as *mut u8).wrapping_sub(16) as *mut c_void, 16);
        }
    });

    // Any `string.mode` outside {1,2,3} takes the `switch` `default:` branch and
    // `memcpy`s `keysize` key bytes.  With *binary* puts that is fully defined,
    // so the whole lifecycle can be compared.
    for &mode in &[0 as c_int, 4, 5, 255, 256, -1, c_int::MAX, c_int::MIN] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(19u64.wrapping_add(mode as i64 as u64));
            let mut p = Pair::new(c, r, Shape::binary(16, 8));
            p.shmode(mode, &format!("E19b shmode={mode}"));
            let mut keys: Vec<[u8; 8]> = Vec::new();
            for _ in 0..30 {
                let b = rng.bytes(8);
                keys.push(b.try_into().unwrap());
            }
            for (i, k) in keys.iter().enumerate() {
                let e = mk_elem(k, 16, &(i as u64).to_ne_bytes());
                p.put_struct(k, &e, HM_BINARY, &format!("E19b m={mode} put {i}"));
            }
            for (i, k) in keys.iter().enumerate() {
                p.geti(k, HM_BINARY, &format!("E19b m={mode} get {i}"));
            }
            for (i, k) in keys.iter().enumerate().step_by(2) {
                p.del(k, HM_BINARY, &format!("E19b m={mode} del {i}"));
            }
            p.free(&format!("E19b m={mode} free"));
        });
    }
}

#[test]
fn err_e20_shmode_none_string_put() {
    // string.mode == SH_NONE with mode>=1 puts: `default:` memcpy of `keysize`
    // key *bytes*, not a `char *`.  Only fresh distinct inserts are defined
    // (any hash match would deref those bytes as a pointer).
    for &shm in &[0 as c_int, 4, 255] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(20u64.wrapping_add(shm as i64 as u64));
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            while keys.len() < 20 {
                let s = rng.ascii(12);
                if seen.insert(s.clone()) {
                    let mut v = s;
                    v.push(0);
                    v.extend_from_slice(&[0u8; 8]);
                    keys.push(v);
                }
            }
            let mut p = Pair::new(c, r, Shape::string(16).raw_keys());
            p.shmode(shm, &format!("E20 shm={shm}"));
            for (i, k) in keys.iter_mut().enumerate() {
                let kp = k.as_mut_ptr() as *mut c_char;
                p.shput_value(
                    kp,
                    8,
                    &(i as u64).to_ne_bytes(),
                    HM_STRING,
                    &format!("E20 shm={shm} put {i}"),
                );
            }
            // the key field must hold the first 8 *bytes of the string*
            let s = p.c.snapshot();
            assert_eq!(s.elems[1][..8], keys[0][..8], "E20 memcpy'd key bytes");
            p.free(&format!("E20 shm={shm} free"));
        });
    }
}

// ===========================================================================
// E21 / E22 / E23 — hmput_default
// ===========================================================================

#[test]
fn err_e21_hmput_default_null() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[1usize, 8, 16, 64] {
            let pc = (c.hmput_default)(std::ptr::null_mut(), es);
            let pr = (r.hmput_default)(std::ptr::null_mut(), es);
            let hc = *(pc as *mut u8).wrapping_sub(es + HDR_SIZE).cast::<ArrHeader>();
            let hr = *(pr as *mut u8).wrapping_sub(es + HDR_SIZE).cast::<ArrHeader>();
            assert_eq!(hc.length, 1, "E21 C es={es}");
            assert_eq!(hr.length, 1, "E21 RUST es={es}");
            assert_eq!(hc.capacity, hr.capacity);
            assert_eq!(hc.temp, hr.temp);
            assert!(hc.hash_table.is_null() && hr.hash_table.is_null());
            let ec = std::slice::from_raw_parts((pc as *const u8).wrapping_sub(es), es);
            let er = std::slice::from_raw_parts((pr as *const u8).wrapping_sub(es), es);
            assert!(ec.iter().all(|&b| b == 0));
            assert_eq!(ec, er);
            (c.hmfree_func)((pc as *mut u8).wrapping_sub(es) as *mut c_void, es);
            (r.hmfree_func)((pr as *mut u8).wrapping_sub(es) as *mut c_void, es);
        }
    });
}

#[test]
fn err_e22_hmput_default_len0() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[1usize, 8, 16] {
            for &cap in &[1usize, 4, 16] {
                // arrgrowf gives length == 0; hmput_default must bump it to 1
                let ac = (c.arrgrowf)(std::ptr::null_mut(), es, 0, cap);
                let ar = (r.arrgrowf)(std::ptr::null_mut(), es, 0, cap);
                // fill the buffer with garbage so the memset is observable
                std::ptr::write_bytes(ac as *mut u8, 0xAB, es);
                std::ptr::write_bytes(ar as *mut u8, 0xAB, es);
                let hc = (ac as *mut u8).wrapping_add(es) as *mut c_void;
                let hr = (ar as *mut u8).wrapping_add(es) as *mut c_void;
                let pc = (c.hmput_default)(hc, es);
                let pr = (r.hmput_default)(hr, es);
                let hdc = *(pc as *mut u8).wrapping_sub(es + HDR_SIZE).cast::<ArrHeader>();
                let hdr_ = *(pr as *mut u8).wrapping_sub(es + HDR_SIZE).cast::<ArrHeader>();
                assert_eq!(hdc.length, 1, "E22 C es={es} cap={cap}");
                assert_eq!(hdr_.length, 1, "E22 RUST es={es} cap={cap}");
                assert_eq!(hdc.capacity, hdr_.capacity);
                let ec = std::slice::from_raw_parts((pc as *const u8).wrapping_sub(es), es);
                let er = std::slice::from_raw_parts((pr as *const u8).wrapping_sub(es), es);
                assert!(ec.iter().all(|&b| b == 0), "E22 C element zeroed");
                assert_eq!(ec, er, "E22 element bytes");
                (c.hmfree_func)((pc as *mut u8).wrapping_sub(es) as *mut c_void, es);
                (r.hmfree_func)((pr as *mut u8).wrapping_sub(es) as *mut c_void, es);
            }
        }
    });
}

#[test]
fn err_e23_hmput_default_twice() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[8usize, 16] {
            let p1c = (c.hmput_default)(std::ptr::null_mut(), es);
            let p1r = (r.hmput_default)(std::ptr::null_mut(), es);
            std::ptr::write_bytes((p1c as *mut u8).wrapping_sub(es), 0x5A, es);
            std::ptr::write_bytes((p1r as *mut u8).wrapping_sub(es), 0x5A, es);
            let p2c = (c.hmput_default)(p1c, es);
            let p2r = (r.hmput_default)(p1r, es);
            assert_eq!(p1c, p2c, "E23 C: second call is a no-op");
            assert_eq!(p1r, p2r, "E23 RUST: second call is a no-op");
            let ec = std::slice::from_raw_parts((p2c as *const u8).wrapping_sub(es), es);
            let er = std::slice::from_raw_parts((p2r as *const u8).wrapping_sub(es), es);
            assert!(ec.iter().all(|&b| b == 0x5A), "E23 C not re-zeroed");
            assert_eq!(ec, er);
            (c.hmfree_func)((p2c as *mut u8).wrapping_sub(es) as *mut c_void, es);
            (r.hmfree_func)((p2r as *mut u8).wrapping_sub(es) as *mut c_void, es);
        }
    });
}

// ===========================================================================
// E24 / E25 / E26 / E27 — hashing edge cases
// ===========================================================================

#[test]
fn err_e24_hash_bytes_zero_len() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &seed in &[0usize, 1, 2, 0x3141_5926, usize::MAX] {
            let hc = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let hr = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(hc, hr, "E24 seed={seed:#x}");
            // must not depend on the (unread) pointer
            let mut buf = [0xFFu8; 8];
            let h2 = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(hc, h2, "E24 C len=0 must ignore p");
            let h3 = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(hr, h3, "E24 RUST len=0 must ignore p");
        }
    });
}

#[test]
fn err_e25_hash_bytes_tail() {
    with_libs(DEFAULT_SEED, |c, r| {
        let mut rng = Rng::new(25);
        // every (full-block count, tail residue) combination up to 5 blocks
        for blocks in 0usize..=5 {
            for tail in 0usize..8 {
                let len = blocks * 8 + tail;
                for _ in 0..64 {
                    let mut buf = rng.bytes(len + 8);
                    for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
                        let hc =
                            unsafe { (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                        let hr =
                            unsafe { (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                        assert_eq!(hc, hr, "E25 len={len} (blocks={blocks} tail={tail})");
                    }
                }
                // high-bit-only patterns in each tail position
                for pos in 0..len.min(8) {
                    let mut buf = vec![0u8; len + 8];
                    buf[pos] = 0x80;
                    for &seed in &[0usize, usize::MAX] {
                        let hc =
                            unsafe { (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                        let hr =
                            unsafe { (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                        assert_eq!(hc, hr, "E25 len={len} pos={pos} high bit");
                    }
                }
            }
        }
    });
}

/// E26 — the reserved hash values 0 (`HASH_EMPTY`) and 1 (`HASH_DELETED`).
/// A raw hash of 0/1 is astronomically unlikely (2^-63), so the `hash += 2`
/// clamp is not directly reachable; what *is* reachable and is verified here is
/// the reserved-value invariant that the clamp exists to protect:
/// `index >= 0 ⇒ hash >= 2`, `index == -2 ⇒ hash == 1`, `index == -1 ⇒ hash == 0`.
#[test]
fn err_e26_hash_lt_2_clamp() {
    for seed in 1u64..=4 {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(0x26 + seed);
            let mut p = Pair::new(c, r, Shape::binary(8, 4));
            for i in 0..1500 {
                let k = k32(rng.below(80) as i32);
                if rng.below(3) == 0 {
                    p.del(&k, HM_BINARY, &format!("E26 s={seed} i={i} del"));
                } else {
                    let e = mk_elem(&k, 8, &k32(rng.i32()));
                    p.put_struct(&k, &e, HM_BINARY, &format!("E26 s={seed} i={i} put"));
                }
                for (label, m) in [("C", &p.c), ("RUST", &p.r)] {
                    let s = m.snapshot();
                    if let Some(t) = s.table {
                        for (slot, (&h, &idx)) in
                            t.hashes.iter().zip(t.indices.iter()).enumerate()
                        {
                            match idx {
                                -1 => assert_eq!(
                                    h, 0,
                                    "E26 {label} empty slot {slot} must have hash 0"
                                ),
                                -2 => assert_eq!(
                                    h, 1,
                                    "E26 {label} tombstone {slot} must have hash 1"
                                ),
                                _ => assert!(
                                    h >= 2,
                                    "E26 {label} live slot {slot} hash {h} must be >= 2"
                                ),
                            }
                        }
                    }
                }
            }
            p.free(&format!("E26 s={seed} free"));
        });
    }
}

#[test]
fn err_e27_hash_string_empty() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &seed in &[0usize, 1, 2, 0x3141_5926, usize::MAX, usize::MAX - 1] {
            let mut e = [0u8; 1];
            let hc = (c.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            let hr = (r.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(hc, hr, "E27 seed={seed:#x}");
        }
        // an empty string as a map key
        let mut p = Pair::new(c, r, Shape::string(16));
        p.shmode(SH_STRDUP, "E27 shmode");
        let mut empty = vec![0u8; 9];
        let kp = empty.as_mut_ptr() as *mut c_char;
        p.shput_value(kp, 8, &1u64.to_ne_bytes(), HM_STRING, "E27 put empty");
        assert!(p.sgeti(kp, HM_STRING, "E27 get empty") >= 0);
        assert_eq!(p.sdel(kp, HM_STRING, "E27 del empty"), 1);
        p.free("E27 free");
    });
}

// ===========================================================================
// E29 / E30 / E31 / E32 / E33 / E34 — string arena
// ===========================================================================

struct Arena2 {
    ca: Box<StringArena>,
    ra: Box<StringArena>,
}

impl Arena2 {
    fn new() -> Arena2 {
        Arena2 {
            ca: Box::new(StringArena::zeroed()),
            ra: Box::new(StringArena::zeroed()),
        }
    }
    unsafe fn alloc(&mut self, c: &Api, r: &Api, s: &[u8], ctx: &str) {
        let mut cs = s.to_vec();
        cs.push(0);
        let mut rs = s.to_vec();
        rs.push(0);
        let pc = (c.stralloc)(
            &mut *self.ca as *mut StringArena as *mut c_void,
            cs.as_mut_ptr() as *mut c_char,
        );
        let pr = (r.stralloc)(
            &mut *self.ra as *mut StringArena as *mut c_void,
            rs.as_mut_ptr() as *mut c_char,
        );
        assert_eq!(read_cstr(pc), s.to_vec(), "{ctx}: C string");
        assert_eq!(read_cstr(pr), s.to_vec(), "{ctx}: RUST string");
        assert_eq!(self.ca.remaining, self.ra.remaining, "{ctx}: remaining");
        assert_eq!(self.ca.block, self.ra.block, "{ctx}: block");
        assert_eq!(
            self.ca.storage.is_null(),
            self.ra.storage.is_null(),
            "{ctx}: storage null-ness"
        );
    }
    unsafe fn reset(&mut self, c: &Api, r: &Api) {
        (c.strreset)(&mut *self.ca as *mut StringArena as *mut c_void);
        (r.strreset)(&mut *self.ra as *mut StringArena as *mut c_void);
    }
}

#[test]
fn err_e29_stralloc_huge() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &len in &[512usize, 513, 1024, 5000, 200_000] {
            // storage == NULL
            let mut a = Arena2::new();
            a.alloc(c, r, &vec![b'H'; len], &format!("E29 first len={len}"));
            assert_eq!(a.ca.remaining, 0, "E29 C remaining=0 len={len}");
            assert_eq!(a.ra.remaining, 0, "E29 RUST remaining=0 len={len}");
            a.reset(c, r);
            // storage != NULL
            let mut a = Arena2::new();
            a.alloc(c, r, b"seed", &format!("E29 seed len={len}"));
            let rem_c = a.ca.remaining;
            let rem_r = a.ra.remaining;
            a.alloc(c, r, &vec![b'G'; len], &format!("E29 huge len={len}"));
            assert_eq!(a.ca.remaining, rem_c, "E29 C remaining untouched len={len}");
            assert_eq!(a.ra.remaining, rem_r, "E29 RUST remaining untouched len={len}");
            a.reset(c, r);
        }
    });
}

#[test]
fn err_e30_stralloc_block_cap() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut a = Arena2::new();
        for i in 0..40 {
            let rem = a.ca.remaining;
            a.alloc(c, r, &vec![b'f'; rem.max(1) - 1], &format!("E30 fill {i}"));
            a.alloc(c, r, b"s", &format!("E30 spill {i}"));
            assert!(a.ca.block <= 22, "E30 block must saturate, got {}", a.ca.block);
        }
        assert_eq!(a.ca.block, 22, "E30 C saturated block");
        assert_eq!(a.ra.block, 22, "E30 RUST saturated block");
        a.reset(c, r);
    });
}

/// E31 — `a->block` outside the range the library itself produces makes the
/// `(size_t)512u << (block>>1)` shift exceed / wrap the word width.
#[test]
fn err_e31_stralloc_block_shift_overflow() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        // shift 55..63 → blocksize wraps to 0 (well-defined for unsigned)
        let mut blocks: Vec<u8> = (110u8..=127).collect();
        // shift >= 64 → the C shift is UB; on x86-64 both compilers emit a
        // masking `shl`, which is what the Rust `wrapping_shl` reproduces.
        blocks.extend(128u8..=153);
        blocks.extend(238u8..=255);
        for b in blocks {
            for &len in &[0usize, 1, 5, 600] {
                let mut a = Arena2::new();
                a.ca.block = b;
                a.ra.block = b;
                a.alloc(c, r, &vec![b'p'; len], &format!("E31 block={b} len={len}"));
                a.alloc(c, r, &vec![b'q'; len], &format!("E31 block={b} len={len} 2nd"));
                a.reset(c, r);
            }
        }
    });
}

#[test]
fn err_e32_stralloc_empty_string() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut a = Arena2::new();
        for i in 0..1200 {
            a.alloc(c, r, b"", &format!("E32 empty {i}"));
        }
        a.reset(c, r);
        // empty string with a pre-set block
        for b in [0u8, 1, 10, 21, 22, 110, 128] {
            let mut a = Arena2::new();
            a.ca.block = b;
            a.ra.block = b;
            a.alloc(c, r, b"", &format!("E32 empty block={b}"));
            a.reset(c, r);
        }
    });
}

#[test]
fn err_e33_strreset_empty() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut a = Arena2::new();
        // garbage in the non-pointer fields must be zeroed by the memset
        a.ca.remaining = 12345;
        a.ra.remaining = 12345;
        a.ca.block = 9;
        a.ra.block = 9;
        a.ca.mode = 3;
        a.ra.mode = 3;
        a.reset(c, r);
        assert_eq!(a.ca.remaining, 0);
        assert_eq!(a.ra.remaining, 0);
        assert_eq!(a.ca.block, 0);
        assert_eq!(a.ra.block, 0);
        assert_eq!(a.ca.mode, 0);
        assert_eq!(a.ra.mode, 0);
        assert!(a.ca.storage.is_null() && a.ra.storage.is_null());
    });
}

#[test]
fn err_e34_strreset_twice() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut a = Arena2::new();
        for i in 0..10 {
            a.alloc(c, r, &vec![b'x'; 400], &format!("E34 fill {i}"));
        }
        a.reset(c, r);
        a.reset(c, r);
        a.reset(c, r);
        assert_eq!(a.ca.remaining, 0);
        assert_eq!(a.ra.remaining, 0);
        assert!(a.ca.storage.is_null() && a.ra.storage.is_null());
        // usable again
        a.alloc(c, r, b"again", "E34 reuse");
        a.reset(c, r);
    });
}

// ===========================================================================
// E36 / E37 — the hmput_key / hmdel_key asserts (must never fire)
// ===========================================================================

#[test]
fn err_e36_hmput_grow_boundary() {
    // exercise `i+1 == arrcap` and `i+1 == arrcap+1` for every capacity the
    // doubling schedule produces
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[8usize, 16, 24] {
            let mut p = Pair::new(c, r, Shape::binary(es, 4));
            for k in 0i32..300 {
                let key = k32(k);
                let e = mk_elem(&key, es, &k32(k));
                p.put_struct(&key, &e, HM_BINARY, &format!("E36 es={es} put {k}"));
                let s = p.c.snapshot();
                assert!(s.length <= s.capacity, "E36 length <= capacity invariant");
            }
            p.free(&format!("E36 es={es} free"));
        }
    });
}

#[test]
fn err_e37_hmdel_assert_free() {
    // exhaustive random delete/insert streams: any divergence in the
    // memmove + re-find bookkeeping would trip assert 846 or 849 in one of the
    // two libraries, and the deep snapshot compare catches the rest.
    for seed in 1u64..=6 {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(0x37 + seed);
            let mut p = Pair::new(c, r, Shape::binary(16, 8));
            for i in 0..1200 {
                let k = (rng.below(45) as u64).to_ne_bytes();
                match rng.below(2) {
                    0 => {
                        let e = mk_elem(&k, 16, &rng.next_u64().to_ne_bytes());
                        p.put_struct(&k, &e, HM_BINARY, &format!("E37 s={seed} i={i} put"));
                    }
                    _ => {
                        p.del(&k, HM_BINARY, &format!("E37 s={seed} i={i} del"));
                    }
                }
            }
            p.free(&format!("E37 s={seed} free"));
        });
    }
}

// ===========================================================================
// E40 / E41 / E42 — intput / strkey
// ===========================================================================

#[test]
fn err_e40_intput_ok() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for n in [
            0i32,
            1,
            -1,
            8,
            10,
            12,
            -9,
            -11,
            i32::MIN,
            i32::MAX,
            i32::MIN + 1,
            i32::MAX - 1,
        ] {
            (c.intput)(n as c_int);
            (r.intput)(n as c_int);
        }
    });
}

#[test]
fn err_e41_strkey_int_min() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for n in [i32::MIN, i32::MIN + 1, i32::MAX, -1, 0] {
            let sc = read_cstr((c.strkey)(n));
            let sr = read_cstr((r.strkey)(n));
            let want = format!("test_{n}").into_bytes();
            assert_eq!(sc, want, "E41 C strkey({n})");
            assert_eq!(sr, want, "E41 RUST strkey({n})");
        }
    });
}

#[test]
fn err_e42_strkey_static_reuse() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let a = (c.strkey)(i32::MIN);
        let b = (c.strkey)(0);
        assert_eq!(a, b, "E42 C same static buffer");
        assert_eq!(read_cstr(a), b"test_0".to_vec());
        let a = (r.strkey)(i32::MIN);
        let b = (r.strkey)(0);
        assert_eq!(a, b, "E42 RUST same static buffer");
        assert_eq!(read_cstr(a), b"test_0".to_vec());
    });
}

// ===========================================================================
// E44 / E45
// ===========================================================================

#[test]
fn err_e44_shput_same_key() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut key = b"the_same_key\0\0\0\0\0\0\0\0\0".to_vec();
            let kp = key.as_mut_ptr() as *mut c_char;
            let mut p = Pair::new(c, r, Shape::string(16));
            p.shmode(sh, &format!("E44 sh={sh} shmode"));
            let i0 = p.shput_value(kp, 8, &1u64.to_ne_bytes(), HM_STRING, "E44 put 1");
            p.assert_temp_key(b"the_same_key", "E44 temp_key 1");
            for n in 2u64..20 {
                let i = p.shput_value(
                    kp,
                    8,
                    &n.to_ne_bytes(),
                    HM_STRING,
                    &format!("E44 sh={sh} put {n}"),
                );
                assert_eq!(i, i0, "E44 sh={sh}: same slot reused");
                p.assert_temp_key(b"the_same_key", &format!("E44 sh={sh} temp_key {n}"));
            }
            assert_eq!(p.c.hmlen(), 1, "E44 sh={sh}: still one entry");
            p.free(&format!("E44 sh={sh} free"));
        });
    }
}

#[test]
fn err_e45_keyoffset_nonzero() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        // hmput_key/hmget_key hardcode keyoffset = 0; only hmdel_key takes one.
        for &(es, ko) in &[(16usize, 0usize), (16, 4), (16, 8), (16, 12), (32, 20)] {
            let mut rng = Rng::new(45 + (es * 64 + ko) as u64);
            let mut p = Pair::new(c, r, Shape::binary(es, 4).with_keyoffset(ko));
            for k in 0i32..25 {
                let key = k32(k);
                let filler = rng.bytes(es);
                let e = mk_elem(&key, es, &filler);
                p.put_struct(&key, &e, HM_BINARY, &format!("E45 es={es} ko={ko} put {k}"));
            }
            let mut deleted = 0;
            for k in 0i32..25 {
                let d = p.del(&k32(k), HM_BINARY, &format!("E45 es={es} ko={ko} del {k}"));
                deleted += d;
                if ko == 0 {
                    assert_eq!(d, 1, "E45 ko=0 must find the key");
                }
            }
            // whatever the outcome, both libraries must agree (checked inside del)
            eprintln!("E45 es={es} ko={ko}: {deleted} deletes succeeded");
            p.free(&format!("E45 es={es} ko={ko} free"));
        }
    });
}

// ===========================================================================
// Subprocess scenarios: E16, E28, E35, E38, E39, E43
// ===========================================================================

mod child {
    use std::process::Command;

    pub struct Outcome {
        pub code: Option<i32>,
        pub signal: Option<i32>,
        pub assertion: Option<String>,
        pub stderr: String,
    }

    /// Extract the glibc assertion diagnostic, stripped of the leading program
    /// name (which is the same for both children anyway).
    fn assertion_line(stderr: &str) -> Option<String> {
        stderr
            .lines()
            .find(|l| l.contains("Assertion") && l.contains("failed"))
            .map(|l| match l.find(": ") {
                Some(i) => l[i + 2..].to_string(),
                None => l.to_string(),
            })
    }

    pub fn run(scenario: &str, which: &str) -> Outcome {
        use std::os::unix::process::ExitStatusExt;
        let exe = std::env::current_exe().expect("current_exe");
        let out = Command::new(&exe)
            .args([
                "--exact",
                "phase_c_child_scenario",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("DS_SCEN", scenario)
            .env("DS_WHICH", which)
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        Outcome {
            code: out.status.code(),
            signal: out.status.signal(),
            assertion: assertion_line(&stderr),
            stderr,
        }
    }

    /// Run the scenario against both libraries and require identical outcomes.
    pub fn assert_same(scenario: &str) -> Outcome {
        let c = run(scenario, "c");
        let r = run(scenario, "rust");
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "{scenario}: exit status differs\n C   : code={:?} signal={:?}\n{}\n RUST: code={:?} signal={:?}\n{}",
            c.code,
            c.signal,
            c.stderr,
            r.code,
            r.signal,
            r.stderr
        );
        assert_eq!(
            c.assertion, r.assertion,
            "{scenario}: assertion diagnostic differs\n C   : {:?}\n RUST: {:?}",
            c.assertion, r.assertion
        );
        c
    }
}

/// The child body.  Returns immediately (a trivially passing test) unless the
/// parent asked for a specific crash scenario.
#[test]
fn phase_c_child_scenario() {
    let scen = match std::env::var("DS_SCEN") {
        Ok(s) => s,
        Err(_) => return,
    };
    let which = std::env::var("DS_WHICH").unwrap_or_else(|_| "c".into());
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let api: &Api = if which == "c" { c } else { r };
        let (name, arg) = match scen.split_once(':') {
            Some((n, a)) => (n, a),
            None => (scen.as_str(), ""),
        };
        match name {
            // E38 / E39
            "intput" => {
                let n: i32 = arg.parse().unwrap();
                (api.intput)(n as c_int);
                eprintln!("child: intput({n}) returned normally");
            }
            // E28
            "hash_string_null" => {
                let h = (api.hash_string)(std::ptr::null_mut(), 0x1234);
                eprintln!("child: hash_string(NULL) = {h}");
            }
            // E47 — hash_bytes(NULL, len > 0)
            "hash_bytes_null" => {
                let n: usize = arg.parse().unwrap();
                let h = (api.hash_bytes)(std::ptr::null_mut(), n, 0x1234);
                eprintln!("child: hash_bytes(NULL, {n}) = {h}");
            }
            // E48 — stralloc(NULL, str)
            "stralloc_null_arena" => {
                let mut s = b"abc\0".to_vec();
                let p = (api.stralloc)(std::ptr::null_mut(), s.as_mut_ptr() as *mut c_char);
                eprintln!("child: stralloc(NULL, \"abc\") = {p:?}");
            }
            // E49 — stralloc(arena, NULL)
            "stralloc_null_str" => {
                let mut arena = Box::new(StringArena::zeroed());
                let p = (api.stralloc)(
                    &mut *arena as *mut StringArena as *mut c_void,
                    std::ptr::null_mut(),
                );
                eprintln!("child: stralloc(arena, NULL) = {p:?}");
            }
            // E50 — strreset(NULL)
            "strreset_null" => {
                (api.strreset)(std::ptr::null_mut());
                eprintln!("child: strreset(NULL) returned normally");
            }
            // E51 — hmget_key_ts with temp == NULL
            "hmget_ts_null_temp" => {
                let mut key = [1u8, 2, 3, 4];
                let p = (api.hmget_key_ts)(
                    std::ptr::null_mut(),
                    8,
                    key.as_mut_ptr() as *mut c_void,
                    4,
                    std::ptr::null_mut(),
                    HM_BINARY,
                );
                eprintln!("child: hmget_key_ts(temp=NULL) = {p:?}");
            }
            // E52 — hmput_key with key == NULL on a populated map (hash_bytes derefs it)
            "hmput_null_key" => {
                let mut m = Map::new(api, Shape::binary(8, 4));
                for k in 0i32..3 {
                    let mut kb = k.to_ne_bytes();
                    let mut e = [0u8; 8];
                    e[..4].copy_from_slice(&kb);
                    m.put_struct(kb.as_mut_ptr() as *mut c_void, &e, HM_BINARY);
                }
                let out = (api.hmput_key)(m.t as *mut c_void, 8, std::ptr::null_mut(), 4, HM_BINARY);
                eprintln!("child: hmput_key(key=NULL) = {out:?}");
            }
            // E53 — hmdel_key with key == NULL on a populated map
            "hmdel_null_key" => {
                let mut m = Map::new(api, Shape::binary(8, 4));
                for k in 0i32..3 {
                    let mut kb = k.to_ne_bytes();
                    let mut e = [0u8; 8];
                    e[..4].copy_from_slice(&kb);
                    m.put_struct(kb.as_mut_ptr() as *mut c_void, &e, HM_BINARY);
                }
                let out = (api.hmdel_key)(
                    m.t as *mut c_void,
                    8,
                    std::ptr::null_mut(),
                    4,
                    0,
                    HM_BINARY,
                );
                eprintln!("child: hmdel_key(key=NULL) = {out:?}");
            }
            // E54 — hmget_key with key == NULL on a populated map
            "hmget_null_key" => {
                let mut m = Map::new(api, Shape::binary(8, 4));
                for k in 0i32..3 {
                    let mut kb = k.to_ne_bytes();
                    let mut e = [0u8; 8];
                    e[..4].copy_from_slice(&kb);
                    m.put_struct(kb.as_mut_ptr() as *mut c_void, &e, HM_BINARY);
                }
                let out = (api.hmget_key)(m.t as *mut c_void, 8, std::ptr::null_mut(), 4, HM_BINARY);
                eprintln!("child: hmget_key(key=NULL) = {out:?}");
            }
            // E3 — arrgrowf with a min_cap so large that realloc() fails, so the
            // header writes go through `(stbds_array_header *) 32 - 1` == 0.
            "arrgrowf_oom" => {
                let out = (api.arrgrowf)(std::ptr::null_mut(), 1, 0, usize::MAX / 2);
                eprintln!("child: arrgrowf(OOM) = {out:?}");
            }
            // E3b — shmode_func with an elemsize so large that realloc() fails
            "shmode_oom" => {
                let out = (api.shmode_func)(usize::MAX / 8, SH_STRDUP);
                eprintln!("child: shmode_func(OOM) = {out:?}");
            }
            // E43
            "arrfreef_null" => {
                (api.arrfreef)(std::ptr::null_mut());
                eprintln!("child: arrfreef(NULL) returned normally");
            }
            // E35 — hand-built hash_index with slot_count == 1 makes
            // make_hash_index(2, ot) trip the lib.c:401 assert.
            "make_hash_index_assert" => {
                let es = 16usize;
                let a = (api.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
                let hdr = (a as *mut u8).wrapping_sub(HDR_SIZE) as *mut ArrHeader;
                (*hdr).length = 1;
                let tbl: &'static mut HashIndex = Box::leak(Box::new(HashIndex {
                    temp_key: std::ptr::null_mut(),
                    slot_count: 1,
                    used_count: 1,
                    used_count_threshold: 0,
                    used_count_shrink_threshold: 0,
                    tombstone_count: 0,
                    tombstone_count_threshold: 0,
                    seed: 0x3141_5926,
                    slot_count_log2: 0,
                    string: StringArena::zeroed(),
                    storage: std::ptr::null_mut(),
                }));
                (*hdr).hash_table = tbl as *mut HashIndex as *mut c_void;
                let t = (a as *mut u8).wrapping_add(es) as *mut c_void;
                let mut key = [1u8, 2, 3, 4, 5, 6, 7, 8];
                let out = (api.hmput_key)(t, es, key.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
                eprintln!("child: hmput_key returned {out:?}");
            }
            // E16 — mode 2 delete where old_index != final_index: the re-find
            // hashes the *address* of the key field as a C string, so
            // hm_find_slot returns -1 and assert 846 fires.
            "mode2_hmdel" => {
                let es = 16usize;
                let mut m = Map::new(api, Shape::string(es));
                let mut keys: Vec<Vec<u8>> = vec![
                    b"alpha_key\0".to_vec(),
                    b"beta_key\0".to_vec(),
                    b"gamma_key\0".to_vec(),
                ];
                for (i, k) in keys.iter_mut().enumerate() {
                    m.shput_value(
                        k.as_mut_ptr() as *mut c_char,
                        8,
                        &(i as u64).to_ne_bytes(),
                        HM_STRING,
                    );
                }
                // delete the FIRST key: old_index(0) != final_index(2)
                let d = m.del(keys[0].as_mut_ptr() as *mut c_void, 2);
                eprintln!("child: mode-2 hmdel returned {d}");
            }
            other => panic!("unknown scenario {other}"),
        }
    });
}

/// E38 — `intput(9)`
#[test]
fn err_e38_intput_9() {
    let o = child::assert_same("intput:9");
    assert_eq!(o.signal, Some(6), "E38 expected SIGABRT, got {o:?}", o = o.stderr);
    let a = o.assertion.expect("E38 assertion line");
    assert!(
        a.contains("lib.c:955")
            && a.contains("intput")
            && a.contains("hmget(intmap, num) == 7"),
        "E38 unexpected diagnostic: {a}"
    );
}

/// E39 — `intput(11)`
#[test]
fn err_e39_intput_11() {
    let o = child::assert_same("intput:11");
    assert_eq!(o.signal, Some(6), "E39 expected SIGABRT");
    let a = o.assertion.expect("E39 assertion line");
    assert!(
        a.contains("lib.c:955")
            && a.contains("intput")
            && a.contains("hmget(intmap, num) == 7"),
        "E39 unexpected diagnostic: {a}"
    );
}

/// E28 — `stbds_hash_string(NULL, seed)`
#[test]
fn err_e28_hash_string_null() {
    let o = child::assert_same("hash_string_null");
    assert_eq!(o.signal, Some(11), "E28 expected SIGSEGV");
}

/// E35 — `stbds_make_hash_index` assert (lib.c:401)
#[test]
fn err_e35_make_hash_index_assert() {
    let o = child::assert_same("make_hash_index_assert");
    assert_eq!(o.signal, Some(6), "E35 expected SIGABRT");
    let a = o.assertion.expect("E35 assertion line");
    assert!(
        a.contains("lib.c:401")
            && a.contains("stbds_make_hash_index")
            && a.contains("t->used_count_threshold + t->tombstone_count_threshold < t->slot_count"),
        "E35 unexpected diagnostic: {a}"
    );
}

/// E16 — `mode == 2` delete with `old_index != final_index` → assert 846
#[test]
fn err_e16_mode2_hmdel() {
    let o = child::assert_same("mode2_hmdel");
    assert_eq!(o.signal, Some(6), "E16 expected SIGABRT");
    let a = o.assertion.expect("E16 assertion line");
    assert!(
        a.contains("lib.c:846") && a.contains("stbds_hmdel_key") && a.contains("slot >= 0"),
        "E16 unexpected diagnostic: {a}"
    );
}

/// E43 — `stbds_arrfreef(NULL)` → `free((char *) NULL - 32)`
#[test]
fn err_e43_arrfreef_null() {
    let o = child::assert_same("arrfreef_null");
    assert!(
        o.signal.is_some() || o.code == Some(101),
        "E43 expected a fault in both libraries, got {:?}/{:?}",
        o.code,
        o.signal
    );
}

/// E47 — `stbds_hash_bytes(NULL, len > 0, seed)` dereferences `p`
#[test]
fn err_e47_hash_bytes_null_nonzero_len() {
    for n in [1usize, 4, 7, 8, 9, 16, 1000] {
        let o = child::assert_same(&format!("hash_bytes_null:{n}"));
        assert_eq!(o.signal, Some(11), "E47 len={n} expected SIGSEGV");
    }
}

/// E48 — `stbds_stralloc(NULL, str)` dereferences the arena
#[test]
fn err_e48_stralloc_null_arena() {
    let o = child::assert_same("stralloc_null_arena");
    assert_eq!(o.signal, Some(11), "E48 expected SIGSEGV");
}

/// E49 — `stbds_stralloc(arena, NULL)` dereferences the string
#[test]
fn err_e49_stralloc_null_str() {
    let o = child::assert_same("stralloc_null_str");
    assert_eq!(o.signal, Some(11), "E49 expected SIGSEGV");
}

/// E50 — `stbds_strreset(NULL)` dereferences the arena
#[test]
fn err_e50_strreset_null() {
    let o = child::assert_same("strreset_null");
    assert_eq!(o.signal, Some(11), "E50 expected SIGSEGV");
}

/// E51 — `stbds_hmget_key_ts(..., temp = NULL, ...)` writes through `temp`
#[test]
fn err_e51_hmget_ts_null_temp() {
    let o = child::assert_same("hmget_ts_null_temp");
    assert_eq!(o.signal, Some(11), "E51 expected SIGSEGV");
}

/// E52 / E53 / E54 — `key == NULL` on a populated map (`hash_bytes` derefs it)
#[test]
fn err_e52_e54_null_key() {
    for scen in ["hmput_null_key", "hmdel_null_key", "hmget_null_key"] {
        let o = child::assert_same(scen);
        assert_eq!(o.signal, Some(11), "{scen} expected SIGSEGV");
    }
}

/// E3 — `realloc` failure inside `stbds_arrgrowf` / `stbds_shmode_func`
#[test]
fn err_e3_arrgrowf_alloc_failure() {
    for scen in ["arrgrowf_oom", "shmode_oom"] {
        let o = child::assert_same(scen);
        assert_eq!(
            o.signal,
            Some(11),
            "{scen} expected SIGSEGV (write through NULL+32 header)"
        );
    }
}
