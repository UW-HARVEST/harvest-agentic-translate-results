//! Differential tests for `allocate_block` / `free_block`.
//!
//! Covers CONFIGS.md rows 10-18 and 44, and ERRORS.md rows 1, 2, 3, 4, 5, 13, 18.

mod common;

use common::*;
use core::ffi::{c_int, c_void};

/// Allocate through `imp`, copy out the whole observable state, free through
/// `imp`.  Returns `None` when `allocate_block` rejected the request.
unsafe fn snapshot(imp: &Impl, count: usize, init: c_int) -> Option<(usize, Vec<i32>)> {
    unsafe {
        let p = (imp.allocate_block)(count, init);
        if p.is_null() {
            return None;
        }
        let size = (*p).size;
        let data = (*p).data;
        assert!(!data.is_null(), "{}: data must be non-NULL here", imp.name);
        let mut v = Vec::with_capacity(size);
        for i in 0..size {
            v.push(*data.add(i));
        }
        (imp.free_block)(p);
        Some((size, v))
    }
}

fn check(c: &Impl, r: &Impl, count: usize, init: c_int, ctx: &str) {
    let a = unsafe { snapshot(c, count, init) };
    let b = unsafe { snapshot(r, count, init) };
    match (&a, &b) {
        (None, None) => {}
        (Some((sa, da)), Some((sb, db))) => {
            assert_eq!(sa, sb, "{ctx}: size mismatch (count={count} init={init})");
            assert_eq!(
                da, db,
                "{ctx}: data mismatch (count={count} init={init})\n C={:?}\n R={:?}",
                &da[..da.len().min(24)],
                &db[..db.len().min(24)]
            );
        }
        _ => panic!(
            "{ctx}: NULL-ness mismatch for count={count} init={init}: C={} Rust={}",
            a.is_some(),
            b.is_some()
        ),
    }
}

