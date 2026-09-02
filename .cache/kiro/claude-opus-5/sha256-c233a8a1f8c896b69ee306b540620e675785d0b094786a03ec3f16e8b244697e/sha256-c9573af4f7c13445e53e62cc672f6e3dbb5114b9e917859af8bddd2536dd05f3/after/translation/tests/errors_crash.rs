//! Phase C, continued: the rows of `ERRORS.md` whose "expected C result" is a
//! crash or an `assert()` abort.  Each call runs in a forked child so the
//! termination status is an observable, comparable result.

mod common;
use common::*;

use std::ffi::{c_int, c_void, CString};

// ===========================================================================
// row 3 -- stbds_arrgrowf with a size computation that overflows
//          (`elemsize * min_cap + sizeof(header)` wraps)
// ===========================================================================
#[test]
fn e03_arrgrowf_size_overflow() {
    let _g = serial();
    let p = pair();
    for (elemsize, addlen, min_cap) in [
        (1usize, 0usize, usize::MAX),
        (1, usize::MAX, 0),
        (16, 0, usize::MAX / 16 + 1),
        (8, 0, usize::MAX / 8 + 4),
        (usize::MAX, 0, 4),
    ] {
        let co = child_outcome(|| unsafe {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
            std::hint::black_box(a);
        });
        let ro = child_outcome(|| unsafe {
            let a = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
            std::hint::black_box(a);
        });
        assert_eq!(
            co, ro,
            "row3 arrgrowf(NULL,{elemsize},{addlen},{min_cap}): C={co} RUST={ro}"
        );
    }
}

// ===========================================================================
// row 4 -- stbds_arrfreef(NULL): the C has NO null check, so it calls
//          free((char *) NULL - 32).  Both libraries must fail identically.
// ===========================================================================
#[test]
fn e04_arrfreef_null() {
    let _g = serial();
    let p = pair();
    let co = child_outcome(|| unsafe { (p.c.arrfreef)(std::ptr::null_mut()) });
    let ro = child_outcome(|| unsafe { (p.r.arrfreef)(std::ptr::null_mut()) });
    assert_eq!(co, ro, "row4 arrfreef(NULL): C={co} RUST={ro}");
    assert_ne!(co, "exited(0)", "row4: expected the C to die on free(NULL-32)");
}

// ===========================================================================
// row 19 -- stbds_hmdel_key: STBDS_ASSERT(slot >= 0) when the re-find of the
//           element moved in from the tail fails.
//
// We construct it deterministically: build a table, locate the bucket slot
// holding `final_index`, and mark it deleted in BOTH libraries (the bucket
// layouts are provably identical -- every other test compares them), then
// delete an element with `old_index != final_index`.  The re-find then hits an
// empty slot, returns -1, and the assert fires.
// ===========================================================================
unsafe fn build_and_corrupt_then_delete(lib: &Lib, keys: &mut [u64]) {
    let elemsize = 16usize;
    let mut h: *mut c_void = std::ptr::null_mut();
    for (i, k) in keys.iter_mut().enumerate() {
        h = put_and_fill(
            lib,
            h,
            elemsize,
            8,
            k as *mut u64 as *mut c_void,
            STBDS_HM_BINARY,
            i as u8,
        )
        .0;
    }
    let table = table_of(h, elemsize);
    let final_index = header_of(h, elemsize).length as isize - 2;
    // find and clobber the bucket slot that maps to `final_index`
    let nbuckets = (*table).slot_count >> BUCKET_SHIFT;
    let mut done = false;
    for bi in 0..nbuckets {
        let b = (*table).storage.add(bi);
        for j in 0..BUCKET_LENGTH {
            if (*b).index[j] == final_index {
                (*b).hash[j] = 1; // STBDS_HASH_DELETED
                (*b).index[j] = -2; // STBDS_INDEX_DELETED
                done = true;
                break;
            }
        }
        if done {
            break;
        }
    }
    assert!(done, "could not find the bucket slot for final_index");
    // delete the FIRST key: old_index == 0 != final_index -> memmove + re-find
    let mut k0 = keys[0];
    (lib.hmdel_key)(
        h,
        elemsize,
        &mut k0 as *mut u64 as *mut c_void,
        8,
        0,
        STBDS_HM_BINARY,
    );
}

