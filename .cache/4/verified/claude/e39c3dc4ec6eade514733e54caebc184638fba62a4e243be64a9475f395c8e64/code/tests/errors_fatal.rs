//! Phase C, ERRORS.md — rows whose expected C result is a *fatal signal*
//! (`assert()` -> `SIGABRT`, or a NULL dereference -> `SIGSEGV`).
//!
//! Each scenario is run in a `fork()`ed child and the two implementations must
//! terminate the SAME way (identical signal, or identical clean exit code).
//! The `assert()` message text legitimately differs (it names the C file and
//! line), so only the wait status is compared; the child's stdout/stderr go to
//! `/dev/null`.
//!
//! EVERYTHING lives in ONE `#[test]` function on purpose: `fork()` from a
//! process with several running test threads risks inheriting a held malloc
//! lock in the child.  With a single test the harness runs only this body.

mod common;
use common::*;
use std::ffi::c_void;

#[test]
fn fatal_error_paths() {
    // The RELEASE Rust `.so` is compared here, not the dev one: the C library is
    // built without -DNDEBUG and without -O, i.e. with no instrumentation, and
    // the Rust release profile matches that.  The Rust *dev* profile enables
    // `debug_assertions`, which insert MIR null-pointer-dereference checks, so a
    // scenario the C answers with SIGSEGV the dev build answers with a
    // non-unwinding panic -> SIGABRT.  That is Rust deliberately trapping the
    // same UB, not a behavioural difference in the translation; see
    // `dev_build_only_traps_the_same_ub` at the bottom of this file.
    let (p, _g) = session_release(INITIAL_HASH_SEED);

    // -----------------------------------------------------------------------
    // row 2 — arrgrowf: the allocation fails, `b = NULL + 32`, then
    //         `stbds_header(b)->length = 0` writes to address 0.
    // -----------------------------------------------------------------------
    // elemsize * min_cap + 32 must NOT wrap, and must be unallocatable.
    for (es, min_cap) in [
        (1usize, 1usize << 50),
        (1, 1usize << 55),
        (8, 1usize << 52),
        (16, 1usize << 50),
    ] {
        let out = |lib: &Lib| unsafe {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
            // never reached; keep `a` observable so nothing is optimised away
            std::hint::black_box(a);
        };
        assert_same_fate(
            p,
            &format!("row2 arrgrowf(NULL, es={es}, 0, min_cap=2^{})", min_cap.trailing_zeros()),
            out,
        );
        let c = fork_run(|| out(&p.c));
        assert_eq!(
            c,
            Outcome::Signaled(SIGSEGV),
            "row2: the C must die with SIGSEGV"
        );
    }

    // -----------------------------------------------------------------------
    // row 4 — arrfreef(NULL) => free((char*)NULL - 32) => glibc abort
    // -----------------------------------------------------------------------
    // `free((char*)NULL - 32)` = `free(0xffffffffffffffe0)`: glibc reads the
    // chunk header just below that address and faults, so the observed outcome
    // is SIGSEGV rather than its "free(): invalid pointer" abort.  Whatever it
    // is, both implementations must do the same thing.
    let f = |lib: &Lib| unsafe { (lib.arrfreef)(std::ptr::null_mut()) };
    assert_same_fate(p, "row4 arrfreef(NULL)", f);
    assert!(
        fork_run(|| f(&p.c)).is_fatal_signal(),
        "row4: free() of an invalid pointer must be fatal in the C"
    );

    // -----------------------------------------------------------------------
    // rows 29/30 — hmdel_key's swap-with-last re-lookup assert.
    //
    // Reachable for ANY `mode >= 2`: `hmdel_key` picks the re-lookup key with
    //   `mode == STBDS_HM_STRING`  (lib.c:842)
    // but `stbds_hm_find_slot` decides how to hash with
    //   `mode >= STBDS_HM_STRING` (lib.c:590)
    // so for mode >= 2 it hands `find_slot` the ADDRESS of the key pointer and
    // `find_slot` hashes those pointer bytes as a string.  The slot is never
    // found => `STBDS_ASSERT(slot >= 0)` fires.  Row 30's
    // `b->index[i] == final_index` assert sits immediately after and is
    // therefore dominated by row 29 -- it can never be observed on its own.
    // -----------------------------------------------------------------------
    for &mode in &[2i32, 3, 100, i32::MAX] {
        let es = 16usize;
        let ks = 8usize;
        let scenario = move |lib: &Lib| unsafe {
            let mut t: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..6usize {
                let mut s = format!("fk_{i}").into_bytes();
                s.push(0);
                keys.push(s);
                let k = keys.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                t = (lib.hmput_key)(t, es, k, ks, mode);
            }
            // delete the FIRST key => old_index (0) != final_index (4)
            let mut s = b"fk_0\0".to_vec();
            let k = s.as_mut_ptr() as *mut c_void;
            let r = (lib.hmdel_key)(t, es, k, ks, 0, mode);
            std::hint::black_box(r);
        };
        assert_same_fate(p, &format!("rows29/30 hmdel_key non-last, mode={mode}"), scenario);
        assert_eq!(
            fork_run(|| scenario(&p.c)),
            Outcome::Signaled(SIGABRT),
            "rows29/30: the C assert must fire for mode={mode}"
        );
    }

    // sanity: for mode == 1 exactly, the same scenario must SUCCEED on both
    for &mode in &[1i32] {
        let es = 16usize;
        let ks = 8usize;
        let scenario = move |lib: &Lib| unsafe {
            let mut t: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..6usize {
                let mut s = format!("fk_{i}").into_bytes();
                s.push(0);
                keys.push(s);
                let k = keys.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                t = (lib.hmput_key)(t, es, k, ks, mode);
            }
            let mut s = b"fk_0\0".to_vec();
            let k = s.as_mut_ptr() as *mut c_void;
            let r = (lib.hmdel_key)(t, es, k, ks, 0, mode);
            assert!(!r.is_null());
        };
        assert_same_fate(p, &format!("rows29/30 control, mode={mode}"), scenario);
        assert_eq!(
            fork_run(|| scenario(&p.c)),
            Outcome::Exited(0),
            "mode==1 must delete a non-last element cleanly"
        );
    }

    // -----------------------------------------------------------------------
    // row 34b — stralloc with `remaining > 0` but `storage == NULL`:
    //           the fast path dereferences `a->storage->storage`.
    // -----------------------------------------------------------------------
    // `len` for "x" is 2, so `remaining >= 2` is what actually takes the fast
    // path (`remaining == 1` still enters the block and allocates one, which is
    // covered as the control case below).
    for remaining in [2usize, 8, 512, usize::MAX] {
        let scenario = move |lib: &Lib| unsafe {
            let mut arena = [0u64; 3];
            let ap = arena.as_mut_ptr() as *mut u8;
            wr_usize(ap, ARENA_REMAINING, remaining);
            let mut s = b"x\0".to_vec();
            let r = (lib.stralloc)(ap as *mut c_void, s.as_mut_ptr() as *mut i8);
            std::hint::black_box(r);
        };
        assert_same_fate(
            p,
            &format!("row34b stralloc(remaining={remaining}, storage=NULL)"),
            scenario,
        );
        assert!(
            fork_run(|| scenario(&p.c)).is_fatal_signal(),
            "row34b: dereferencing a NULL storage must be fatal (remaining={remaining})"
        );
    }
    // control: remaining < len still allocates a block, so it must succeed
    for remaining in [0usize, 1] {
        let scenario = move |lib: &Lib| unsafe {
            let mut arena = [0u64; 3];
            let ap = arena.as_mut_ptr() as *mut u8;
            wr_usize(ap, ARENA_REMAINING, remaining);
            let mut s = b"x\0".to_vec();
            let r = (lib.stralloc)(ap as *mut c_void, s.as_mut_ptr() as *mut i8);
            assert!(!r.is_null());
            (lib.strreset)(ap as *mut c_void);
        };
        assert_same_fate(
            p,
            &format!("row34b control stralloc(remaining={remaining} < len)"),
            scenario,
        );
        assert_eq!(fork_run(|| scenario(&p.c)), Outcome::Exited(0));
    }

    // -----------------------------------------------------------------------
    // row 38 (fatal half) — `block` values whose blocksize cannot be allocated:
    //   blocksize = 512 << (block>>1); block 44..109 => 2^40..2^63 => malloc
    //   fails => `sb->next = a->storage` writes through NULL.
    // -----------------------------------------------------------------------
    for block in [60u8, 80, 100, 108] {
        let scenario = move |lib: &Lib| unsafe {
            let mut arena = [0u64; 3];
            let ap = arena.as_mut_ptr() as *mut u8;
            wr_u8(ap, ARENA_BLOCK, block);
            let mut s = b"y\0".to_vec();
            let r = (lib.stralloc)(ap as *mut c_void, s.as_mut_ptr() as *mut i8);
            std::hint::black_box(r);
        };
        assert_same_fate(p, &format!("row38-fatal stralloc(block={block})"), scenario);
    }

    // -----------------------------------------------------------------------
    // row 56 — hmfree_func's strdup loop runs over `1..length`, so the DEFAULT
    //          slot (raw index 0) is never freed.
    //
    // Proof by construction: plant an *interior* pointer (into the middle of a
    // live allocation) in the default slot's key field.  glibc aborts on
    // free() of an interior pointer, so if either implementation started the
    // loop at 0 the child would die with SIGABRT.  Both must exit cleanly.
    // -----------------------------------------------------------------------
    let scenario = |lib: &Lib| unsafe {
        let es = 16usize;
        let ks = 8usize;
        let t = (lib.shmode_func)(es, STBDS_SH_STRDUP as i32);
        let mut t = t;
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..5usize {
            let mut s = format!("sk_{i}").into_bytes();
            s.push(0);
            keys.push(s);
            let k = keys.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            t = (lib.hmput_key)(t, es, k, ks, STBDS_HM_STRING);
        }
        // t[-1] is the default slot == raw element 0
        let victim: Vec<u8> = vec![7u8; 64];
        let interior = victim.as_ptr().add(8) as *mut u8;
        wr_ptr((t as *mut u8).sub(es), 0, interior);
        (lib.hmfree_func)((t as *mut u8).sub(es) as *mut c_void, es);
        std::hint::black_box(&victim);
    };
    assert_same_fate(p, "row56 hmfree_func must not free the default slot", scenario);
    assert_eq!(
        fork_run(|| scenario(&p.c)),
        Outcome::Exited(0),
        "row56: the C must NOT free raw element 0"
    );

    // -----------------------------------------------------------------------
    // Generic FFI boundary sweep: NULL pointers and out-of-range enum values
    // into every exported entry point that takes them.  Whatever happens
    // (clean return or a signal) must happen identically.
    // -----------------------------------------------------------------------
    let null = std::ptr::null_mut::<c_void>();

    // hash_string(NULL, seed) dereferences immediately
    let s1 = |lib: &Lib| unsafe {
        std::hint::black_box((lib.hash_string)(std::ptr::null_mut(), 1234));
    };
    assert_same_fate(p, "generic hash_string(NULL)", s1);

    // hash_bytes(NULL, len>0, seed) dereferences
    for len in [1usize, 8, 4096] {
        let s = move |lib: &Lib| unsafe {
            std::hint::black_box((lib.hash_bytes)(std::ptr::null_mut(), len, 5));
        };
        assert_same_fate(p, &format!("generic hash_bytes(NULL, {len})"), s);
    }

    // stralloc(arena, NULL) -> strlen(NULL)
    let s2 = |lib: &Lib| unsafe {
        let mut arena = [0u64; 3];
        std::hint::black_box((lib.stralloc)(
            arena.as_mut_ptr() as *mut c_void,
            std::ptr::null_mut(),
        ));
    };
    assert_same_fate(p, "generic stralloc(arena, NULL)", s2);

    // stralloc(NULL, str) -> a->remaining read through NULL
    let s3 = |lib: &Lib| unsafe {
        let mut s = b"z\0".to_vec();
        std::hint::black_box((lib.stralloc)(null, s.as_mut_ptr() as *mut i8));
    };
    assert_same_fate(p, "generic stralloc(NULL, str)", s3);

    // strreset(NULL) -> a->storage read through NULL
    let s4 = |lib: &Lib| unsafe { (lib.strreset)(null) };
    assert_same_fate(p, "generic strreset(NULL)", s4);

    // hmget_key / hmget_key_ts / hmput_key with a NULL *key* but a non-NULL map
    for &mode in &[-5i32, 0, 1, 2, 1000, i32::MIN, i32::MAX] {
        let s = move |lib: &Lib| unsafe {
            let es = 16usize;
            let t = (lib.hmput_key)(std::ptr::null_mut(), es, null, 8, mode);
            std::hint::black_box(t);
        };
        assert_same_fate(p, &format!("generic hmput_key(NULL map, NULL key, mode={mode})"), s);

        let s = move |lib: &Lib| unsafe {
            let es = 16usize;
            let mut key = b"k\0".to_vec();
            let mut t =
                (lib.hmput_key)(std::ptr::null_mut(), es, key.as_mut_ptr() as *mut c_void, 8, mode);
            t = (lib.hmget_key)(t, es, null, 8, mode);
            std::hint::black_box(t);
        };
        assert_same_fate(p, &format!("generic hmget_key(NULL key, mode={mode})"), s);

        let s = move |lib: &Lib| unsafe {
            let es = 16usize;
            let mut key = b"k\0".to_vec();
            let mut t =
                (lib.hmput_key)(std::ptr::null_mut(), es, key.as_mut_ptr() as *mut c_void, 8, mode);
            let mut tmp: isize = 0;
            t = (lib.hmget_key_ts)(t, es, null, 8, &mut tmp, mode);
            std::hint::black_box((t, tmp));
        };
        assert_same_fate(p, &format!("generic hmget_key_ts(NULL key, mode={mode})"), s);
    }

    // hmget_key_ts with a NULL `temp` out-pointer
    let s5 = |lib: &Lib| unsafe {
        let mut key = b"k\0".to_vec();
        std::hint::black_box((lib.hmget_key_ts)(
            std::ptr::null_mut(),
            16,
            key.as_mut_ptr() as *mut c_void,
            8,
            std::ptr::null_mut(),
            0,
        ));
    };
    assert_same_fate(p, "generic hmget_key_ts(temp=NULL)", s5);

    // hmfree_func on a NULL-but-offset pointer (what `hmfree(p)` does for a
    // non-NULL p): pass a bogus non-NULL map
    let s6 = |lib: &Lib| unsafe { (lib.hmfree_func)(64 as *mut c_void, 8) };
    assert_same_fate(p, "generic hmfree_func(bogus non-NULL)", s6);

    // shmode_func with a zero elemsize (the memset/length writes still happen)
    for &mode in &[0i32, 1, 2, 3, 999, -1] {
        let s = move |lib: &Lib| unsafe {
            let t = (lib.shmode_func)(0, mode);
            std::hint::black_box(t);
        };
        assert_same_fate(p, &format!("generic shmode_func(elemsize=0, mode={mode})"), s);
    }

    // keysize larger than elemsize: hmput_key memcpy's past the element
    for &(es, ks) in &[(4usize, 64usize), (8, 4096)] {
        let s = move |lib: &Lib| unsafe {
            let mut key = vec![0xABu8; ks];
            let t = (lib.hmput_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                ks,
                0,
            );
            std::hint::black_box(t);
        };
        assert_same_fate(p, &format!("generic hmput_key(es={es} < keysize={ks})"), s);
    }

    // arrgrowf with elemsize 0 and huge counts (no allocation is needed)
    for &min_cap in &[0usize, 1, 1 << 40, usize::MAX] {
        let s = move |lib: &Lib| unsafe {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            std::hint::black_box(a);
        };
        assert_same_fate(p, &format!("generic arrgrowf(es=0, min_cap={min_cap})"), s);
    }

    // arrgrowf where elemsize*min_cap WRAPS (the classic integer-overflow input)
    for &(es, min_cap) in &[
        (2usize, usize::MAX),
        (8, usize::MAX / 4),
        (1, usize::MAX),
        (16, (usize::MAX / 16) + 1),
    ] {
        let s = move |lib: &Lib| unsafe {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
            std::hint::black_box(a);
        };
        assert_same_fate(
            p,
            &format!("generic arrgrowf overflow(es={es}, min_cap={min_cap})"),
            s,
        );
    }

    // arr_push with the extreme ints.
    //
    // NOTE: `arr_push(n)` performs ~n^2/100 pushes, so `i32::MAX` is ~10^16
    // operations -- not runnable for EITHER implementation, and the `i += 50`
    // step would additionally overflow `int` (UB) once `i > INT_MAX - 50`.
    // The largest feasible values are used instead.
    for &num in &[i32::MIN, i32::MIN + 1, -1, 0, 1, 49, 50, 51, 2000] {
        let s = move |lib: &Lib| unsafe { (lib.arr_push)(num) };
        assert_same_fate(p, &format!("generic arr_push({num})"), s);
    }

    // strkey across the whole i32 range endpoints
    for &n in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX] {
        let s = move |lib: &Lib| unsafe {
            std::hint::black_box((lib.strkey)(n));
        };
        assert_same_fate(p, &format!("generic strkey({n})"), s);
    }
}

