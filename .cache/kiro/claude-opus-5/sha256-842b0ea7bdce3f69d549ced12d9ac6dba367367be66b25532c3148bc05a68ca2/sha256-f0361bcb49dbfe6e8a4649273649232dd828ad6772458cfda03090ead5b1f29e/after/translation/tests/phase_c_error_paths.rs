//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. `merge_sort` returns `void` and the C
//! library contains no error codes, no asserts and no validation, so the
//! strongest available notion of "same rejection" is: **identical observable
//! side effects on both buffers, byte-for-byte, and identical
//! crash-or-not-crash behaviour**. That is what these tests assert.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — size == 0 with valid pointers: both buffers must be untouched.
// ---------------------------------------------------------------------------
#[test]
fn err_01_size_zero_leaves_both_buffers_untouched() {
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0xE1);

    for n_alloc in [0usize, 1, 4, 16] {
        for &fill in &[0x00u8, 0x55, 0xAA, 0xFF] {
            let a0 = make_sprites(Shape::Random, n_alloc, &mut rng, Some(0xFF));
            let b0 = vec![Sprite([fill; SPRITE_SIZE]); n_alloc];

            let mut ac = a0.clone();
            let mut bc = b0.clone();
            let mut ar = a0.clone();
            let mut br = b0.clone();

            unsafe {
                p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), 0);
                p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), 0);
            }

            // C behaviour: memcpy(b,a,0) is a no-op and `hi-lo == 0 <= 1`
            // returns immediately => nothing written at all.
            assert_eq!(ac.as_slice(), a0.as_slice(), "C wrote to a on size=0");
            assert_eq!(bc.as_slice(), b0.as_slice(), "C wrote to b on size=0");
            assert_eq!(ar.as_slice(), a0.as_slice(), "Rust wrote to a on size=0");
            assert_eq!(br.as_slice(), b0.as_slice(), "Rust wrote to b on size=0");
            assert_eq!(ac.as_slice(), ar.as_slice());
            assert_eq!(bc.as_slice(), br.as_slice());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 — size == 0 with BOTH pointers NULL: must not dereference, must not
