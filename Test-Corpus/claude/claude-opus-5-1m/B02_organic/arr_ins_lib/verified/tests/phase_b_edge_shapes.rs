//! Phase B — CONFIGS.md rows 75-79: degenerate-but-well-defined input shapes
//! that the rest of the suite does not reach.
mod common;

use common::*;
use std::ffi::c_void;

unsafe fn hdr(a: *mut c_void) -> Header {
    *(((a as *mut u8).sub(HEADER_SIZE)) as *mut Header)
}
unsafe fn map_hdr(hp: *mut c_void, elemsize: usize) -> Header {
    hdr((hp as *mut u8).sub(elemsize) as *mut c_void)
}

// --- row 75: elemsize == 0 in stbds_arrgrowf ----------------------------
// `elemsize * min_cap + sizeof(stbds_array_header)` == 32, so the allocation is
// header-only and no element byte is ever touched. Well defined.
#[test]
fn cfg_75_arrgrowf_zero_elemsize() {
    let s = session();
    for min_cap in [0usize, 1, 4, 5, 1000, usize::MAX] {
        for addlen in [0usize, 1, 7] {
            unsafe {
                let c = (s.c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
                let r = (s.rust.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
                assert_eq!(c.is_null(), r.is_null());
                if c.is_null() {
                    continue;
                }
                assert_same(
                    &format!("row75 arrgrowf(NULL, 0, {}, {})", addlen, min_cap),
                    &dump_array(c, 0, 0),
                    &dump_array(r, 0, 0),
                );
                (s.c.arrfreef)(c);
                (s.rust.arrfreef)(r);
            }
        }
    }
    // grow ladder with elemsize 0
    unsafe {
        let mut c: *mut c_void = std::ptr::null_mut();
        let mut r: *mut c_void = std::ptr::null_mut();
        for n in 0..40usize {
            if c.is_null() || hdr(c).length + 1 > hdr(c).capacity {
                c = (s.c.arrgrowf)(c, 0, 1, 0);
                r = (s.rust.arrgrowf)(r, 0, 1, 0);
            }
            (*(((c as *mut u8).sub(HEADER_SIZE)) as *mut Header)).length = n + 1;
            (*(((r as *mut u8).sub(HEADER_SIZE)) as *mut Header)).length = n + 1;
            assert_same(
                &format!("row75 ladder n={}", n),
                &dump_array(c, 0, 0),
                &dump_array(r, 0, 0),
            );
        }
        (s.c.arrfreef)(c);
        (s.rust.arrfreef)(r);
    }
}

// --- row 76: keysize == 0 (memcmp of 0 bytes is always "equal") ----------
// `stbds_hash_bytes(key, 0, seed)` is the same value for every key and
// `memcmp(key, stored, 0) == 0` always matches, so the map degenerates into a
// single-entry map that every put updates in place. Fully deterministic.
#[test]
fn cfg_76_zero_keysize_binary_map() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 76);
    for elemsize in [8usize, 16, 40] {
        let lay = Layout {
            name: "K0",
            elemsize,
            keysize: 0,
        };
        unsafe {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            for i in 0..30usize {
                let val = rng.bytes(elemsize);
                cp = map_put_binary(s.c, cp, lay, &[], &val, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &[], &val, HM_BINARY);
                assert_same(
                    &format!("row76 es={} put #{}", elemsize, i),
                    &dump_map(cp, DumpOpts::raw(elemsize)),
                    &dump_map(rp, DumpOpts::raw(elemsize)),
                );
                // exactly one live entry no matter how many puts
                assert_eq!(map_hdr(cp, elemsize).length, 2);
                assert_eq!(map_hdr(rp, elemsize).length, 2);
            }
            // lookups / deletes with a zero-length key
            let mut dummy = [0u8; 1];
            let (c1, ci) = map_geti(s.c, cp, lay, dummy.as_mut_ptr() as *mut c_void, HM_BINARY);
            let (r1, ri) = map_geti(s.rust, rp, lay, dummy.as_mut_ptr() as *mut c_void, HM_BINARY);
            cp = c1;
            rp = r1;
            assert_eq!(ci, ri);
            assert_eq!(ci, 0);
            let (c2, ct) = map_del(s.c, cp, lay, dummy.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
            let (r2, rt) =
                map_del(s.rust, rp, lay, dummy.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
            cp = c2;
            rp = r2;
            assert_eq!(ct, rt);
            assert_eq!(ct, 1);
            assert_same(
                &format!("row76 es={} after delete", elemsize),
                &dump_map(cp, DumpOpts::raw(elemsize)),
                &dump_map(rp, DumpOpts::raw(elemsize)),
            );
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
}

// --- row 77: keysize == elemsize ("set", no value part) -----------------
#[test]
fn cfg_77_keysize_equals_elemsize() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 77);
    for elemsize in [1usize, 2, 4, 8, 16, 32] {
        let lay = Layout {
            name: "SET",
            elemsize,
            keysize: elemsize,
        };
        unsafe {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let want = if elemsize == 1 { 200usize } else { 60 };
            for _ in 0..want {
                let k = rng.bytes(elemsize);
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                cp = map_put_binary(s.c, cp, lay, &k, &[], HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &k, &[], HM_BINARY);
                assert_same(
                    &format!("row77 es={} n={}", elemsize, keys.len()),
                    &dump_map(cp, DumpOpts::raw(elemsize)),
                    &dump_map(rp, DumpOpts::raw(elemsize)),
                );
            }
            for k in keys.iter() {
                let mut kk = k.clone();
                let (c1, ci) = map_geti(s.c, cp, lay, kk.as_mut_ptr() as *mut c_void, HM_BINARY);
                let (r1, ri) = map_geti(s.rust, rp, lay, kk.as_mut_ptr() as *mut c_void, HM_BINARY);
                cp = c1;
                rp = r1;
                assert_eq!(ci, ri);
                assert!(ci >= 0);
            }
            for k in keys.iter() {
                let mut kk = k.clone();
                let (c1, ct) = map_del(s.c, cp, lay, kk.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let (r1, rt) =
                    map_del(s.rust, rp, lay, kk.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                cp = c1;
                rp = r1;
                assert_eq!(ct, rt);
                assert_same(
                    &format!("row77 es={} del", elemsize),
                    &dump_map(cp, DumpOpts::raw(elemsize)),
                    &dump_map(rp, DumpOpts::raw(elemsize)),
                );
            }
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
}

// --- row 78: very large elemsize -----------------------------------------
#[test]
fn cfg_78_large_elemsize() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 78);
    for elemsize in [256usize, 1024, 4096] {
        let lay = Layout {
            name: "HUGE",
            elemsize,
            keysize: 8,
        };
        unsafe {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..20 {
                let k = rng.bytes(8);
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                let v = rng.bytes(elemsize - 8);
                cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
            }
            assert_same(
                &format!("row78 es={} after puts", elemsize),
                &dump_map(cp, DumpOpts::raw(elemsize)),
                &dump_map(rp, DumpOpts::raw(elemsize)),
            );
            for k in keys.iter() {
                let mut kk = k.clone();
                let (c1, ct) = map_del(s.c, cp, lay, kk.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let (r1, rt) =
                    map_del(s.rust, rp, lay, kk.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                cp = c1;
                rp = r1;
                assert_eq!(ct, rt);
            }
            assert_same(
                &format!("row78 es={} after dels", elemsize),
                &dump_map(cp, DumpOpts::raw(elemsize)),
                &dump_map(rp, DumpOpts::raw(elemsize)),
            );
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
}

// --- row 79: hmget_key_ts must never touch the header `temp` -------------
#[test]
fn cfg_79_hmget_ts_leaves_header_temp_alone() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 79);
    let lay = L_I2I;
    unsafe {
        let mut cp: *mut c_void = std::ptr::null_mut();
        let mut rp: *mut c_void = std::ptr::null_mut();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for _ in 0..25 {
            let k = rng.next_u32().to_ne_bytes().to_vec();
            if keys.contains(&k) {
                continue;
            }
            keys.push(k.clone());
            let v = rng.next_u32().to_ne_bytes();
            cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
            rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
        }
        // plant a recognisable value in the header temp
        for sentinel in [-5isize, 0, 12345, isize::MIN, isize::MAX] {
            (*(((cp as *mut u8).sub(lay.elemsize + HEADER_SIZE)) as *mut Header)).temp = sentinel;
            (*(((rp as *mut u8).sub(lay.elemsize + HEADER_SIZE)) as *mut Header)).temp = sentinel;
            for k in keys.iter().chain(std::iter::once(&vec![0xEEu8; 4])) {
                let mut kk = k.clone();
                let (c1, ct, chdr) =
                    map_geti_ts(s.c, cp, lay, kk.as_mut_ptr() as *mut c_void, HM_BINARY);
                let (r1, rt, rhdr) =
                    map_geti_ts(s.rust, rp, lay, kk.as_mut_ptr() as *mut c_void, HM_BINARY);
                cp = c1;
                rp = r1;
                assert_eq!(ct, rt);
                assert_eq!(chdr, rhdr);
                assert_eq!(
                    chdr, sentinel,
                    "C hmget_key_ts must not write the header temp"
                );
                assert_eq!(
                    rhdr, sentinel,
                    "RUST hmget_key_ts must not write the header temp"
                );
            }
        }
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);
    }
}
