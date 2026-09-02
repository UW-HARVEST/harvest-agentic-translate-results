//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundary cases (null
//! pointers, zero and oversized lengths, one-past-range values).
//!
//! The library's only public function returns `void` and validates nothing, so
//! "same error" means: the same observable effect (which bytes were written) or
//! the same fatal signal. Rows that can only end in a fatal signal are run in a
//! child process so the C and Rust crashes can be compared.

mod common;

use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ---------------------------------------------------------------------------
// ERRORS.md rows 1-3 — the comparator's three exits
// ---------------------------------------------------------------------------

/// Row 1: `a->sort_bits <= b->sort_bits` returns 1 immediately.
/// Row 2: the `texture_id` tiebreak is UNREACHABLE.
/// Row 3: `a->sort_bits > b->sort_bits` returns 0.
///
/// Driven through `merge_sort` on 2-element arrays, which reduces to exactly one
/// comparator call, so each row maps to a distinct observable outcome.
#[test]
fn errors_row01_03_comparator_exits() {
    let pair = Pair::load();

    // Row 1 (strictly less), Row 1 via equality (which is what makes row 2
    // dead), and Row 3 (greater).
    let cases: &[(&str, i32, i32, u64, u64)] = &[
        // label,                       bits_a,    bits_b,   tex_a,      tex_b
        ("row01 a<b", -5, 5, u64::MAX, 0),
        ("row01 a==b, tex_a>tex_b (row02 would flip)", 7, 7, u64::MAX, 0),
        ("row01 a==b, tex_a<tex_b", 7, 7, 0, u64::MAX),
        ("row01 a==b, tex equal", 7, 7, 9, 9),
        ("row03 a>b", 5, -5, 0, u64::MAX),
        ("row01 INT_MIN<=INT_MAX", i32::MIN, i32::MAX, u64::MAX, 0),
        ("row03 INT_MAX>INT_MIN", i32::MAX, i32::MIN, 0, u64::MAX),
        ("row01 -1<=0", -1, 0, u64::MAX, 0),
        ("row03 0>-1", 0, -1, 0, u64::MAX),
    ];

    for (label, ba, bb, ta, tb) in cases {
        let a = vec![Sprite::new(*ta, *ba, [0xAA; 4]), Sprite::new(*tb, *bb, [0xBB; 4])];
        let b = vec![Sprite::new(0xDEAD, 0x1234, [0xCC; 4]); 2];
        diff_on(&pair, label, &a, &b);
    }

    // Independently pin the *C* outcome for the equality case, so a Rust
    // comparator that "fixes" the dead tiebreak cannot slip through by matching
    // some other divergent implementation.
    let mut a = vec![Sprite::new(u64::MAX, 7, [0; 4]), Sprite::new(0, 7, [0; 4])];
    let mut b = vec![Sprite::zeroed(); 2];
    unsafe { (pair.c)(a.as_mut_ptr(), b.as_mut_ptr(), 2) };
    assert_eq!(
        a[0].texture_id(),
        u64::MAX,
        "C is expected to keep input order for equal sort_bits (dead tiebreak)"
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — left run exhausted, `j` advances with no bound check
// ERRORS.md row 5 — right run exhausted, `a + j` must NOT be dereferenced
// ---------------------------------------------------------------------------

#[test]
fn errors_row04_left_run_exhausted() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x04);
    // Ascending data drives `i >= split` at the end of every merge.
    for &n in SIZES {
        for _ in 0..4 {
            let a = gen_ascending(&mut rng, n);
            diff_on(&pair, &format!("row04 n={n}"), &a, &garbage_scratch(&mut rng, n));
        }
    }
}

