//! Phase B rows 1-12, 52: hashing primitives, `stbds_arrgrowf`/`arrfreef`,
//! and `strkey`.  Both implementations are reached only through `dlsym`.

mod common;
use common::*;

use std::ffi::{c_int, c_void, CStr, CString};

// --- rows 1-6: stbds_hash_bytes -------------------------------------------

unsafe fn hash_bytes_both(p: &Pair, buf: &mut [u8], len: usize, seed: usize) {
    let cv = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
    let rv = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
    assert_eq!(
        cv, rv,
        "hash_bytes(len={len}, seed={seed:#x}, bytes={:x?}) C={cv:#x} RUST={rv:#x}",
        &buf[..len.min(buf.len())]
    );
}

#[test]
fn cfg01_hash_bytes_len0() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 1);
    unsafe {
        for _ in 0..500 {
            let seed = r.next_u64() as usize;
            let mut buf = r.bytes(8);
            hash_bytes_both(p, &mut buf, 0, seed);
        }
        // and the fixed interesting seeds
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, usize::MAX - 1] {
            let mut buf = [0u8; 8];
            hash_bytes_both(p, &mut buf, 0, seed);
        }
    }
}

#[test]
fn cfg02_hash_bytes_tail_1_to_7() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 2);
    unsafe {
        for len in 1..8usize {
            for _ in 0..400 {
                let mut buf = r.bytes(len.max(1));
                let seed = r.next_u64() as usize;
                hash_bytes_both(p, &mut buf, len, seed);
            }
            // deterministic corner bytes: all 0x00, all 0xff, single high bit
            for pat in [0x00u8, 0xff, 0x80, 0x7f] {
                let mut buf = vec![pat; len];
                for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
                    hash_bytes_both(p, &mut buf, len, seed);
                }
            }
        }
    }
}

#[test]
fn cfg03_hash_bytes_len8() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 3);
    unsafe {
        for _ in 0..1000 {
            let mut buf = r.bytes(8);
            let seed = r.next_u64() as usize;
            hash_bytes_both(p, &mut buf, 8, seed);
        }
    }
}

#[test]
fn cfg04_hash_bytes_len9_to_64() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 4);
    unsafe {
        for len in 9..=64usize {
            for _ in 0..60 {
                let mut buf = r.bytes(len);
                let seed = r.next_u64() as usize;
                hash_bytes_both(p, &mut buf, len, seed);
            }
        }
    }
}

#[test]
fn cfg05_hash_bytes_high_bit_bytes() {
    let _g = serial();
    // Exercises the C `int` promotion + sign-extension path: d[3] and d[7]
    // with the high bit set.
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 5);
    unsafe {
        for len in 8..=64usize {
            for _ in 0..40 {
                let mut buf: Vec<u8> =
                    (0..len).map(|_| 0x80u8 | (r.next_u64() & 0x7f) as u8).collect();
                let seed = r.next_u64() as usize;
                hash_bytes_both(p, &mut buf, len, seed);
            }
            // exactly the sign bits, nothing else
            let mut buf = vec![0u8; len];
            for i in (3..len).step_by(4) {
                buf[i] = 0x80;
            }
            hash_bytes_both(p, &mut buf, len, 0);
            hash_bytes_both(p, &mut buf, len, usize::MAX);
        }
    }
}

#[test]
fn cfg06_hash_bytes_large() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 6);
    unsafe {
        for len in [512usize, 513, 1000, 1024, 4095, 4096] {
            for seed in [0usize, 1, usize::MAX, r.next_u64() as usize] {
                let mut buf = r.bytes(len);
                hash_bytes_both(p, &mut buf, len, seed);
            }
        }
    }
}

#[test]
fn cfg06b_hash_bytes_null_and_zero() {
    let _g = serial();
    // ERRORS row 30 lives in errors.rs; this just double-checks p=NULL/len=0
    // does not diverge here either.
    let p = pair();
    unsafe {
        for seed in [0usize, 7, usize::MAX] {
            let cv = (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let rv = (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(cv, rv, "hash_bytes(NULL,0,{seed:#x})");
        }
    }
}

// --- rows 7-8: stbds_hash_string ------------------------------------------

unsafe fn hash_string_both(p: &Pair, s: &CString, seed: usize) {
    let ptr = s.as_ptr() as *mut _;
    let cv = (p.c.hash_string)(ptr, seed);
    let rv = (p.r.hash_string)(ptr, seed);
    assert_eq!(cv, rv, "hash_string({:?}, {seed:#x}) C={cv:#x} RUST={rv:#x}", s);
}

#[test]
fn cfg07_hash_string_random() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 7);
    unsafe {
        hash_string_both(p, &CString::new("").unwrap(), 0);
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            hash_string_both(p, &CString::new("").unwrap(), seed);
            hash_string_both(p, &CString::new("a").unwrap(), seed);
        }
        for len in 1..=64usize {
            for _ in 0..40 {
                let s = r.cstring(len);
                let seed = r.next_u64() as usize;
                hash_string_both(p, &s, seed);
            }
        }
    }
}

#[test]
fn cfg08_hash_string_high_bytes() {
    let _g = serial();
    // `(unsigned char) *str++` -- bytes >= 0x80 must not sign-extend.
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 8);
    unsafe {
        for len in 1..=48usize {
            for _ in 0..30 {
                let v: Vec<u8> =
                    (0..len).map(|_| 0x80u8 | (r.next_u64() & 0x7f) as u8).collect();
                let s = CString::new(v).unwrap();
                let seed = r.next_u64() as usize;
                hash_string_both(p, &s, seed);
            }
            let s = CString::new(vec![0xffu8; len]).unwrap();
            hash_string_both(p, &s, 0);
            hash_string_both(p, &s, usize::MAX);
        }
    }
}

