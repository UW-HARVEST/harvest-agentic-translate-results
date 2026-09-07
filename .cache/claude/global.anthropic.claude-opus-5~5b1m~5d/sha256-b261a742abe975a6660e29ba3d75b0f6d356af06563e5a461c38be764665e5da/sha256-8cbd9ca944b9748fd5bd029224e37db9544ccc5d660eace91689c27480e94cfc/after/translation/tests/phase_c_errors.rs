//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `merge_sort` has no error channel (it returns `void`), so "same error" means
//! "same observable rejection": either both implementations leave memory in the
//! same state and return, or both die from the same fatal signal. The fatal
//! rows use `fork()` + `waitpid()` so the exact signal number is compared, not
//! merely "both failed somehow".

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — size == 0, valid non-null buffers: rejected by the `hi-lo<=1` guard.
// ---------------------------------------------------------------------------
#[test]
fn err01_size_zero_valid_pointers_is_a_noop() {
    let mut rng = Rng::new(0xE001);
    for iter in 0..500 {
        let mut a = Arena::new(4 * ELEM_SIZE);
        let mut b = Arena::new(4 * ELEM_SIZE);
        a.fill_random(&mut rng);
        b.fill_random(&mut rng);
        let before_a = a.bytes().to_vec();
        let before_b = b.bytes().to_vec();

        let (mut ac, mut bc) = (a.clone(), b.clone());
        let (mut ar, mut br) = (a.clone(), b.clone());
        unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), 0) };
        unsafe { rust_merge_sort()(ar.ptr(), br.ptr(), 0) };

        cmp_arena(&format!("err01/iter={iter}"), "a", &ac, &ar);
        cmp_arena(&format!("err01/iter={iter}"), "b", &bc, &br);
        // and the shared, exact expectation: nothing was written at all.
        assert_eq!(ac.bytes(), &before_a[..], "err01: C wrote to `a`");
        assert_eq!(bc.bytes(), &before_b[..], "err01: C wrote to `b`");
        assert_eq!(ar.bytes(), &before_a[..], "err01: Rust wrote to `a`");
        assert_eq!(br.bytes(), &before_b[..], "err01: Rust wrote to `b`");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — size == 0 with NULL pointers: must return normally, no fault.
// ---------------------------------------------------------------------------
#[test]
fn err02_size_zero_null_pointers_returns_normally() {
    // Run in children so that a (wrong) fault is observed rather than aborting
    // the test process, and so the two are compared on equal footing.
    diff_fatal("err02/null+null/size=0", std::ptr::null_mut(), std::ptr::null_mut(), 0);
    // And the expected outcome specifically: clean exit for BOTH.
    let cf = c_merge_sort();
    let rf = rust_merge_sort();
    assert_eq!(
        run_in_child(move || unsafe { cf(std::ptr::null_mut(), std::ptr::null_mut(), 0) }),
        ChildOutcome::Exited(0),
        "err02: C did not return normally for size=0 with NULL pointers"
    );
    assert_eq!(
        run_in_child(move || unsafe { rf(std::ptr::null_mut(), std::ptr::null_mut(), 0) }),
        ChildOutcome::Exited(0),
        "err02: Rust did not return normally for size=0 with NULL pointers"
    );

    // Mixed null / non-null with size 0 is equally harmless.
    let mut arena = Arena::new(2 * ELEM_SIZE);
    let mut rng = Rng::new(0xE002);
    arena.fill_random(&mut rng);
    let p = arena.ptr();
    diff_fatal("err02/null+valid/size=0", std::ptr::null_mut(), p, 0);
    diff_fatal("err02/valid+null/size=0", p, std::ptr::null_mut(), 0);
}