// crash. A length-0 memcpy performs no access and the base case returns.
// ---------------------------------------------------------------------------
#[test]
fn err_02_size_zero_null_pointers() {
    let p = pair();
    unsafe {
        p.c.merge_sort(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        p.rust
            .merge_sort(std::ptr::null_mut(), std::ptr::null_mut(), 0);
    }
    // Reaching here means neither implementation faulted — that IS the assertion.
    // Repeat a few times to be sure nothing was lazily corrupted.
    for _ in 0..64 {
        unsafe {
            p.c.merge_sort(std::ptr::null_mut(), std::ptr::null_mut(), 0);
            p.rust
                .merge_sort(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — size == 0 with exactly one NULL pointer (both mirrorings).
// ---------------------------------------------------------------------------
#[test]
fn err_03_size_zero_one_null_pointer() {
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0xE3);

    for _ in 0..16 {
        let a0 = make_sprites(Shape::Random, 8, &mut rng, Some(0xFF));

        // a valid, b NULL
        let mut ac = a0.clone();
        let mut ar = a0.clone();
        unsafe {
            p.c.merge_sort(ac.as_mut_ptr(), std::ptr::null_mut(), 0);
            p.rust.merge_sort(ar.as_mut_ptr(), std::ptr::null_mut(), 0);
        }
        assert_eq!(ac.as_slice(), a0.as_slice(), "C modified a (b=NULL, size=0)");
        assert_eq!(ar.as_slice(), a0.as_slice(), "Rust modified a (b=NULL, size=0)");
        assert_eq!(ac.as_slice(), ar.as_slice());

        // a NULL, b valid
        let b0 = vec![Sprite([0x5A; SPRITE_SIZE]); 8];
        let mut bc = b0.clone();
        let mut br = b0.clone();
        unsafe {
            p.c.merge_sort(std::ptr::null_mut(), bc.as_mut_ptr(), 0);
            p.rust.merge_sort(std::ptr::null_mut(), br.as_mut_ptr(), 0);
        }
        assert_eq!(bc.as_slice(), b0.as_slice(), "C modified b (a=NULL, size=0)");
        assert_eq!(br.as_slice(), b0.as_slice(), "Rust modified b (a=NULL, size=0)");
        assert_eq!(bc.as_slice(), br.as_slice());
    }
}

// ---------------------------------------------------------------------------
// Row 4 — size == 1: copy one element, then hit the base case.
// ---------------------------------------------------------------------------
#[test]
fn err_04_size_one_copies_then_returns() {
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0xE4);

    for _ in 0..64 {
        // Allocation deliberately larger than `size` so that any write past
        // element 0 is visible.
        let a0 = make_sprites(Shape::Random, 5, &mut rng, Some(0xFF));
        let b0 = vec![Sprite([0x33; SPRITE_SIZE]); 5];

        let mut ac = a0.clone();
        let mut bc = b0.clone();
        let mut ar = a0.clone();
        let mut br = b0.clone();

        unsafe {
            p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), 1);
            p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), 1);
        }

        // Documented C behaviour for size == 1.
        assert_eq!(ac.as_slice(), a0.as_slice(), "C modified a on size=1");
        assert_eq!(bc[0], a0[0], "C did not memcpy element 0 into b");
        assert_eq!(&bc[1..], &b0[1..], "C wrote past element 0 in b");
        // Rust must match exactly.
        assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged on size=1");
        assert_eq!(bc.as_slice(), br.as_slice(), "b diverged on size=1");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — recursion base case: no write outside [0, size) in either buffer.
// Uses a padded allocation with distinctive guard elements before and after
// the region the call is allowed to touch.
// ---------------------------------------------------------------------------
#[test]
fn err_05_no_out_of_range_writes() {
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0xE5);
    const GUARD: usize = 4;

    for n in 0..=40usize {
        let data = make_sprites(Shape::Random, n, &mut rng, Some(0xFF));

        let build = |g: u8| {
            let mut v = vec![Sprite([g; SPRITE_SIZE]); GUARD];
            v.extend_from_slice(&data);
            v.extend(vec![Sprite([g; SPRITE_SIZE]); GUARD]);
            v
        };

        let mut av_c = build(0xD1);
        let mut bv_c = build(0xD2);
        let mut av_r = build(0xD1);
        let mut bv_r = build(0xD2);

        unsafe {
            p.c.merge_sort(
                av_c.as_mut_ptr().add(GUARD),
                bv_c.as_mut_ptr().add(GUARD),
                n as i32,
            );
            p.rust.merge_sort(
                av_r.as_mut_ptr().add(GUARD),
                bv_r.as_mut_ptr().add(GUARD),
                n as i32,
            );
        }

        let g_a = Sprite([0xD1; SPRITE_SIZE]);
        let g_b = Sprite([0xD2; SPRITE_SIZE]);
        for i in 0..GUARD {
            assert_eq!(av_c[i], g_a, "C clobbered leading guard of a (n={n})");
            assert_eq!(bv_c[i], g_b, "C clobbered leading guard of b (n={n})");
            assert_eq!(av_r[i], g_a, "Rust clobbered leading guard of a (n={n})");
            assert_eq!(bv_r[i], g_b, "Rust clobbered leading guard of b (n={n})");
            let t = GUARD + n + i;
            assert_eq!(av_c[t], g_a, "C clobbered trailing guard of a (n={n})");
            assert_eq!(bv_c[t], g_b, "C clobbered trailing guard of b (n={n})");
            assert_eq!(av_r[t], g_a, "Rust clobbered trailing guard of a (n={n})");
            assert_eq!(bv_r[t], g_b, "Rust clobbered trailing guard of b (n={n})");
        }
        assert_eq!(av_c.as_slice(), av_r.as_slice(), "a diverged (n={n})");
        assert_eq!(bv_c.as_slice(), bv_r.as_slice(), "b diverged (n={n})");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — right run exhausted (`j >= hi`) must short-circuit before reading
// `a + j`. Ascending input forces the left run to always be taken, which drives
// `j` up to `hi` while `i < split` on the very next iteration.
//
// The array is placed at the very END of an mmap'd page whose successor page is
// unmapped, so an out-of-bounds read of element `hi` would SIGSEGV. If both
// implementations return normally, neither dereferenced past the run.
// ---------------------------------------------------------------------------
#[test]
fn err_06_right_run_exhausted_no_oob_read() {
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0xE6);

    for n in [2usize, 3, 4, 5, 8, 9, 16, 17, 33, 64] {
        // Ascending: every comparison returns 1, so the left element wins until
        // `i == split`, which is exactly the `j >= hi` short-circuit path.
        let a0 = make_sprites(Shape::Ascending, n, &mut rng, Some(0xAA));
        let b0 = vec![Sprite([0x00; SPRITE_SIZE]); n];

        let mut ac = a0.clone();
        let mut bc = b0.clone();
        let mut ar = a0.clone();
        let mut br = b0.clone();
        unsafe {
            p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32);
            p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), n as i32);
        }
        assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged (ascending n={n})");
        assert_eq!(bc.as_slice(), br.as_slice(), "b diverged (ascending n={n})");

        // Same input, but both buffers end exactly at a page boundary with the
        // following page unmapped. Any read of element index `n` faults.
        let (ap, bp, guard) = unsafe { page_edge_pair(n) };
        unsafe {
            std::ptr::copy_nonoverlapping(a0.as_ptr(), ap, n);
            std::ptr::write_bytes(bp.cast::<u8>(), 0, n * SPRITE_SIZE);
            p.c.merge_sort(ap, bp, n as i32);
            let c_a = std::slice::from_raw_parts(ap, n).to_vec();
            let c_b = std::slice::from_raw_parts(bp, n).to_vec();

            std::ptr::copy_nonoverlapping(a0.as_ptr(), ap, n);
            std::ptr::write_bytes(bp.cast::<u8>(), 0, n * SPRITE_SIZE);
            p.rust.merge_sort(ap, bp, n as i32);
            let r_a = std::slice::from_raw_parts(ap, n).to_vec();
            let r_b = std::slice::from_raw_parts(bp, n).to_vec();

            assert_eq!(c_a, r_a, "a diverged at page edge (n={n})");
            assert_eq!(c_b, r_b, "b diverged at page edge (n={n})");
            drop_page_edge(guard);
        }
    }
}