// --- row 9 / 58: rand_seed + shmode_func ----------------------------------

#[test]
fn cfg09_rand_seed_and_table_seed() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 9);
    unsafe {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, r.next_u64() as usize] {
            (p.c.rand_seed)(seed);
            (p.r.rand_seed)(seed);
            // three successive tables observe the seed advance
            for _ in 0..3 {
                let ct = (p.c.shmode_func)(16, SH_STRDUP);
                let rt = (p.r.shmode_func)(16, SH_STRDUP);
                assert_eq_dump(
                    &format!("shmode_func after rand_seed({seed:#x})"),
                    &dump_table(ct, 16, 8),
                    &dump_table(rt, 16, 8),
                );
                (p.c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
                (p.r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}

#[test]
fn cfg58_shmode_func_elemsize_x_mode() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 24, 64] {
            for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
                (p.c.rand_seed)(0x1234_5678);
                (p.r.rand_seed)(0x1234_5678);
                let ct = (p.c.shmode_func)(elemsize, mode);
                let rt = (p.r.shmode_func)(elemsize, mode);
                assert_eq_dump(
                    &format!("shmode_func(elemsize={elemsize}, mode={mode})"),
                    &dump_table(ct, elemsize, elemsize.min(8)),
                    &dump_table(rt, elemsize, elemsize.min(8)),
                );
                (p.c.hmfree_func)((ct as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (p.r.hmfree_func)((rt as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// --- rows 10-12: stbds_arrgrowf / stbds_arrfreef --------------------------

unsafe fn arr_dump(a: *mut c_void) -> String {
    if a.is_null() {
        return "NULL".to_string();
    }
    let h = &*((a as *mut u8).sub(HDRSIZE) as *mut ArrayHeader);
    format!(
        "length={} capacity={} temp={} table_null={}",
        h.length, h.capacity, h.temp, h.hash_table.is_null()
    )
}

#[test]
fn cfg10_arrgrowf_from_null_cross_product() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [1usize, 2, 4, 8, 16, 64] {
            for addlen in [0usize, 1, 2, 3, 4, 5, 100] {
                for min_cap in [0usize, 1, 3, 4, 5, 100] {
                    let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let what = format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap})");
                    assert_eq!(
                        ca.is_null(),
                        ra.is_null(),
                        "{what}: nullness differs"
                    );
                    if ca.is_null() {
                        continue;
                    }
                    assert_eq_dump(&what, &arr_dump(ca), &arr_dump(ra));
                    (p.c.arrfreef)(ca);
                    (p.r.arrfreef)(ra);
                }
            }
        }
    }
}

#[test]
fn cfg11_arrgrowf_chained_growth() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 11);
    unsafe {
        for elemsize in [1usize, 4, 8, 16] {
            for trial in 0..40 {
                let mut ca: *mut c_void = std::ptr::null_mut();
                let mut ra: *mut c_void = std::ptr::null_mut();
                for step in 0..12 {
                    let addlen = r.below(9);
                    let min_cap = r.below(13);
                    ca = (p.c.arrgrowf)(ca, elemsize, addlen, min_cap);
                    ra = (p.r.arrgrowf)(ra, elemsize, addlen, min_cap);
                    let what = format!(
                        "chained arrgrowf elemsize={elemsize} trial={trial} step={step} addlen={addlen} min_cap={min_cap}"
                    );
                    assert_eq_dump(&what, &arr_dump(ca), &arr_dump(ra));
                    if ca.is_null() {
                        // both returned the unchanged NULL input (no-op branch)
                        assert!(ra.is_null(), "{what}: nullness differs");
                        continue;
                    }
                    // simulate a consumer bumping the length like arrput does
                    let ch = &mut *((ca as *mut u8).sub(HDRSIZE) as *mut ArrayHeader);
                    let rh = &mut *((ra as *mut u8).sub(HDRSIZE) as *mut ArrayHeader);
                    let newlen = (ch.length + addlen).min(ch.capacity);
                    ch.length = newlen;
                    rh.length = newlen;
                }
                if !ca.is_null() {
                    (p.c.arrfreef)(ca);
                    (p.r.arrfreef)(ra);
                }
            }
        }
    }
}

#[test]
fn cfg12_arrgrowf_then_free() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [1usize, 8, 32] {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            assert_eq_dump("arrgrowf before free", &arr_dump(ca), &arr_dump(ra));
            (p.c.arrfreef)(ca);
            (p.r.arrfreef)(ra);
        }
    }
}

// --- row 52: strkey -------------------------------------------------------

#[test]
fn cfg52_strkey() {
    let _g = serial();
    let p = pair();
    let mut r = Rng::new(TEST_SEED ^ 52);
    unsafe {
        let mut ns: Vec<c_int> = vec![0, 1, 9, 10, 99, 100, 12345, -1, i32::MIN, i32::MAX];
        for _ in 0..500 {
            ns.push(r.next_u64() as i32);
        }
        // The returned pointer is a shared static buffer; check the pointer is
        // stable across calls in each library, and the contents match.
        let c0 = (p.c.strkey)(0);
        let r0 = (p.r.strkey)(0);
        for n in ns {
            let cp = (p.c.strkey)(n);
            let rp = (p.r.strkey)(n);
            assert_eq!(cp, c0, "C strkey buffer moved for n={n}");
            assert_eq!(rp, r0, "RUST strkey buffer moved for n={n}");
            let cs = CStr::from_ptr(cp).to_bytes().to_vec();
            let rs = CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(
                cs,
                rs,
                "strkey({n}) C={:?} RUST={:?}",
                String::from_utf8_lossy(&cs),
                String::from_utf8_lossy(&rs)
            );
        }
    }
}