// ---------------------------------------------------------------------------
// Row 3 — size == 1: memcpy only, recursion rejected; element 1 never touched.
// ---------------------------------------------------------------------------
#[test]
fn err03_size_one_copies_only_one_element() {
    let mut rng = Rng::new(0xE003);
    for iter in 0..500 {
        // 4 elements of slack; only element 0 may be touched.
        let mut a = Arena::new(4 * ELEM_SIZE);
        let mut b = Arena::new(4 * ELEM_SIZE);
        a.fill_random(&mut rng);
        b.fill_random(&mut rng);
        let before_a = a.bytes().to_vec();
        let before_b = b.bytes().to_vec();

        let (mut ac, mut bc) = (a.clone(), b.clone());
        let (mut ar, mut br) = (a.clone(), b.clone());
        unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), 1) };
        unsafe { rust_merge_sort()(ar.ptr(), br.ptr(), 1) };

        cmp_arena(&format!("err03/iter={iter}"), "a", &ac, &ar);
        cmp_arena(&format!("err03/iter={iter}"), "b", &bc, &br);
        // exact expected C effect: `a` untouched, `b[0] == a[0]`, rest of `b`
        // untouched.
        assert_eq!(ac.bytes(), &before_a[..], "err03: `a` must be untouched");
        assert_eq!(&bc.bytes()[..ELEM_SIZE], &before_a[..ELEM_SIZE], "err03: b[0] != a[0]");
        assert_eq!(
            &bc.bytes()[ELEM_SIZE..],
            &before_b[ELEM_SIZE..],
            "err03: b[1..] must be untouched"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 4 — `hi - lo <= 1` recursion guard, hit at every leaf.
//
// The guard is only observable through the composed result, so this test drives
// sizes where leaves dominate (1, 2, 3) and additionally asserts that no byte
// outside `[0, size)` is ever written, which is what the guard guarantees.
// ---------------------------------------------------------------------------
#[test]
fn err04_recursion_leaf_guard() {
    let mut rng = Rng::new(0xE004);
    for size in 0..=3i32 {
        for iter in 0..300 {
            let n = size as usize;
            let mut a = Arena::new((n + 4) * ELEM_SIZE);
            let mut b = Arena::new((n + 4) * ELEM_SIZE);
            a.fill_random(&mut rng);
            b.fill_random(&mut rng);
            for i in 0..n {
                a.write_fields(i, rng.next_u64() % 4, (rng.next_u32() % 3) as i32);
            }
            let tail_a = a.bytes()[n * ELEM_SIZE..].to_vec();
            let tail_b = b.bytes()[n * ELEM_SIZE..].to_vec();

            let (mut ac, mut bc) = (a.clone(), b.clone());
            let (mut ar, mut br) = (a.clone(), b.clone());
            unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), size) };
            unsafe { rust_merge_sort()(ar.ptr(), br.ptr(), size) };

            let lbl = format!("err04/size={size}/iter={iter}");
            cmp_arena(&lbl, "a", &ac, &ar);
            cmp_arena(&lbl, "b", &bc, &br);
            assert_eq!(&ac.bytes()[n * ELEM_SIZE..], &tail_a[..], "{lbl}: C overran `a`");
            assert_eq!(&bc.bytes()[n * ELEM_SIZE..], &tail_b[..], "{lbl}: C overran `b`");
            assert_eq!(&ar.bytes()[n * ELEM_SIZE..], &tail_a[..], "{lbl}: Rust overran `a`");
            assert_eq!(&br.bytes()[n * ELEM_SIZE..], &tail_b[..], "{lbl}: Rust overran `b`");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — the predicate's `return 0` path, and the DEAD second `if`.
//
// The predicate is `static`, so it is probed through `merge_sort` with size 2:
// for a 2-element input the output order is a direct read-out of the predicate.
// ---------------------------------------------------------------------------
#[test]
fn err05_predicate_reject_path_and_dead_branch() {
    let mut rng = Rng::new(0xE005);

    // (a) `a->sort_bits > b->sort_bits` -> predicate returns 0 -> swap.
    for iter in 0..300 {
        let lo = -1_000_000 + (rng.next_u32() % 2_000_000) as i32;
        let hi = lo + 1 + (rng.next_u32() % 1000) as i32;
        let sprites = [
            Sprite { texture_id: rng.next_u64(), sort_bits: hi, _pad: 0 },
            Sprite { texture_id: rng.next_u64(), sort_bits: lo, _pad: 0 },
        ];
        let mut a = Arena::new(2 * ELEM_SIZE);
        a.write_sprites(&sprites);
        let mut b = Arena::new(2 * ELEM_SIZE);
        b.fill_random(&mut rng);
        diff_run(&format!("err05a/iter={iter}"), &a, &b, 2);

        // exact expected C result: the greater key ends up last in `a`.
        let mut ac = a.clone();
        let mut bc = b.clone();
        unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), 2) };
        let out: [Sprite; 2] = unsafe { std::ptr::read(ac.ptr() as *const [Sprite; 2]) };
        assert_eq!(out[0].sort_bits, lo, "err05a: predicate reject path misordered");
        assert_eq!(out[1].sort_bits, hi, "err05a: predicate reject path misordered");
    }

    // (b) DEAD second `if`: with equal keys the first `if` already returns 1, so
    // `texture_id` must NEVER influence the order. Verified by checking that
    // swapping only the texture ids does not swap the output order, in BOTH
    // implementations.
    for iter in 0..300 {
        let sb = rng.next_i32();
        let (t_small, t_big) = {
            let (x, y) = (rng.next_u64(), rng.next_u64());
            if x <= y { (x, y) } else { (y, x) }
        };
        for (first, second) in [(t_small, t_big), (t_big, t_small)] {
            let sprites = [
                Sprite { texture_id: first, sort_bits: sb, _pad: 0 },
                Sprite { texture_id: second, sort_bits: sb, _pad: 0 },
            ];
            let mut a = Arena::new(2 * ELEM_SIZE);
            a.write_sprites(&sprites);
            let mut b = Arena::new(2 * ELEM_SIZE);
            b.fill_random(&mut rng);
            let lbl = format!("err05b/iter={iter}/{first}-{second}");
            diff_run(&lbl, &a, &b, 2);

            let (mut ac, mut bc) = (a.clone(), b.clone());
            unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), 2) };
            let out: [Sprite; 2] = unsafe { std::ptr::read(ac.ptr() as *const [Sprite; 2]) };
            assert_eq!(
                (out[0].texture_id, out[1].texture_id),
                (first, second),
                "{lbl}: texture_id affected ordering, but the second `if` is dead code"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — extreme signed operands in the predicate (no overflow: plain `<=`).
// ---------------------------------------------------------------------------
#[test]
fn err06_predicate_extreme_signed_operands() {
    const KEYS: [i32; 6] = [i32::MIN, i32::MIN + 1, -1, 0, i32::MAX - 1, i32::MAX];
    let mut rng = Rng::new(0xE006);
    for &x in &KEYS {
        for &y in &KEYS {
            let sprites = [
                Sprite { texture_id: u64::MAX, sort_bits: x, _pad: 0 },
                Sprite { texture_id: 0, sort_bits: y, _pad: 0 },
            ];
            let mut a = Arena::new(2 * ELEM_SIZE);
            a.write_sprites(&sprites);
            let mut b = Arena::new(2 * ELEM_SIZE);
            b.fill_random(&mut rng);
            let lbl = format!("err06/{x}-vs-{y}");
            diff_run(&lbl, &a, &b, 2);

            // exact expected C result: `<=` on the signed keys, no wraparound.
            let (mut ac, mut bc) = (a.clone(), b.clone());
            unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), 2) };
            let out: [Sprite; 2] = unsafe { std::ptr::read(ac.ptr() as *const [Sprite; 2]) };
            let expected = if x <= y { (x, y) } else { (y, x) };
            assert_eq!(
                (out[0].sort_bits, out[1].sort_bits),
                expected,
                "{lbl}: signed comparison overflowed"
            );
        }
    }
    // A larger extreme-value sweep, still fully differential.
    for n in 2..=20usize {
        for iter in 0..30 {
            let sprites: Vec<Sprite> = (0..n)
                .map(|_| Sprite {
                    texture_id: rng.next_u64(),
                    sort_bits: KEYS[rng.below(KEYS.len())],
                    _pad: 0,
                })
                .collect();
            let mut a = Arena::new(n * ELEM_SIZE);
            a.write_sprites(&sprites);
            let mut b = Arena::new(n * ELEM_SIZE);
            b.fill_random(&mut rng);
            diff_run(&format!("err06/sweep/size={n}/iter={iter}"), &a, &b, n as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7 / 11 / 12 — pathological `size` values (negative, INT_MIN, huge).
//
// These make the C compute `16 * (size_t)size`, an astronomically large byte
// count, and hand it to `memcpy`. A plain "did it crash?" comparison is NOT a
// valid differential here: it was measured to depend on the process
// address-space layout (identical `size`, different buffer address -> the C
// alone flips between `Exited(0)` and `SIGSEGV`), and the C child and the Rust
// child necessarily have different layouts. See ERRORS.md.
//
// `diff_guarded` removes that dependency by running both implementations on the
// SAME addresses, inside a small read/write window surrounded by a 256 MiB
// PROT_NONE reservation, with a SIGSEGV handler capturing `si_addr`. It then
// compares three observables: termination outcome, the EXACT faulting address,
// and every byte of the window. Equal fault addresses mean the two
// implementations passed identical `(dst, src, n)` to the same `memcpy` — which
// is exactly the translated logic under test.
// ---------------------------------------------------------------------------

/// The window is one page = 256 sprites; `b` is placed at element 128.
const B_IDX: usize = 128;

#[test]
fn err07_negative_size() {
    let mut rng = Rng::new(0xE007);
    let mut ran = 0;
    for size in [
        -1i32, -2, -3, -4, -7, -8, -15, -16, -17, -31, -32, -100, -127, -128, -255, -256, -257,
        -1000, -4096, -65535, -65536, -1 << 20, -(1 << 24),
    ] {
        if diff_guarded(&format!("err07/size={size}"), size, B_IDX, &mut rng) {
            ran += 1;
        }
    }
    // Random negative values across the whole negative half of `int`.
    for _ in 0..64 {
        let size = -1 - ((rng.next_u32() % (i32::MAX as u32)) as i32);
        if diff_guarded(&format!("err07/random/size={size}"), size, B_IDX, &mut rng) {
            ran += 1;
        }
    }
    assert!(ran > 0, "err07: every guarded reservation failed; row not covered");

    // Non-vacuity: the C really does fault (inside our guard) for a negative
    // size, so the equality assertions above are comparing real behaviour.
    let g = GuardedArena::new().expect("guarded reservation");
    let (c_out, c_addr, r_out, r_addr) = guarded_detail(&g, -1, B_IDX, &mut rng);
    assert_eq!(c_out, ChildOutcome::Exited(42), "err07: C did not fault for size=-1");
    assert_eq!(r_out, ChildOutcome::Exited(42), "err07: Rust did not fault for size=-1");
    assert_ne!(c_addr, u64::MAX, "err07: no fault address captured for C");
    assert_eq!(c_addr, r_addr, "err07: fault addresses differ for size=-1");
}

#[test]
fn err11_int_min_size() {
    let mut rng = Rng::new(0xE011);
    let mut ran = 0;
    for size in [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 16, i32::MIN / 2] {
        if diff_guarded(&format!("err11/size={size}"), size, B_IDX, &mut rng) {
            ran += 1;
        }
    }
    assert!(ran > 0, "err11: every guarded reservation failed; row not covered");

    let g = GuardedArena::new().expect("guarded reservation");
    let (c_out, c_addr, r_out, r_addr) = guarded_detail(&g, i32::MIN, B_IDX, &mut rng);
    assert_eq!(c_out, ChildOutcome::Exited(42), "err11: C did not fault for size=INT_MIN");
    assert_eq!(r_out, ChildOutcome::Exited(42), "err11: Rust did not fault for size=INT_MIN");
    assert_eq!(c_addr, r_addr, "err11: fault addresses differ for size=INT_MIN");
}

#[test]
fn err12_arbitrary_int_argument() {
    let mut rng = Rng::new(0xE012);
    let mut ran = 0;

    // Huge positive sizes -> the copy runs past the window into the guard.
    for size in [
        i32::MAX,
        i32::MAX - 1,
        1 << 30,
        1 << 24,
        1 << 20,
        100_000_000,
        65537,
        65536,
        4097,
        4096,
        1000,
        513,
        512,
        257,
        256,
        255,
    ] {
        if diff_guarded(&format!("err12/size={size}"), size, B_IDX, &mut rng) {
            ran += 1;
        }
    }
    // The entire `i32` domain is a legal FFI argument — exactly the
    // "out-of-range enum value" class. Sample it broadly.
    for _ in 0..128 {
        let size = rng.next_i32();
        if diff_guarded(&format!("err12/random/size={size}"), size, B_IDX, &mut rng) {
            ran += 1;
        }
    }
    // Powers of two and their neighbours, positive and negative.
    for shift in 0..31u32 {
        for delta in [-1i64, 0, 1] {
            let v = (1i64 << shift) + delta;
            for signed in [v, -v] {
                if signed >= i32::MIN as i64 && signed <= i32::MAX as i64 {
                    let size = signed as i32;
                    if diff_guarded(&format!("err12/pow2/size={size}"), size, B_IDX, &mut rng) {
                        ran += 1;
                    }
                }
            }
        }
    }
    assert!(ran > 0, "err12: every guarded reservation failed; row not covered");

    // Values one step past the *valid* range for a real 8-element buffer, on
    // ordinary heap memory (deterministic because they stay in bounds of the
    // 64-element allocation).
    let mut a = Arena::new(64 * ELEM_SIZE);
    let mut b = Arena::new(64 * ELEM_SIZE);
    a.fill_random(&mut rng);
    b.fill_random(&mut rng);
    for size in [0i32, 1, 7, 8, 9, 10, 63, 64] {
        diff_run(&format!("err12/boundary/size={size}"), &a, &b, size);
    }

    // Non-vacuity for the huge-positive regime.
    let g = GuardedArena::new().expect("guarded reservation");
    let (c_out, c_addr, r_out, r_addr) = guarded_detail(&g, i32::MAX, B_IDX, &mut rng);
    assert_eq!(c_out, ChildOutcome::Exited(42), "err12: C did not fault for size=INT_MAX");
    assert_eq!(r_out, ChildOutcome::Exited(42), "err12: Rust did not fault for size=INT_MAX");
    assert_eq!(c_addr, r_addr, "err12: fault addresses differ for size=INT_MAX");
}

// ---------------------------------------------------------------------------
// Row 8 — NULL pointer(s) with a non-zero size.
// ---------------------------------------------------------------------------
#[test]
fn err08_null_pointers_with_nonzero_size_are_fatal_in_both() {
    let mut arena = Arena::new(8 * ELEM_SIZE);
    let mut rng = Rng::new(0xE008);
    arena.fill_random(&mut rng);
    let p = arena.ptr();
    let null = std::ptr::null_mut::<Sprite>();
    for size in [1i32, 2, 3, 8] {
        diff_fatal(&format!("err08/null+null/size={size}"), null, null, size);
        diff_fatal(&format!("err08/valid+null/size={size}"), p, null, size);
        diff_fatal(&format!("err08/null+valid/size={size}"), null, p, size);
        // and the C really does die, so these are not vacuously-equal passes.
        let cf = c_merge_sort();
        let out = run_in_child(move || unsafe { cf(null, null, size) });
        assert!(
            matches!(out, ChildOutcome::Signaled(_)),
            "err08/size={size}: C was expected to die from a signal, got {out:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 9 — size larger than the caller's logical element count.
//
// Run inside a deliberately over-allocated arena so the out-of-range accesses
// stay inside mapped, initialised memory and the result is deterministic; the
// two implementations must then agree byte-for-byte over the whole arena.
// ---------------------------------------------------------------------------
#[test]
fn err09_oversized_size_within_overallocated_arena() {
    let mut rng = Rng::new(0xE009);
    const CAP: usize = 128;
    for &(logical, size) in &[(0usize, 8i32), (1, 8), (4, 64), (4, 128), (7, 33), (0, 128)] {
        for iter in 0..20 {
            let mut a = Arena::new(CAP * ELEM_SIZE);
            let mut b = Arena::new(CAP * ELEM_SIZE);
            a.fill_random(&mut rng);
            b.fill_random(&mut rng);
            for i in 0..logical {
                a.write_fields(i, rng.next_u64(), (rng.next_u32() % 5) as i32);
            }
            diff_run(
                &format!("err09/logical={logical}/size={size}/iter={iter}"),
                &a,
                &b,
                size,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — aliased buffers, `a == b`.
// ---------------------------------------------------------------------------
#[test]
fn err10_aliased_buffers() {
    let mut rng = Rng::new(0xE010);
    for n in 0..=24usize {
        for iter in 0..25 {
            let mut a = Arena::new((n + 3) * ELEM_SIZE);
            a.fill_random(&mut rng);
            for i in 0..n {
                let key = if iter % 2 == 0 { rng.next_i32() } else { (rng.next_u32() % 3) as i32 };
                a.write_fields(i, rng.next_u64(), key);
            }
            diff_run_aliased(&format!("err10/size={n}/iter={iter}"), &a, n as i32);
        }
    }
}
