//! Level 6: parameter and state edge cases that the ordinary stb_ds macros
//! never reach, but that the exported functions accept.

mod harness;

use harness::*;
use std::ffi::c_void;

const ELEMSIZE: usize = 16;

/// `stbds_shmode_func` stores `(unsigned char) mode`, so out-of-range and
/// negative modes must truncate identically.
#[test]
fn shmode_func_truncates_mode_to_u8() {
    let _g = shared_lock();
    let p = pair();
    for &mode in &[4i32, 5, 127, 128, 255, 256, 257, 300, -1, -2, -256, i32::MIN] {
        unsafe {
            p.c.rand_seed(0x1111_2222);
            p.rs.rand_seed(0x1111_2222);
            let ct = p.c.shmode_func(ELEMSIZE, mode);
            let rt = p.rs.shmode_func(ELEMSIZE, mode);
            let a = snapshot_binary(ct, ELEMSIZE, &[]);
            let b = snapshot_binary(rt, ELEMSIZE, &[]);
            assert_eq!(a, b, "shmode_func(mode={})", mode);
            assert_eq!(
                a.string_mode,
                (mode as u32 & 0xff) as u8,
                "string.mode for mode={}",
                mode
            );
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

/// A map created through `shmode_func` with a truncated mode that lands outside
/// 1..=3 falls into `hmput_key`'s `default:` arm, i.e. a binary `memcpy` of the
/// key even though `mode == STBDS_HM_STRING` was requested at the call site.
#[test]
fn out_of_range_string_mode_falls_back_to_memcpy() {
    let _g = shared_lock();
    let p = pair();
    let keysize = 8usize;
    for &mode in &[0i32, 4, 200, 256] {
        unsafe {
            p.c.rand_seed(0x3141_5926);
            p.rs.rand_seed(0x3141_5926);
            let mut ct = p.c.shmode_func(ELEMSIZE, mode);
            let mut rt = p.rs.shmode_func(ELEMSIZE, mode);
            let defined = [(0usize, keysize), (8usize, 4usize)];
            for i in 0..40u64 {
                let key = i.to_le_bytes().to_vec();
                let value = (i as i32).to_le_bytes().to_vec();
                ct = hmput(&p.c, ct, ELEMSIZE, &key, keysize, &value, 8);
                rt = hmput(&p.rs, rt, ELEMSIZE, &key, keysize, &value, 8);
                assert_eq!(
                    snapshot_binary(ct, ELEMSIZE, &defined),
                    snapshot_binary(rt, ELEMSIZE, &defined),
                    "mode={} insert {}",
                    mode,
                    i
                );
            }
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

/// `stbds_hmput_default` also has a `length == 0` branch, which needs an array
/// produced by `stbds_arrgrowf` directly (the macros never make one).
#[test]
fn hmput_default_on_zero_length_array() {
    let _g = shared_lock();
    let p = pair();
    unsafe {
        let ca = p.c.arrgrowf(std::ptr::null_mut(), ELEMSIZE, 0, 1);
        let ra = p.rs.arrgrowf(std::ptr::null_mut(), ELEMSIZE, 0, 1);
        assert_eq!(header(ca).length, 0);
        assert_eq!(header(ra).length, 0);

        let ct = p.c.hmput_default((ca as *mut u8).add(ELEMSIZE) as *mut c_void, ELEMSIZE);
        let rt = p
            .rs
            .hmput_default((ra as *mut u8).add(ELEMSIZE) as *mut c_void, ELEMSIZE);
        assert_eq!(
            snapshot_binary(ct, ELEMSIZE, &[]),
            snapshot_binary(rt, ELEMSIZE, &[]),
            "hmput_default on a zero-length array"
        );
        assert_eq!(hmlen(ct, ELEMSIZE), 0);
        hmfree(&p.c, ct, ELEMSIZE);
        hmfree(&p.rs, rt, ELEMSIZE);
    }
}

/// `keyoffset` is a real parameter of `stbds_hmdel_key`, but `stbds_hmput_key`
/// always stores keys at offset 0, so a non-zero `keyoffset` is only valid if
/// the element carries a second copy of the key there. Build exactly that: an
/// element with the key pointer duplicated at offset 0 and offset 8, then delete
/// through `keyoffset = 8`. (Passing a `keyoffset` that does *not* describe the
/// stored layout makes the C library trip its own
/// `STBDS_ASSERT(slot >= 0)` - that is the C code's behaviour, not something to
/// compare outputs for.)
#[test]
fn hmdel_key_honours_nonzero_keyoffset() {
    let _g = shared_lock();
    let p = pair();
    let keys: Vec<Vec<u8>> = (0..80).map(|i| cstring(&format!("dup_key_{}", i))).collect();

    for &keyoffset in &[0usize, 8] {
        unsafe {
            p.c.rand_seed(0x7654_3210);
            p.rs.rand_seed(0x7654_3210);
            // SH_DEFAULT stores the caller's char* in the slot. (SH_NONE would
            // take hmput_key's `default:` arm, which memcpy's `keysize` bytes
            // *from* the key pointer - i.e. the first 8 characters of the
            // string - and the slot would then hold a bogus pointer.)
            let mut ct = p.c.shmode_func(ELEMSIZE, SH_DEFAULT);
            let mut rt = p.rs.shmode_func(ELEMSIZE, SH_DEFAULT);

            let dup = |t: *mut c_void, idx: isize, kp: *mut c_void| {
                *((t as *mut u8).add(ELEMSIZE * idx as usize + 8) as *mut *mut c_void) = kp;
            };

            for k in &keys {
                let kp = k.as_ptr() as *mut c_void;
                ct = p.c.hmput_key(ct, ELEMSIZE, kp, 8, HM_STRING);
                rt = p.rs.hmput_key(rt, ELEMSIZE, kp, 8, HM_STRING);
                let ci = header((ct as *mut u8).sub(ELEMSIZE) as *mut c_void).temp;
                let ri = header((rt as *mut u8).sub(ELEMSIZE) as *mut c_void).temp;
                assert_eq!(ci, ri);
                dup(ct, ci, kp);
                dup(rt, ri, kp);
            }
            // element 0 is the zeroed default slot; its duplicate stays NULL
            assert_eq!(
                snapshot_string(ct, ELEMSIZE, 8, false),
                snapshot_string(rt, ELEMSIZE, 8, false),
                "keyoffset={} before deletes",
                keyoffset
            );

            for (i, k) in keys.iter().enumerate() {
                let kp = k.as_ptr() as *mut c_void;
                let c2 = p.c.hmdel_key(ct, ELEMSIZE, kp, 8, keyoffset, HM_STRING);
                let r2 = p.rs.hmdel_key(rt, ELEMSIZE, kp, 8, keyoffset, HM_STRING);
                let cr = header((c2 as *mut u8).sub(ELEMSIZE) as *mut c_void).temp;
                let rr = header((r2 as *mut u8).sub(ELEMSIZE) as *mut c_void).temp;
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "hmdel(keyoffset={}, key {}) result", keyoffset, i);
                assert_eq!(cr, 1, "delete of key {} should have succeeded", i);
                assert_eq!(
                    snapshot_string(ct, ELEMSIZE, 8, false),
                    snapshot_string(rt, ELEMSIZE, 8, false),
                    "hmdel keyoffset={} key={}",
                    keyoffset,
                    i
                );
            }
            assert_eq!(hmlen(ct, ELEMSIZE), 0);
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

/// `stbds_hmget_key` / `_ts` accept `mode` values above `STBDS_HM_STRING`
/// (`stbds_pshgeti` uses `STBDS_HM_PTR_TO_STRING == 2`); the code only ever
/// tests `mode >= STBDS_HM_STRING`.
#[test]
fn modes_above_hm_string_take_the_string_path() {
    let _g = shared_lock();
    let p = pair();
    let value_offset = 8usize;
    let keys: Vec<Vec<u8>> = (0..60).map(|i| cstring(&format!("ptr_key_{}", i))).collect();

    unsafe {
        p.c.rand_seed(0x0FED_CBA9);
        p.rs.rand_seed(0x0FED_CBA9);
        // SH_DEFAULT keeps the caller's pointer, which is what stbds_pshput
        // relies on when it passes STBDS_HM_PTR_TO_STRING == 2.
        let mut ct = p.c.shmode_func(ELEMSIZE, SH_DEFAULT);
        let mut rt = p.rs.shmode_func(ELEMSIZE, SH_DEFAULT);

        for (i, k) in keys.iter().enumerate() {
            let kp = k.as_ptr() as *mut c_void;
            ct = p.c.hmput_key(ct, ELEMSIZE, kp, 8, 2);
            rt = p.rs.hmput_key(rt, ELEMSIZE, kp, 8, 2);
            let ci = header((ct as *mut u8).sub(ELEMSIZE) as *mut c_void).temp;
            let ri = header((rt as *mut u8).sub(ELEMSIZE) as *mut c_void).temp;
            assert_eq!(ci, ri, "put index for {}", i);
            *((ct as *mut u8).add(ELEMSIZE * ci as usize + value_offset) as *mut i32) = i as i32;
            *((rt as *mut u8).add(ELEMSIZE * ri as usize + value_offset) as *mut i32) = i as i32;
            assert_eq!(
                snapshot_string(ct, ELEMSIZE, value_offset, false),
                snapshot_string(rt, ELEMSIZE, value_offset, false),
                "mode 2 insert {}",
                i
            );
        }
        for (i, k) in keys.iter().enumerate() {
            let kp = k.as_ptr() as *mut c_void;
            ct = p.c.hmget_key(ct, ELEMSIZE, kp, 8, 2);
            rt = p.rs.hmget_key(rt, ELEMSIZE, kp, 8, 2);
            let ci = header((ct as *mut u8).sub(ELEMSIZE) as *mut c_void).temp;
            let ri = header((rt as *mut u8).sub(ELEMSIZE) as *mut c_void).temp;
            assert_eq!(ci, ri, "mode 2 get index for {}", i);
            assert!(ci >= 0, "mode 2 key {} missing", i);
        }
        hmfree(&p.c, ct, ELEMSIZE);
        hmfree(&p.rs, rt, ELEMSIZE);
    }
}

/// `stbds_arrgrowf` is also the array API's growth hook; drive it the way
/// `arrput`/`arraddn` do (grow by one, bump length) for a long run and compare
/// the payload the whole way.
#[test]
fn arrgrowf_as_arrput_matches() {
    let p = pair();
    for &elemsize in &[1usize, 4, 8, 16] {
        unsafe {
            let mut ca: *mut c_void = std::ptr::null_mut();
            let mut ra: *mut c_void = std::ptr::null_mut();
            for i in 0..2000usize {
                // stbds_arrmaybegrow(a, 1)
                let need_grow = ca.is_null() || header(ca).length + 1 > header(ca).capacity;
                assert_eq!(
                    need_grow,
                    ra.is_null() || header(ra).length + 1 > header(ra).capacity
                );
                if need_grow {
                    ca = p.c.arrgrowf(ca, elemsize, 1, 0);
                    ra = p.rs.arrgrowf(ra, elemsize, 1, 0);
                }
                assert_eq!(header(ca).capacity, header(ra).capacity, "cap at {}", i);
                assert_eq!(header(ca).length, header(ra).length, "len at {}", i);

                let byte = (i % 251) as u8;
                let off = header(ca).length * elemsize;
                std::ptr::write_bytes((ca as *mut u8).add(off), byte, elemsize);
                std::ptr::write_bytes((ra as *mut u8).add(off), byte, elemsize);
                let nl = header(ca).length + 1;
                set_length(ca, nl);
                set_length(ra, nl);
            }
            let n = header(ca).length * elemsize;
            let cs = std::slice::from_raw_parts(ca as *const u8, n);
            let rs = std::slice::from_raw_parts(ra as *const u8, n);
            assert_eq!(cs, rs, "payload for elemsize {}", elemsize);
            p.c.arrfreef(ca);
            p.rs.arrfreef(ra);
        }
    }
}