/// Documents (and pins) the ONE difference between the dev and release Rust
/// `.so`: for inputs that make the C perform undefined behaviour (a store
/// through a NULL pointer after a failed `realloc`), the release build faults
/// exactly like the C, while the dev build's `debug_assertions` turn the same UB
/// into a controlled abort.
///
/// This is a build-profile property, not a translation difference -- but it is
/// asserted rather than assumed.
#[test]
fn dev_build_only_traps_the_same_ub() {
    let cpair = libs(); // C + dev Rust
    let rpair = libs_release(); // C + release Rust

    let scenario = |lib: &Lib| unsafe {
        let a = (lib.arrgrowf)(std::ptr::null_mut(), 1, 0, 1usize << 50);
        std::hint::black_box(a);
    };

    let c = fork_run(|| scenario(&cpair.c));
    let dev = fork_run(|| scenario(&cpair.rs));
    let rel = fork_run(|| scenario(&rpair.rs));

    eprintln!("C={c:?}  Rust(dev)={dev:?}  Rust(release)={rel:?}");
    assert_eq!(c, Outcome::Signaled(SIGSEGV), "the C must fault");
    assert_eq!(
        rel, c,
        "the RELEASE Rust .so must fault exactly like the C .so"
    );
    assert_eq!(
        dev,
        Outcome::Signaled(SIGABRT),
        "the dev Rust .so is expected to trap the same UB as an abort"
    );
}