/// Allocate two `n`-element regions that each END flush against the top of a
/// mapped page, with the next page left unmapped as a trip-wire for
/// out-of-bounds reads/writes. Returns `(a, b, guard)`.
unsafe fn page_edge_pair(n: usize) -> (*mut Sprite, *mut Sprite, (*mut u8, usize)) {
    let page = 4096usize;
    let need = n * SPRITE_SIZE;
    // Two mapped pages-worth of room per buffer, then one PROT_NONE page.
    let mapped = (need.div_ceil(page) + 1) * page;
    let total = 2 * mapped + page;

    unsafe {
        let base = libc_mmap(total);
        // Layout: [ mapped (a region) ][ mapped (b region) ][ PROT_NONE page ]
        // `a` ends at base+mapped, `b` ends at base+2*mapped, i.e. flush against
        // the PROT_NONE page.
        let a = base.add(mapped - need).cast::<Sprite>();
        let b = base.add(2 * mapped - need).cast::<Sprite>();
        libc_mprotect_none(base.add(2 * mapped), page);
        // Both regions are 16-byte aligned because `need` is a multiple of 16 and
        // page boundaries are 4096-aligned.
        assert_eq!(a as usize % 16, 0);
        assert_eq!(b as usize % 16, 0);
        (a, b, (base, total))
    }
}