#[test]
fn e19_hmdel_refind_assert() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 19);
    unsafe {
        let keys: Vec<u64> = (0..6).map(|_| rng.next_u64()).collect();
        let co = child_outcome(|| {
            (p.c.rand_seed)(0x1357);
            let mut k = keys.clone();
            build_and_corrupt_then_delete(&p.c, &mut k);
        });
        let ro = child_outcome(|| {
            (p.r.rand_seed)(0x1357);
            let mut k = keys.clone();
            build_and_corrupt_then_delete(&p.r, &mut k);
        });
        assert_eq!(co, ro, "row19: C={co} RUST={ro}");
        assert_eq!(
            co, SIGABRT,
            "row19: expected SIGABRT from the failing STBDS_ASSERT, got {co}"
        );
    }
}

// ===========================================================================
// Extra crash-parity checks for the generic FFI boundaries where the C is
// documented (ERRORS.md rows 9-14) as NOT dereferencing the key: verify that a
// NULL key really is accepted, i.e. the child exits cleanly in BOTH libraries.
// ===========================================================================
#[test]
fn e_null_key_accepted_where_c_does_not_read_it() {
    let _g = serial();
    let p = pair();
    let cases: Vec<(&str, fn(&Lib))> = vec![
        ("hmdel_key(NULL a, NULL key)", |lib: &Lib| unsafe {
            (lib.hmdel_key)(
                std::ptr::null_mut(),
                16,
                std::ptr::null_mut(),
                8,
                0,
                STBDS_HM_STRING,
            );
        }),
        ("hmget_key_ts(NULL a, NULL key)", |lib: &Lib| unsafe {
            let mut t: isize = 0;
            (lib.hmget_key_ts)(
                std::ptr::null_mut(),
                16,
                std::ptr::null_mut(),
                8,
                &mut t,
                STBDS_HM_STRING,
            );
        }),
        ("hmput_key(NULL a, NULL key, keysize 0)", |lib: &Lib| unsafe {
            (lib.hmput_key)(
                std::ptr::null_mut(),
                16,
                std::ptr::null_mut(),
                0,
                STBDS_HM_BINARY,
            );
        }),
        ("hash_bytes(NULL, 0)", |lib: &Lib| unsafe {
            std::hint::black_box((lib.hash_bytes)(std::ptr::null_mut(), 0, 7));
        }),
        ("hmfree_func(NULL)", |lib: &Lib| unsafe {
            (lib.hmfree_func)(std::ptr::null_mut(), 16);
        }),
    ];
    for (name, f) in cases {
        let co = child_outcome(|| f(&p.c));
        let ro = child_outcome(|| f(&p.r));
        assert_eq!(co, ro, "{name}: C={co} RUST={ro}");
        assert_eq!(co, "exited(0)", "{name}: expected a clean exit, got {co}");
    }
}

// ===========================================================================
// And the mirror image: where the C *does* dereference a NULL key it must
// crash in both.
// ===========================================================================
#[test]
fn e_null_key_crashes_where_c_reads_it() {
    let _g = serial();
    let p = pair();
    let cases: Vec<(&str, fn(&Lib))> = vec![
        ("hash_string(NULL)", |lib: &Lib| unsafe {
            std::hint::black_box((lib.hash_string)(std::ptr::null_mut(), 3));
        }),
        ("hash_bytes(NULL, 8)", |lib: &Lib| unsafe {
            std::hint::black_box((lib.hash_bytes)(std::ptr::null_mut(), 8, 3));
        }),
        ("hmput_key(NULL key, STRING)", |lib: &Lib| unsafe {
            (lib.hmput_key)(
                std::ptr::null_mut(),
                16,
                std::ptr::null_mut(),
                8,
                STBDS_HM_STRING,
            );
        }),
        ("stralloc(arena, NULL)", |lib: &Lib| unsafe {
            let mut a = StringArena::new();
            (lib.stralloc)(&mut a, std::ptr::null_mut());
        }),
        ("strreset(NULL)", |lib: &Lib| unsafe {
            (lib.strreset)(std::ptr::null_mut());
        }),
    ];
    for (name, f) in cases {
        let co = child_outcome(|| f(&p.c));
        let ro = child_outcome(|| f(&p.r));
        assert_eq!(co, ro, "{name}: C={co} RUST={ro}");
        assert_ne!(co, "exited(0)", "{name}: expected the C to fault");
    }
}