/// Guards the harness itself: the `.so` files under test must reflect the
/// CURRENT `src/lib.rs` and `c_src/src/lib.c`.
///
/// `[lib] crate-type = ["cdylib"]` produces no rlib, so cargo omits the `lib`
/// target from the integration-test unit graph -- plain `cargo test` does NOT
/// rebuild `libarr_push_lib.so`. `common::ensure_built` compensates by invoking
/// `cargo build --lib` before the first `dlopen`. This test proves the resulting
/// `.so` is newer than the source, i.e. that a source edit really is picked up.
#[test]
fn harness_tests_the_current_source() {
    // touching libs() triggers the rebuild
    let _ = libs();
    let _ = libs_release();

    let src = std::fs::metadata(manifest_dir().join("src/lib.rs"))
        .unwrap()
        .modified()
        .unwrap();
    for so in [rust_so_path(), rust_release_so_path()] {
        let m = std::fs::metadata(&so)
            .unwrap_or_else(|e| panic!("{}: {e}", so.display()))
            .modified()
            .unwrap();
        assert!(
            m >= src,
            "{} is OLDER than src/lib.rs -- the suite would be testing stale code",
            so.display()
        );
    }
    let csrc = std::fs::metadata(manifest_dir().join("c_src/src/lib.c"))
        .unwrap()
        .modified()
        .unwrap();
    let cso = std::fs::metadata(c_so_path()).unwrap().modified().unwrap();
    assert!(
        cso >= csrc,
        "{} is OLDER than c_src/src/lib.c -- rebuild the C library",
        c_so_path().display()
    );
}