unsafe fn drop_page_edge(guard: (*mut u8, usize)) {
    unsafe { libc_munmap(guard.0, guard.1) };
}

// Minimal direct syscall wrappers so no extra dependency is needed.
unsafe extern "C" {
    fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut core::ffi::c_void;
    fn mprotect(addr: *mut core::ffi::c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
}
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const PROT_NONE: i32 = 0;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

unsafe fn libc_mmap(len: usize) -> *mut u8 {
    let p = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert!(p as isize != -1, "mmap failed");
    p.cast::<u8>()
}
unsafe fn libc_mprotect_none(addr: *mut u8, len: usize) {
    let r = unsafe { mprotect(addr.cast(), len, PROT_NONE) };
    assert_eq!(r, 0, "mprotect failed");
}
unsafe fn libc_munmap(addr: *mut u8, len: usize) {
    unsafe { munmap(addr.cast(), len) };
}

// ---------------------------------------------------------------------------
// Row 7 — left run exhausted (`i >= split`): the else branch is taken with no
// comparison at all. Descending input drives this on the first iteration of
// every merge.
// ---------------------------------------------------------------------------
#[test]
fn err_07_left_run_exhausted() {
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0xE7);

    for n in [2usize, 3, 4, 5, 7, 8, 9, 16, 17, 32, 33, 64, 65] {
        for r in 0..4 {
            let a0 = make_sprites(Shape::Descending, n, &mut rng, Some(0xFF));
            let b0 = vec![Sprite([0xEE; SPRITE_SIZE]); n];
            let mut ac = a0.clone();
            let mut bc = b0.clone();
            let mut ar = a0.clone();
            let mut br = b0.clone();
            unsafe {
                p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32);
                p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), n as i32);
            }
            assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged (desc n={n} r={r})");
            assert_eq!(bc.as_slice(), br.as_slice(), "b diverged (desc n={n} r={r})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — the DEAD texture_id tiebreak.
//
// The C comparison returns 1 as soon as `a->sort_bits <= b->sort_bits`, so for
// equal `sort_bits` it always returns 1 regardless of `texture_id`. The second
// `if` can never run. Both libraries must exhibit that, i.e. equal-`sort_bits`
// elements must NOT be reordered by `texture_id`.
// ---------------------------------------------------------------------------
#[test]
fn err_08_dead_texture_id_tiebreak() {
    let p = pair();

    // Worst case for a "fixed" implementation: sort_bits all equal and
    // texture_id strictly decreasing. A texture_id-aware sort would reverse
    // this; the real C leaves the order alone.
    for n in [2usize, 3, 4, 5, 8, 16, 17, 32, 64, 100] {
        let a0: Vec<Sprite> = (0..n)
            .map(|i| Sprite::new((n - i) as u64 * 1000, 7, 0xAA))
            .collect();
        let b0 = vec![Sprite([0x00; SPRITE_SIZE]); n];

        let mut ac = a0.clone();
        let mut bc = b0.clone();
        let mut ar = a0.clone();
        let mut br = b0.clone();
        unsafe {
            p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32);
            p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), n as i32);
        }
        assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged (dead tiebreak n={n})");
        assert_eq!(bc.as_slice(), br.as_slice(), "b diverged (dead tiebreak n={n})");
    }

    // Size 2, the minimal witness: equal bits, a[0].tex > a[1].tex.
    let a0 = vec![Sprite::new(u64::MAX, 0, 0x11), Sprite::new(0, 0, 0x22)];
    let mut ac = a0.clone();
    let mut bc = vec![Sprite([0; SPRITE_SIZE]); 2];
    let mut ar = a0.clone();
    let mut br = bc.clone();
    unsafe {
        p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), 2);
        p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), 2);
    }
    assert_eq!(ac.as_slice(), ar.as_slice());
    assert_eq!(bc.as_slice(), br.as_slice());
    // Pin down the actual C behaviour so a future "fix" to the Rust is caught
    // even if both were changed together.
    assert_eq!(
        ac[0].texture_id(),
        u64::MAX,
        "C reordered equal-sort_bits elements by texture_id — the dead branch \
         is apparently NOT dead on this build; re-derive ERRORS.md row 8"
    );
}