#[test]
fn errors_row05_right_run_exhausted_short_circuit() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x05);
    // Descending data drives `j >= hi` at the end of every merge; if the Rust
    // evaluated the comparator instead of short-circuiting it would read
    // `a[hi]`, which for the outermost merge is one past the buffer. Running
    // this under the exact allocation size makes such a read detectable.
    for &n in SIZES {
        for _ in 0..4 {
            let a = gen_descending(&mut rng, n);
            diff_on(&pair, &format!("row05 n={n}"), &a, &garbage_scratch(&mut rng, n));
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 — `hi - lo <= 1` early return
// ---------------------------------------------------------------------------

#[test]
fn errors_row06_recurse_early_return() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x06);
    // size 0 and 1 hit the early return at the top level; every larger size
    // hits it at every leaf.
    for _ in 0..128 {
        for n in [0usize, 1] {
            let a = gen_full_random(&mut rng, n.max(1));
            let b = garbage_scratch(&mut rng, n.max(1));
            diff_with_size_on(&pair, &format!("row06 size={n}"), &a, &b, n as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 7 — size == 0 leaves both buffers untouched
// ---------------------------------------------------------------------------

#[test]
fn errors_row07_size_zero_no_writes() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x07);
    for _ in 0..128 {
        let a = gen_full_random(&mut rng, 16);
        let b = garbage_scratch(&mut rng, 16);
        diff_with_size_on(&pair, "row07 size=0", &a, &b, 0);
    }

    // Pin the absolute behaviour too: the C must not write a single byte.
    let a0 = gen_full_random(&mut rng, 16);
    let b0 = garbage_scratch(&mut rng, 16);
    for (label, f) in [("C", pair.c), ("Rust", pair.rust)] {
        let mut a = a0.clone();
        let mut b = b0.clone();
        unsafe { f(a.as_mut_ptr(), b.as_mut_ptr(), 0) };
        assert_eq!(a, a0, "{label} modified `a` for size=0");
        assert_eq!(b, b0, "{label} modified `b` for size=0");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 8 — size == 1 copies 16 bytes, sorts nothing
// ---------------------------------------------------------------------------

#[test]
fn errors_row08_size_one_copies_all_16_bytes() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..256 {
        let a = gen_full_random(&mut rng, 1);
        let b = garbage_scratch(&mut rng, 1);
        diff_on(&pair, "row08 size=1", &a, &b);
    }

    // Absolute check: `b[0]` must equal `a[0]` byte-for-byte including padding,
    // and `a` must be unchanged. Confirms the 16-byte (not 12-byte) copy.
    let a0 = vec![Sprite::new(0x0123_4567_89AB_CDEF, -12345, [0x11, 0x22, 0x33, 0x44])];
    let b0 = vec![Sprite::new(0xFFFF_FFFF_FFFF_FFFF, 999, [0x99; 4])];
    for (label, f) in [("C", pair.c), ("Rust", pair.rust)] {
        let mut a = a0.clone();
        let mut b = b0.clone();
        unsafe { f(a.as_mut_ptr(), b.as_mut_ptr(), 1) };
        assert_eq!(a, a0, "{label}: `a` must be unchanged for size=1");
        assert_eq!(b[0].0, a0[0].0, "{label}: `b[0]` must be a full 16-byte copy of `a[0]`");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 9 — null pointers with size == 0
// ---------------------------------------------------------------------------

#[test]
fn errors_row09_null_pointers_size_zero() {
    let pair = Pair::load();
    for size in [0i32] {
        unsafe { (pair.c)(std::ptr::null_mut(), std::ptr::null_mut(), size) };
        unsafe { (pair.rust)(std::ptr::null_mut(), std::ptr::null_mut(), size) };
    }
    // Mixed null / non-null with size == 0.
    let mut buf = vec![Sprite::new(1, 2, [3; 4]); 4];
    let snapshot = buf.clone();
    unsafe { (pair.c)(buf.as_mut_ptr(), std::ptr::null_mut(), 0) };
    unsafe { (pair.rust)(buf.as_mut_ptr(), std::ptr::null_mut(), 0) };
    assert_eq!(buf, snapshot, "size=0 must not touch `a`");

    let mut buf2 = vec![Sprite::new(4, 5, [6; 4]); 4];
    let snapshot2 = buf2.clone();
    unsafe { (pair.c)(std::ptr::null_mut(), buf2.as_mut_ptr(), 0) };
    unsafe { (pair.rust)(std::ptr::null_mut(), buf2.as_mut_ptr(), 0) };
    assert_eq!(buf2, snapshot2, "size=0 must not touch `b`");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 10 — negative `size` widens to a ~2**64 memcpy length
//
// Both implementations must die the same way. Run out of process: the child
// re-executes this same test binary with `HARVEST_CRASH_IMPL` set.
// ---------------------------------------------------------------------------

const CRASH_IMPL_VAR: &str = "HARVEST_CRASH_IMPL";
const CRASH_SIZE_VAR: &str = "HARVEST_CRASH_SIZE";

/// Child-side entry point. Does nothing unless the env vars are set, so it is a
/// harmless no-op during a normal test run.
///
/// Prints a canonical digest of both buffers so the parent can compare the
/// observable effect, not merely "both survived".
#[test]
fn zz_crash_child_hook() {
    let Ok(which) = std::env::var(CRASH_IMPL_VAR) else {
        return;
    };
    let size: i32 = std::env::var(CRASH_SIZE_VAR)
        .expect("HARVEST_CRASH_SIZE")
        .parse()
        .expect("i32");

    let pair = Pair::load();
    let f = match which.as_str() {
        "c" => pair.c,
        "rust" => pair.rust,
        other => panic!("unknown impl {other}"),
    };

    // Modest real allocations; `size` is a lie, exactly as the C would receive
    // it from a buggy caller. Guard pages of known bytes on either side let us
    // detect any write that spills out of the logical arrays.
    const N: usize = 32;
    let mut a: Vec<Sprite> = (0..3 * N)
        .map(|i| Sprite::new(0x1000 + i as u64, i as i32, [0x5A; 4]))
        .collect();
    let mut b: Vec<Sprite> = (0..3 * N)
        .map(|i| Sprite::new(0x2000 + i as u64, -(i as i32), [0xA5; 4]))
        .collect();

    let ap = unsafe { a.as_mut_ptr().add(N) };
    let bp = unsafe { b.as_mut_ptr().add(N) };

    println!("CHILD-BEGIN {which} size={size}");
    unsafe { f(ap, bp, size) };
    println!("A {}", digest(&a));
    println!("B {}", digest(&b));
    println!("CHILD-SURVIVED");
    // Flush before the harness tears down.
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::process::exit(42);
}

/// FNV-1a over the raw bytes of the buffer, so the parent compares exact byte
/// images across the process boundary.
fn digest(v: &[Sprite]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for s in v {
        for &byte in &s.0 {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    /// `Some((digest_a, digest_b))` when the call returned normally.
    digests: Option<(String, String)>,
}

fn run_crash_child(which: &str, size: i32) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "zz_crash_child_hook", "--nocapture", "--test-threads=1"])
        .env(CRASH_IMPL_VAR, which)
        .env(CRASH_SIZE_VAR, size.to_string())
        .env("RUST_BACKTRACE", "0")
        // Propagate .so discovery to the child.
        .env("HARVEST_C_SO", c_so_path())
        .env("HARVEST_RUST_SO", rust_so_path())
        .output()
        .expect("spawn crash child");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let grab = |tag: &str| {
        stdout
            .lines()
            .find_map(|l| l.strip_prefix(tag).map(|s| s.trim().to_string()))
    };
    let digests = match (grab("A "), grab("B ")) {
        (Some(a), Some(b)) if stdout.contains("CHILD-SURVIVED") => Some((a, b)),
        _ => None,
    };

    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        digests,
    }
}

#[test]
fn errors_row10_negative_size_same_fatal_outcome() {
    for size in [-1i32, -2, -16, -1000, i32::MIN, i32::MIN + 1] {
        let c = run_crash_child("c", size);
        let r = run_crash_child("rust", size);
        assert_eq!(
            c, r,
            "negative size={size}: C and Rust must behave identically \
             (same exit status/signal AND same resulting buffer bytes)\n  C   ={c:?}\n  Rust={r:?}"
        );
        // The row must actually have been exercised: either both crashed with a
        // signal, or both returned and reported comparable digests.
        assert!(
            c.signal.is_some() || c.digests.is_some(),
            "negative size={size}: child produced neither a signal nor digests: {c:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 11 — `lo + hi` signed overflow for enormous `size`
//
// Not executable (needs ~32 GiB of buffers). What IS executable is the same
// arithmetic at a scale we can afford: `size` values whose midpoint computation
// is exercised at every level, plus the largest `size` we can actually allocate.
// The overflow itself is documented in ERRORS.md and reproduced in the Rust with
// `wrapping_add` + truncating `/`, matching gcc's `add; shr $0x1f; add; sar $1`.
// ---------------------------------------------------------------------------

#[test]
fn errors_row11_midpoint_arithmetic_at_largest_affordable_size() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x11);
    // 1 << 17 elements = 2 MiB per buffer. Large enough for 17 recursion levels.
    for &n in &[65535usize, 65536, 65537, 131071, 131072] {
        let a = gen_small_range(&mut rng, n);
        let b = garbage_scratch(&mut rng, n);
        diff_on(&pair, &format!("row11 midpoint n={n}"), &a, &b);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 12 — padding propagation
// ---------------------------------------------------------------------------

#[test]
fn errors_row12_padding_propagated_not_normalised() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x12);

    // Differential across sizes.
    for &n in SIZES {
        for _ in 0..4 {
            let a: Vec<Sprite> = (0..n)
                .map(|i| Sprite::new(rng.next_u64(), (i % 3) as i32, [0xF0 | (i as u8 & 0xF); 4]))
                .collect();
            diff_on(&pair, &format!("row12 n={n}"), &a, &zero_scratch(n));
        }
    }

    // Absolute check: the C must carry non-zero padding into the output.
    let a0: Vec<Sprite> = (0..8)
        .map(|i| Sprite::new(i as u64, (7 - i) as i32, [0xDE, 0xAD, 0xBE, 0xEF]))
        .collect();
    let mut a = a0.clone();
    let mut b = vec![Sprite::zeroed(); 8];
    unsafe { (pair.c)(a.as_mut_ptr(), b.as_mut_ptr(), 8) };
    assert!(
        a.iter().all(|s| s.padding() == [0xDE, 0xAD, 0xBE, 0xEF]),
        "C is expected to propagate padding through the 16-byte struct copy, got {a:?}"
    );
}

// ---------------------------------------------------------------------------
// Generic FFI boundary cases required by Phase C beyond the table
// ---------------------------------------------------------------------------

/// `size` values one step past each documented boundary, on both sides.
#[test]
fn boundary_sizes_around_documented_ranges() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xB1);

    // Allocate generously, then pass a `size` at/below the allocation so the
    // call stays in bounds while still probing the boundary values.
    const CAP: usize = 64;
    for size in [0i32, 1, 2, 3, 63, 64] {
        for _ in 0..16 {
            let a = gen_full_random(&mut rng, CAP);
            let b = garbage_scratch(&mut rng, CAP);
            diff_with_size_on(&pair, &format!("boundary size={size}"), &a, &b, size);
        }
    }
}

/// `size` smaller than the allocation (undersized length): the C must leave the
/// tail of both buffers untouched, and the Rust must leave exactly the same
/// bytes untouched.
#[test]
fn undersized_length_leaves_tail_untouched() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xB2);
    for &n in &[1usize, 2, 3, 5, 8, 17, 64] {
        for size in 0..=(n as i32) {
            let a = gen_full_random(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_with_size_on(&pair, &format!("undersized n={n} size={size}"), &a, &b, size);
        }
    }
}

/// `size` LARGER than the real allocation (oversized length). Both
/// implementations read/write out of bounds; the requirement is that they do so
/// identically. Kept to a small overrun into a deliberately oversized
/// allocation so the test is well-defined rather than a coin-flip crash: the
/// backing store is `CAP` elements, the API is told `size`, and only the first
/// `size <= CAP` are compared.
#[test]
fn oversized_length_reported_to_api() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xB3);
    const CAP: usize = 128;
    // The library is handed a size larger than the caller's "logical" length
    // (16) but still inside the real allocation (128).
    for size in [17i32, 31, 32, 33, 100, 127, 128] {
        for _ in 0..8 {
            let a = gen_full_random(&mut rng, CAP);
            let b = garbage_scratch(&mut rng, CAP);
            diff_with_size_on(&pair, &format!("oversized size={size}"), &a, &b, size);
        }
    }
}

/// There are no enums in `c_src/include/lib.h`, so the "out-of-range enum
/// value" class has no instance. The equivalent for this API is an arbitrary
/// `int` in the only scalar parameter. Every non-negative `int` that the caller
/// can back with real memory is covered above; the whole negative half of the
/// range is covered by `errors_row10_*`. This test documents and pins the
/// exhaustive small-int sweep.
#[test]
fn exhaustive_small_int_size_sweep() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xB4);
    const CAP: usize = 512;
    let a = gen_full_random(&mut rng, CAP);
    let b = garbage_scratch(&mut rng, CAP);
    for size in 0..=(CAP as i32) {
        diff_with_size_on(&pair, &format!("sweep size={size}"), &a, &b, size);
    }
}
