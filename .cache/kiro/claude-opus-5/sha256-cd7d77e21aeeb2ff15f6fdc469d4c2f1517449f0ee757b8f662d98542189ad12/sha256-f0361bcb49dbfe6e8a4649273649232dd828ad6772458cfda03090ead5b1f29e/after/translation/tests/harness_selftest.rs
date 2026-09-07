//! Negative controls for the differential harness itself.
//!
//! Every other test asserts "C and Rust agree". These tests assert that the
//! comparison machinery would actually NOTICE a disagreement — otherwise a
//! green suite would prove nothing.

mod common;

use common::*;
use std::ffi::c_void;


/// The map snapshot must be sensitive to the entry set, the element payload,
/// the bucket layout, `temp` and `length`.
///
/// Note: the bucket-layout probe uses *string* keys. `stbds_siphash_bytes`
/// XORs `seed` into `v0..v3` twice (`v0 = X ^ seed`, then `v0 ^= C ^ seed`), so
/// the seed cancels out and `stbds_hash_bytes` is seed-INDEPENDENT — a real
/// quirk of the C source that both libraries reproduce, and the reason a binary
/// map's bucket layout does not change with the seed.
#[test]
fn harness_snapshot_detects_differences() {
    let _g = lock();
    let (c, _r) = libs();
    let elemsize = 16usize;
    let keysize = 8usize;

    unsafe {
        let mut keys: Vec<Vec<u8>> = (0..20u32)
            .map(|i| {
                let mut v = format!("key_{i}").into_bytes();
                v.push(0);
                v
            })
            .collect();

        let mut build = |seed: usize| -> *mut c_void {
            (c.rand_seed)(seed);
            let mut h: *mut c_void = (c.shmode_func)(elemsize, STBDS_SH_STRDUP);
            for k in keys.iter_mut() {
                h = (c.hmput_key)(
                    h,
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    keysize,
                    STBDS_HM_STRING,
                );
                let t =
                    std::ptr::read_unaligned((h as *const u8).sub(elemsize + 8) as *const isize);
                write_tail_at(h, elemsize, t, 8, 1);
            }
            h
        };

        let a = build(1);
        let b = build(2);

        let sa = snap_map(a, elemsize, KeyKind::StringPtr { keyoffset: 0 }, false);
        let sb = snap_map(b, elemsize, KeyKind::StringPtr { keyoffset: 0 }, false);
        assert_ne!(
            sa.table.as_ref().unwrap().seed,
            sb.table.as_ref().unwrap().seed,
            "harness: different global seeds must produce different table seeds"
        );
        assert_ne!(sa, sb, "harness: snapshots must differ when the seed differs");
        assert_ne!(
            sa.table.as_ref().unwrap().slots,
            sb.table.as_ref().unwrap().slots,
            "harness: bucket layout must be part of the comparison"
        );

        // element payload sensitivity
        let sa2 = snap_map(a, elemsize, KeyKind::StringPtr { keyoffset: 0 }, false);
        assert_eq!(sa, sa2, "harness: snapshot must be stable");
        *(a as *mut u8).add(elemsize + 8) ^= 0xFF;
        let sa3 = snap_map(a, elemsize, KeyKind::StringPtr { keyoffset: 0 }, false);
        assert_ne!(sa, sa3, "harness: element bytes must be compared");
        *(a as *mut u8).add(elemsize + 8) ^= 0xFF;

        // header `temp` sensitivity
        let before = snap_map(a, elemsize, KeyKind::StringPtr { keyoffset: 0 }, false);
        let tp = (a as *mut u8).sub(elemsize + 8) as *mut isize;
        let saved = *tp;
        *tp = saved ^ 0x5A5A;
        let after = snap_map(a, elemsize, KeyKind::StringPtr { keyoffset: 0 }, false);
        assert_ne!(before, after, "harness: header temp must be compared");
        *tp = saved;

        // length sensitivity
        let raw = (a as *mut u8).sub(elemsize);
        let lp = raw.sub(HEADER) as *mut usize;
        let saved_len = *lp;
        *lp = saved_len - 1;
        let shorter = snap_map(a, elemsize, KeyKind::StringPtr { keyoffset: 0 }, false);
        assert_ne!(before, shorter, "harness: length must be compared");
        *lp = saved_len;

        (c.hmfree_func)(raw as *mut c_void, elemsize);
        (c.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// Pin the `stbds_siphash_bytes` seed-cancellation quirk: `stbds_hash_bytes`
/// must ignore its `seed` argument in BOTH libraries, while
/// `stbds_hash_string` must honour it.
#[test]
fn harness_hash_bytes_is_seed_independent_in_both() {
    let _g = lock();
    let (c, r) = libs();
    let mut rng = Rng::new(0x5EED);
    unsafe {
        for _ in 0..300 {
            let len = rng.range(0, 64);
            let mut buf = rng.bytes(len.max(1));
            let p = buf.as_mut_ptr() as *mut c_void;
            let base_c = (c.hash_bytes)(p, len, 0);
            let base_r = (r.hash_bytes)(p, len, 0);
            same("selftest hash_bytes seed 0", base_c, base_r);
            for seed in [1usize, 2, DEFAULT_SEED, usize::MAX, rng.next_u64() as usize] {
                assert_eq!(
                    (c.hash_bytes)(p, len, seed),
                    base_c,
                    "C hash_bytes unexpectedly depends on the seed"
                );
                assert_eq!(
                    (r.hash_bytes)(p, len, seed),
                    base_r,
                    "Rust hash_bytes unexpectedly depends on the seed"
                );
            }
        }
        // hash_string, by contrast, does depend on the seed
        let mut s = b"abcdef\0".to_vec();
        let sp = s.as_mut_ptr() as *mut _;
        let h0c = (c.hash_string)(sp, 0);
        let h1c = (c.hash_string)(sp, 1);
        assert_ne!(h0c, h1c, "C hash_string must depend on the seed");
        same("selftest hash_string seed 0", h0c, (r.hash_string)(sp, 0));
        same("selftest hash_string seed 1", h1c, (r.hash_string)(sp, 1));
    }
}

/// `same()` must panic on a mismatch (and not on a match).
#[test]
fn harness_same_panics_on_mismatch() {
    let _g = lock();
    same("equal values", 42u64, 42u64);
    let r = std::panic::catch_unwind(|| same("unequal values", 1u64, 2u64));
    assert!(r.is_err(), "harness: same() failed to report a divergence");
}

/// `in_child` must distinguish a clean exit, a non-zero exit and a fatal signal.
#[test]
fn harness_in_child_reports_outcomes() {
    let _g = lock();
    assert_eq!(in_child(|| {}), Outcome::Exited(0));
    assert_eq!(
        in_child(|| unsafe {
            let p: *mut u8 = std::ptr::null_mut();
            std::ptr::write_volatile(p, 1);
        }),
        Outcome::Signaled(SIGSEGV),
        "harness: a null write must be reported as SIGSEGV"
    );
    let (c, _r) = libs();
    // a known-aborting C path (row 30 style) must be reported as SIGABRT
    let o = in_child(|| unsafe {
        let mut a = Arena::zeroed();
        a.remaining = 1000;
        let mut s = b"x\0".to_vec();
        (c.stralloc)(&mut a, s.as_mut_ptr() as *mut _);
    });
    assert!(is_fatal(o), "harness: expected a fatal outcome, got {o:?}");
}

/// The arena snapshot must notice a different block layout.
#[test]
fn harness_arena_snapshot_is_sensitive() {
    let _g = lock();
    let (c, _r) = libs();
    unsafe {
        let mut a = Arena::zeroed();
        let mut b = Arena::zeroed();
        let mut s1: Vec<u8> = vec![b'x'; 10];
        s1.push(0);
        let mut s2: Vec<u8> = vec![b'x'; 5000];
        s2.push(0);
        (c.stralloc)(&mut a, s1.as_mut_ptr() as *mut _);
        (c.stralloc)(&mut b, s2.as_mut_ptr() as *mut _);
        assert_ne!(
            (a.remaining, a.block),
            (b.remaining, b.block),
            "harness: arena state must differ between the short and oversize paths"
        );
        (c.strreset)(&mut a);
        (c.strreset)(&mut b);
    }
}