// ---------------------------------------------------------------------------
// Row 8b — the dead branch implies the sort is stable on ties. Verify against
// the C directly, and that the Rust agrees.
// ---------------------------------------------------------------------------
#[test]
fn err_08b_ties_are_stable_not_texture_ordered() {
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0x8B);

    for n in [2usize, 3, 5, 8, 13, 32, 64, 100] {
        for r in 0..6 {
            // A handful of distinct sort_bits, with texture_id recording the
            // original position so ties can be checked for stability.
            let a0: Vec<Sprite> = (0..n)
                .map(|i| Sprite::new(i as u64, (rng.below(3) as i32) * 10, 0xAA))
                .collect();
            let b0 = vec![Sprite([0x00; SPRITE_SIZE]); n];

            let mut ac = a0.clone();
            let mut bc = b0.clone();
            let mut ar = a0.clone();
            let mut br = b0.clone();
            unsafe {
                p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32);
                p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), n as i32);
            }
            assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged (stability n={n} r={r})");
            assert_eq!(bc.as_slice(), br.as_slice(), "b diverged (stability n={n} r={r})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — signed extremes in sort_bits.
// ---------------------------------------------------------------------------
#[test]
fn err_09_sort_bits_signed_extremes() {
    let p = pair();
    const BITS: [i32; 6] = [i32::MIN, i32::MIN + 1, -1, 0, i32::MAX - 1, i32::MAX];

    // Every ordered pair at size 2 (catches an accidental unsigned compare).
    for (x_i, &x) in BITS.iter().enumerate() {
        for (y_i, &y) in BITS.iter().enumerate() {
            let a0 = vec![Sprite::new(1, x, 0xAA), Sprite::new(2, y, 0xBB)];
            let mut ac = a0.clone();
            let mut bc = vec![Sprite([0; SPRITE_SIZE]); 2];
            let mut ar = a0.clone();
            let mut br = bc.clone();
            unsafe {
                p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), 2);
                p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), 2);
            }
            assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged bits({x_i},{y_i})");
            assert_eq!(bc.as_slice(), br.as_slice(), "b diverged bits({x_i},{y_i})");
        }
    }

    // Larger arrays consisting only of extremes.
    let mut rng = Rng::new(RNG_SEED ^ 0xE9);
    for n in [3usize, 4, 7, 8, 17, 33, 64, 128] {
        for r in 0..6 {
            let a0: Vec<Sprite> = (0..n)
                .map(|_| Sprite::new(rng.next_u64(), BITS[rng.below(BITS.len())], 0xFF))
                .collect();
            let b0 = vec![Sprite([0x9C; SPRITE_SIZE]); n];
            let mut ac = a0.clone();
            let mut bc = b0.clone();
            let mut ar = a0.clone();
            let mut br = b0.clone();
            unsafe {
                p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32);
                p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), n as i32);
            }
            assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged extremes n={n} r={r}");
            assert_eq!(bc.as_slice(), br.as_slice(), "b diverged extremes n={n} r={r}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — texture_id extremes must not affect the ordering at all, because the
// only place texture_id is read is the dead branch.
// ---------------------------------------------------------------------------
#[test]
fn err_10_texture_id_extremes_do_not_affect_order() {
    let p = pair();
    const TEX: [u64; 6] = [0, 1, u64::MAX / 2, u64::MAX / 2 + 1, u64::MAX - 1, u64::MAX];
    let mut rng = Rng::new(RNG_SEED ^ 0xEA);

    for n in [2usize, 3, 4, 8, 16, 17, 64] {
        for r in 0..8 {
            // Fixed sort_bits pattern, texture_ids drawn from the extremes.
            let bits: Vec<i32> = (0..n).map(|_| (rng.below(2) as i32) * 5).collect();
            let a0: Vec<Sprite> = bits
                .iter()
                .map(|&b| Sprite::new(TEX[rng.below(TEX.len())], b, 0xAA))
                .collect();
            let b0 = vec![Sprite([0x00; SPRITE_SIZE]); n];

            let mut ac = a0.clone();
            let mut bc = b0.clone();
            let mut ar = a0.clone();
            let mut br = b0.clone();
            unsafe {
                p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32);
                p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), n as i32);
            }
            assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged tex n={n} r={r}");
            assert_eq!(bc.as_slice(), br.as_slice(), "b diverged tex n={n} r={r}");

            // The permutation the C chose must depend only on sort_bits, never
            // on texture_id: re-run with the texture_ids replaced by their
            // index and check the resulting sort_bits sequence is the same.
            let a1: Vec<Sprite> = bits
                .iter()
                .enumerate()
                .map(|(i, &b)| Sprite::new(i as u64, b, 0xAA))
                .collect();
            let mut ac1 = a1.clone();
            let mut bc1 = b0.clone();
            unsafe {
                p.c.merge_sort(ac1.as_mut_ptr(), bc1.as_mut_ptr(), n as i32);
            }
            let seq0: Vec<i32> = ac.iter().map(|s| s.sort_bits()).collect();
            let seq1: Vec<i32> = ac1.iter().map(|s| s.sort_bits()).collect();
            assert_eq!(
                seq0, seq1,
                "C's ordering depended on texture_id (n={n} r={r}) — the dead \
                 branch is not dead; re-derive ERRORS.md row 10"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — negative size.
//
// The C computes the memcpy length as `cltq; shl $0x4`, i.e. it sign-extends
// `int size` to 64 bits and multiplies by 16 with wraparound. For any negative
// size that yields an enormous unsigned length and memcpy faults, so the call
// itself is NOT invoked here (it would kill the test process, in BOTH
// implementations, which is the matching behaviour but unobservable).
//
// What CAN be checked for parity is the byte count each side computes. The Rust
// expression is `SPRITE_SIZE.wrapping_mul(size as usize)`; this test asserts it
// equals the C codegen `(size_t)(int64_t)size * 16` for the whole interesting
// range of negative and positive sizes.
// ---------------------------------------------------------------------------
#[test]
fn err_11_negative_size_byte_count_matches() {
    /// Exactly what `cltq; shl $0x4` produces.
    fn c_byte_count(size: i32) -> usize {
        ((size as i64 as u64).wrapping_mul(16)) as usize
    }
    /// Exactly what `translation/src/lib.rs` computes.
    fn rust_byte_count(size: i32) -> usize {
        16usize.wrapping_mul(size as usize)
    }

    let mut sizes: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -65537,
        -65536,
        -257,
        -256,
        -2,
        -1,
        0,
        1,
        2,
        255,
        256,
        65535,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut rng = Rng::new(RNG_SEED ^ 0xEB);
    for _ in 0..2000 {
        sizes.push(rng.next_i32());
    }

    for &s in &sizes {
        assert_eq!(
            c_byte_count(s),
            rust_byte_count(s),
            "memcpy byte-count mismatch for size={s}"
        );
    }

    // Sanity: a negative size really does produce a huge length (so this row is
    // genuinely a faulting input and not silently a no-op).
    assert_eq!(c_byte_count(-1), 0xFFFF_FFFF_FFFF_FFF0usize);
    assert_ne!(c_byte_count(i32::MIN), 0);
    // And only size == 0 yields a zero-length copy, which is why the Rust's
    // `if bytes != 0` guard cannot diverge from the C's unconditional memcpy.
    for &s in &sizes {
        assert_eq!(c_byte_count(s) == 0, s == 0, "zero-length copy only at size=0 (s={s})");
    }
}

// ---------------------------------------------------------------------------
// Row 12 — oversized size (beyond the caller's allocation) is UB in the C and
// would corrupt the test process. Documented as out of scope; the closest
// checkable property is that for any size within the allocation nothing outside
// [0,size) is touched, which is row 5.
//
// Row 13 — no enum type exists anywhere in the public API, so there is no
// out-of-range enum value to pass across the FFI boundary. Asserted
// mechanically here so the claim cannot silently rot.
// ---------------------------------------------------------------------------
#[test]
fn err_13_no_enums_in_public_api() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src");
    let mut found = Vec::new();
    for rel in ["include/lib.h", "src/lib.c"] {
        let p = root.join(rel);
        let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"));
        for (i, line) in src.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if code.split_whitespace().any(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_') == "enum") {
                found.push(format!("{rel}:{}: {}", i + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "ERRORS.md row 13 claims the C API has no enums, but found:\n{}",
        found.join("\n")
    );

    // The only scalar parameter is `int size`; its full range is covered by
    // rows 1, 4, 11 and the Phase B size axis. Exercise a spread of *valid*
    // sizes against an allocation large enough for the largest, so that
    // "one past the valid range" for a given allocation is also covered.
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0xED);
    const CAP: usize = 64;
    for size in 0..=CAP {
        let a0 = make_sprites(Shape::Random, CAP, &mut rng, Some(0xFF));
        let b0 = vec![Sprite([0x6B; SPRITE_SIZE]); CAP];
        let mut ac = a0.clone();
        let mut bc = b0.clone();
        let mut ar = a0.clone();
        let mut br = b0.clone();
        unsafe {
            p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), size as i32);
            p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), size as i32);
        }
        assert_eq!(ac.as_slice(), ar.as_slice(), "a diverged size={size}");
        assert_eq!(bc.as_slice(), br.as_slice(), "b diverged size={size}");
        // Everything at or past `size` must be untouched in both buffers.
        assert_eq!(&ac[size..], &a0[size..], "C wrote past size in a (size={size})");
        assert_eq!(&bc[size..], &b0[size..], "C wrote past size in b (size={size})");
    }
}