// ===========================================================================
// ERRORS.md row 19 via the PUBLIC ABI: `stbds_hmdel_key` uses the exact
// comparison `mode == STBDS_HM_STRING` to pick the re-find call, but
// `stbds_hm_find_slot` dispatches on `mode >= STBDS_HM_STRING`.  With
// `mode == 2` on a string table, a delete that has to move the tail element in
// (`old_index != final_index`) therefore re-finds using the ADDRESS of the
// element instead of the stored `char *`, the lookup fails, and
// `STBDS_ASSERT(slot >= 0)` aborts.  No internal state is touched here.
// ===========================================================================
unsafe fn del_first_with_mode(lib: &Lib, sh: c_int, mode: c_int) {
    let keys: Vec<CString> = (0..4).map(|i| CString::new(format!("aa_{i}")).unwrap()).collect();
    let mut h = (lib.shmode_func)(16, sh);
    for (i, k) in keys.iter().enumerate() {
        h = put_and_fill(lib, h, 16, 8, k.as_ptr() as *mut c_void, mode, i as u8).0;
    }
    // old_index == 0, final_index == 3 -> memmove + re-find
    (lib.hmdel_key)(h, 16, keys[0].as_ptr() as *mut c_void, 8, 0, mode);
}

#[test]
fn e19b_hmdel_refind_assert_via_public_abi() {
    let _g = serial();
    let p = pair();
    for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for mode in [2i32, 5, i32::MAX] {
            let co = child_outcome(|| unsafe {
                (p.c.rand_seed)(0x2468);
                del_first_with_mode(&p.c, sh, mode);
            });
            let ro = child_outcome(|| unsafe {
                (p.r.rand_seed)(0x2468);
                del_first_with_mode(&p.r, sh, mode);
            });
            assert_eq!(co, ro, "row19b sh={sh} mode={mode}: C={co} RUST={ro}");
            assert_eq!(
                co, SIGABRT,
                "row19b sh={sh} mode={mode}: expected SIGABRT from STBDS_ASSERT(slot >= 0), got {co}"
            );
        }
    }
    // Control: with mode == STBDS_HM_STRING the very same delete succeeds.
    for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let co = child_outcome(|| unsafe {
            (p.c.rand_seed)(0x2468);
            del_first_with_mode(&p.c, sh, STBDS_HM_STRING);
        });
        let ro = child_outcome(|| unsafe {
            (p.r.rand_seed)(0x2468);
            del_first_with_mode(&p.r, sh, STBDS_HM_STRING);
        });
        assert_eq!(co, ro, "row19b control sh={sh}");
        assert_eq!(co, "exited(0)", "row19b control sh={sh}: expected success, got {co}");
    }
}

// ===========================================================================
// Sanity: `child_outcome` really distinguishes clean exits from signals,
// otherwise every test above would be vacuous.
// ===========================================================================
#[test]
fn e_child_outcome_self_check() {
    let _g = serial();
    assert_eq!(child_outcome(|| {}), "exited(0)");
    let crash = child_outcome(|| unsafe {
        let p: *mut u8 = 8 as *mut u8;
        std::ptr::write_volatile(p, 1);
    });
    assert_ne!(crash, "exited(0)", "expected the deliberate fault to be reported");
    let _ = CString::new("keep the import used").unwrap();
}