#[test]
fn allocate_and_free_differential() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x0000_0002);

    // ---- row 10 / ERRORS #13: count == 0 -------------------------------
    // glibc calloc(0, 4) returns a *unique non-NULL* pointer, so the
    // `if (!mb->data)` branch is NOT taken.
    for _ in 0..64 {
        check(&c, &r, 0, rng.interesting_i32(), "row10/count0");
    }
    unsafe {
        let p = (c.allocate_block)(0, 5);
        let q = (r.allocate_block)(0, 5);
        assert!(!p.is_null(), "ERRORS#13: C calloc(0,4) unexpectedly NULL");
        assert!(!q.is_null(), "ERRORS#13: Rust calloc(0,4) unexpectedly NULL");
        assert_eq!((*p).size, 0);
        assert_eq!((*q).size, 0);
        assert!(!(*p).data.is_null() && !(*q).data.is_null());
        (c.free_block)(p);
        (r.free_block)(q);
    }

    // ---- row 11: count == 1 --------------------------------------------
    for &init in &[0i32, 1, -1, 7, -7, i32::MAX, i32::MIN, i32::MAX - 1] {
        check(&c, &r, 1, init, "row11/count1");
    }

    // ---- row 12: count 2..=14 (the range betagamma can request) --------
    for count in 2..=14usize {
        for &init in &[0i32, 1, -1, 5, -5, i32::MAX, i32::MIN, 1_000_000_000] {
            check(&c, &r, count, init, "row12/small");
        }
        for _ in 0..40 {
            check(&c, &r, count, rng.interesting_i32(), "row12/small-rand");
        }
    }

    // ---- row 13: multi-page allocation ---------------------------------
    for &count in &[1024usize, 4096, 65_536] {
        for &init in &[0i32, -3, i32::MAX, i32::MIN] {
            check(&c, &r, count, init, "row13/multipage");
        }
    }

    // ---- row 14: init_value = INT_MAX wraps inside the loop ------------
    for count in 1..=20usize {
        check(&c, &r, count, i32::MAX, "row14/wrap-max");
        check(&c, &r, count, i32::MAX - 3, "row14/wrap-max-3");
    }
    unsafe {
        // spot check the wrap direction explicitly
        let p = (c.allocate_block)(3, i32::MAX);
        let q = (r.allocate_block)(3, i32::MAX);
        let cv = [*(*p).data, *(*p).data.add(1), *(*p).data.add(2)];
        let rv = [*(*q).data, *(*q).data.add(1), *(*q).data.add(2)];
        assert_eq!(cv, [i32::MAX, i32::MIN, i32::MIN + 1], "C wrap changed?");
        assert_eq!(cv, rv, "row14: wrap mismatch");
        (c.free_block)(p);
        (r.free_block)(q);
    }

    // ---- row 15: init_value crosses zero -------------------------------
    for count in 1..=20usize {
        check(&c, &r, count, i32::MIN, "row15/min");
        check(&c, &r, count, -1, "row15/minus1");
        check(&c, &r, count, -(count as i32), "row15/minus-count");
        check(&c, &r, count, -(count as i32) / 2, "row15/straddle");
    }

    // ---- row 16: fully random ------------------------------------------
    for i in 0..1000 {
        let count = rng.below(65) as usize; // 0..=64
        let init = rng.interesting_i32();
        check(&c, &r, count, init, &format!("row16/rand{i}"));
    }

    // ---- ERRORS #2: calloc nmemb*size overflow -------------------------
    for &count in &[
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 4 + 1,
        usize::MAX / 4 + 2,
        (1usize << 63) + 1,
        usize::MAX / 3,
    ] {
        let a = unsafe { (c.allocate_block)(count, 1) };
        let b = unsafe { (r.allocate_block)(count, 1) };
        assert!(
            a.is_null(),
            "ERRORS#2: C allocate_block({count}) should fail"
        );
        assert_eq!(
            a.is_null(),
            b.is_null(),
            "ERRORS#2: NULL-ness mismatch for count={count} (C null={}, Rust null={})",
            a.is_null(),
            b.is_null()
        );
    }

    // ---- ERRORS #7: the right operand of `!mem1 || !mem2` ---------------
    // `betagamma` always passes the same `block_size` to both `allocate_block`
    // calls, so the two can never fail independently through the public API.
    // The *code* of that branch is therefore exercised here directly: a
    // successful first block, a NULL second block, then the exact cleanup
    // sequence `free_block(mem1); free_block(mem2);` the C code performs.
    let (ca, ra) = fork_pair(|which, buf| {
        let imp = if which { &r } else { &c };
        unsafe {
            let mem1 = (imp.allocate_block)(6, 42);
            let mem2 = (imp.allocate_block)(usize::MAX, 42); // fails
            let mut flags = 0u8;
            if mem1.is_null() {
                flags |= 1;
            }
            if mem2.is_null() {
                flags |= 2;
            }
            // C: if (!mem1 || !mem2) { free_block(mem1); free_block(mem2); return -1; }
            let ret: i32 = if mem1.is_null() || mem2.is_null() {
                (imp.free_block)(mem1);
                (imp.free_block)(mem2);
                -1
            } else {
                0
            };
            buf[0] = flags;
            buf[1..5].copy_from_slice(&ret.to_ne_bytes());
        }
        5
    });
    assert_eq!(
        ca.exit_code(),
        Some(0),
        "ERRORS#7: C cleanup path crashed: {}",
        ca.describe()
    );
    assert_eq!(
        ca.bytes[0], 2,
        "ERRORS#7: expected mem1 non-NULL and mem2 NULL, flags={}",
        ca.bytes[0]
    );
    assert_eq!(
        &ca.bytes[1..5],
        &(-1i32).to_ne_bytes(),
        "ERRORS#7: the branch must yield -1"
    );
    assert_eq!(
        ca, ra,
        "ERRORS#7: cleanup path differs: C={} Rust={}",
        ca.describe(),
        ra.describe()
    );

    // ---- ERRORS #17: no enum parameters exist in this API ---------------
    // `grep -n enum c_src/include/lib.h c_src/src/lib.c` finds nothing: every
    // parameter is `int`, `size_t` or `uint8_t`, so there is no "value with no
    // valid variant".  The analogous FFI hazard is a bit pattern outside the
    // range a caller would normally produce; sweep those through the `size_t`
    // slot (the `uint8_t` slot is ERRORS #16, the `int` slots are covered by
    // the full-range betagamma sweep).
    for &count in &[
        1usize << 63,               // high bit set
        (1usize << 63) | 1,
        usize::MAX,                 // "-1" as size_t
        usize::MAX - 3,
        0x8000_0000usize,           // just past u32 range boundary
        0xFFFF_FFFFusize,
        0x1_0000_0000usize,
    ] {
        let a = unsafe { (c.allocate_block)(count, -7) };
        let b = unsafe { (r.allocate_block)(count, -7) };
        assert_eq!(
            a.is_null(),
            b.is_null(),
            "ERRORS#17: NULL-ness mismatch for out-of-domain count={count:#x}"
        );
        if !a.is_null() {
            unsafe {
                assert_eq!((*a).size, (*b).size, "ERRORS#17: size mismatch");
                (c.free_block)(a);
                (r.free_block)(b);
            }
        }
    }

    // ---- ERRORS #3: huge but non-overflowing count ----------------------
    for &count in &[1usize << 60, 1usize << 55, 1usize << 50, 1usize << 45] {
        let a = unsafe { (c.allocate_block)(count, 1) };
        let b = unsafe { (r.allocate_block)(count, 1) };
        assert!(
            a.is_null(),
            "ERRORS#3: C allocate_block({count}) unexpectedly succeeded"
        );
        assert_eq!(
            a.is_null(),
            b.is_null(),
            "ERRORS#3: NULL-ness mismatch for count={count}"
        );
    }

    // ---- row 17 / ERRORS #4: free_block(NULL) --------------------------
    // Fork-isolated so that a missing NULL guard shows up as a signal
    // difference instead of killing the test runner.
    let (ca, ra) = fork_pair(|which, buf| {
        let imp = if which { &r } else { &c };
        unsafe {
            for _ in 0..4 {
                (imp.free_block)(core::ptr::null_mut());
            }
        }
        buf[0] = 0x5A;
        1
    });
    assert_eq!(
        ca.exit_code(),
        Some(0),
        "ERRORS#4: C free_block(NULL) must be a no-op, got {}",
        ca.describe()
    );
    assert_eq!(
        ca, ra,
        "ERRORS#4: free_block(NULL) differs: C={} Rust={}",
        ca.describe(),
        ra.describe()
    );
    // and again in-process, to prove there is no hidden state
    unsafe {
        (c.free_block)(core::ptr::null_mut());
        (r.free_block)(core::ptr::null_mut());
    }

    // ---- row 18 / ERRORS #5: free_block with data == NULL ---------------
    // Run in forked children: if either implementation tried to `free(NULL)`
    // incorrectly or double-freed, glibc would abort the child, not the runner.
    let (ca, ra) = fork_pair(|which, buf| {
        let imp = if which { &r } else { &c };
        unsafe {
            for size in [0usize, 1, 7, usize::MAX] {
                let mb = malloc(core::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
                (*mb).data = core::ptr::null_mut();
                (*mb).size = size;
                (imp.free_block)(mb);
            }
        }
        buf[0] = 0xA5;
        1
    });
    assert_eq!(
        ca.exit_code(),
        Some(0),
        "ERRORS#5: C free_block(data=NULL) failed: {}",
        ca.describe()
    );
    assert_eq!(
        ca, ra,
        "ERRORS#5: free_block(data=NULL) differs: C={} Rust={}",
        ca.describe(),
        ra.describe()
    );

    // ---- row 44 / ERRORS #18: cross-library allocate/free ---------------
    // Proves both libraries share the platform allocator; otherwise glibc
    // aborts with "free(): invalid pointer".
    let (ca, ra) = fork_pair(|which, buf| {
        // which == false: C allocates, Rust frees.  which == true: the reverse.
        let (alloc, freer) = if which { (&r, &c) } else { (&c, &r) };
        unsafe {
            let mut acc: i64 = 0;
            for count in [0usize, 1, 5, 13, 4096] {
                let p = (alloc.allocate_block)(count, 3);
                assert!(!p.is_null());
                for i in 0..(*p).size {
                    acc += *(*p).data.add(i) as i64;
                }
                (freer.free_block)(p);
            }
            buf[..8].copy_from_slice(&acc.to_ne_bytes());
        }
        8
    });
    assert_eq!(
        ca.exit_code(),
        Some(0),
        "row44: C-alloc/Rust-free failed: {}",
        ca.describe()
    );
    assert_eq!(
        ra.exit_code(),
        Some(0),
        "row44: Rust-alloc/C-free failed: {}",
        ra.describe()
    );
    assert_eq!(
        ca.bytes, ra.bytes,
        "row44: cross-library contents differ: C={} Rust={}",
        ca.describe(),
        ra.describe()
    );

    // ---- ERRORS #1: malloc(sizeof(MemoryBlock)) itself returns NULL -----
    // Forced by clamping RLIMIT_AS in the child and draining the heap, so the
    // very first `malloc` inside `allocate_block` fails and `calloc` is never
    // reached.
    let limit = vm_size_bytes() + 64 * 1024 * 1024;
    let (ca, ra) = fork_pair(|which, buf| {
        let imp = if which { &r } else { &c };
        unsafe {
            let rl = Rlimit {
                cur: limit,
                max: limit,
            };
            if setrlimit(RLIMIT_AS, &rl) != 0 {
                buf[0] = 0xEE; // could not set the limit
                return 1;
            }
            // Drain the address space, then the existing heap, from coarse to
            // fine.  Nothing is freed and nothing is recorded, so this loop
            // performs no Rust-side allocation.
            for &sz in &[1usize << 20, 1 << 16, 4096, 256, 64, 16] {
                let mut guard = 0u64;
                loop {
                    // `black_box` is essential: without it LLVM's
                    // `isAllocSiteRemovable` deletes a `malloc` whose result is
                    // only compared against NULL, and the drain becomes a no-op
                    // in optimised builds.
                    let p = core::hint::black_box(malloc(core::hint::black_box(sz)));
                    if p.is_null() {
                        break;
                    }
                    guard += 1;
                    if guard > 2_000_000 {
                        break;
                    }
                }
            }
            let p = (imp.allocate_block)(4, 9);
            buf[0] = if p.is_null() { 1 } else { 0 };
        }
        1
    });
    assert_eq!(
        ca.exit_code(),
        Some(0),
        "ERRORS#1: C child failed: {}",
        ca.describe()
    );
    assert_eq!(
        ca.bytes,
        vec![1u8],
        "ERRORS#1: could not force malloc failure in the C library: {}",
        ca.describe()
    );
    assert_eq!(
        ca, ra,
        "ERRORS#1: malloc-failure behaviour differs: C={} Rust={}",
        ca.describe(),
        ra.describe()
    );

    // silence the unused-import warning for c_void when nothing else uses it
    let _ = core::ptr::null_mut::<c_void>();
}