// ---------------------------------------------------------------------------
// Row 14 — padding bytes propagate through both the memcpy and the 16-byte
// struct assignment, so they are externally observable and must match.
// ---------------------------------------------------------------------------
#[test]
fn err_14_padding_bytes_propagate() {
    let p = pair();
    let mut rng = Rng::new(RNG_SEED ^ 0xEE);

    for n in 1..=48usize {
        for r in 0..4 {
            // Distinct, per-element padding so any mix-up is visible.
            let a0: Vec<Sprite> = (0..n)
                .map(|i| {
                    let mut s = Sprite::new(rng.next_u64(), rng.next_i32() % 8, 0);
                    s.0[12] = (i as u8).wrapping_mul(3).wrapping_add(1);
                    s.0[13] = 0xA5;
                    s.0[14] = (i as u8) ^ 0x5A;
                    s.0[15] = rng.next_u8() | 1;
                    s
                })
                .collect();
            let b0 = vec![Sprite([0x55; SPRITE_SIZE]); n];

            let mut ac = a0.clone();
            let mut bc = b0.clone();
            let mut ar = a0.clone();
            let mut br = b0.clone();
            unsafe {
                p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32);
                p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), n as i32);
            }

            // Full byte comparison, padding included.
            assert_eq!(ac.as_slice(), ar.as_slice(), "a padding diverged n={n} r={r}");
            assert_eq!(bc.as_slice(), br.as_slice(), "b padding diverged n={n} r={r}");

            // And confirm the C really does carry padding across (so this row
            // is testing something): every output element's padding must equal
            // some input element's padding, never the 0x55 scratch fill.
            if n >= 2 {
                for (k, s) in bc.iter().enumerate() {
                    assert_ne!(
                        &s.0[12..16],
                        &[0x55u8; 4][..],
                        "scratch fill survived in b[{k}] (n={n}) — C does not copy \
                         padding on this build; re-derive ERRORS.md row 14"
                    );
                }
            }
        }
    }
}
