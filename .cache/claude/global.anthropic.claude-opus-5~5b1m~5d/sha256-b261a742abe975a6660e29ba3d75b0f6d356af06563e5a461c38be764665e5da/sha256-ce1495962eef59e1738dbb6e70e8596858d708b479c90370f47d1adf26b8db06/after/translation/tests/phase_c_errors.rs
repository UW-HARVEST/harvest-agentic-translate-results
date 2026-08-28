//! Phase C — error / rejection-path differential tests, one per `ERRORS.md` row.
//!
//! The library has no error codes at all (`merge_sort` returns `void`, and the C
//! contains no `assert`, no `NULL` check and no range check). Its rejection
//! surface is therefore: (a) the `hi - lo <= 1` guard, (b) silent no-ops, and
//! (c) fatal signals on out-of-domain input.
//!
//! Rows whose expected result is a fatal signal are checked by re-executing this
//! test binary as a child process and comparing the **exact** termination status
//! (`WTERMSIG` / exit code) of the C `.so` against the Rust `.so` — not merely
//! "both failed somehow".

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// ===========================================================================
// Non-fatal rows — tested in-process
// ===========================================================================

/// `ERRORS.md` #1 — `size == 0`: complete no-op, `b` keeps its pre-fill.
#[test]
fn err01_size_zero_is_total_noop() {
    let mut rng = Rng::new(SEED() ^ hash_str("err01"));
    for trial in 0..64 {
        // Allocate real elements so an accidental write is detectable.
        let a = gen_input(K::Rand, T::Rand, P::Garbage, 8, &mut rng);
        let b = gen_scratch(F::Sentinel, 8);
        assert_same(&format!("err01 [trial={trial}]"), &a, &b, 0);

        // And pin the absolute expectation from the C source.
        let c = run_one(pair().c, &a, &b, 0);
        let r = run_one(pair().rust, &a, &b, 0);
        assert_eq!(bytes(&c.a), bytes(&a), "err01: C modified `a` on size=0");
        assert_eq!(bytes(&c.b), bytes(&b), "err01: C modified `b` on size=0");
        assert_eq!(bytes(&r.a), bytes(&a), "err01: Rust modified `a` on size=0");
        assert_eq!(bytes(&r.b), bytes(&b), "err01: Rust modified `b` on size=0");
    }
}

/// `ERRORS.md` #2 — `size == 1`: 16-byte memcpy only, recursion guard returns.
#[test]
fn err02_size_one_copies_exactly_one_element() {
    let mut rng = Rng::new(SEED() ^ hash_str("err02"));
    for trial in 0..64 {
        let a = gen_input(K::Rand, T::Rand, P::Garbage, 4, &mut rng);
        let b = gen_scratch(F::Sentinel, 4);
        assert_same(&format!("err02 [trial={trial}]"), &a, &b, 1);

        let c = run_one(pair().c, &a, &b, 1);
        let r = run_one(pair().rust, &a, &b, 1);
        // `a` untouched; b[0] == a[0] (all 16 bytes); b[1..] still sentinel.
        assert_eq!(bytes(&c.a), bytes(&a), "err02: C modified `a`");
        assert_eq!(bytes(&c.b)[..SPRITE_SIZE], bytes(&a)[..SPRITE_SIZE]);
        assert_eq!(bytes(&c.b)[SPRITE_SIZE..], bytes(&b)[SPRITE_SIZE..]);
        assert_eq!(bytes(&r.a), bytes(&a), "err02: Rust modified `a`");
        assert_eq!(bytes(&r.b)[..SPRITE_SIZE], bytes(&a)[..SPRITE_SIZE]);
        assert_eq!(bytes(&r.b)[SPRITE_SIZE..], bytes(&b)[SPRITE_SIZE..]);
    }
}

/// `ERRORS.md` #3 — `size == 0` with BOTH pointers NULL: must return normally.
#[test]
fn err03_null_pointers_with_size_zero_return_normally() {
    unsafe {
        (pair().c)(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        (pair().rust)(std::ptr::null_mut(), std::ptr::null_mut(), 0);
    }
    // Reaching here means neither implementation dereferenced anything.
    // Also exercise the one-null / one-valid mixes at size 0.
    let mut a = gen_input(K::Rand, T::Rand, P::Zero, 4, &mut Rng::new(1));
    let mut b = gen_scratch(F::Sentinel, 4);
    unsafe {
        (pair().c)(a.as_mut_ptr(), std::ptr::null_mut(), 0);
        (pair().rust)(a.as_mut_ptr(), std::ptr::null_mut(), 0);
        (pair().c)(std::ptr::null_mut(), b.as_mut_ptr(), 0);
        (pair().rust)(std::ptr::null_mut(), b.as_mut_ptr(), 0);
    }
}

/// `ERRORS.md` #10/#11 — the `hi - lo <= 1` recursion guard.
///
/// Reached at the top level by `size` 0 and 1, and at every leaf of the
/// recursion for larger sizes. Verified through the public entry point across
/// the sizes whose split trees contain `hi-lo == 1` leaves on one or both sides.
#[test]
fn err10_11_recursion_guard_leaves() {
    let mut rng = Rng::new(SEED() ^ hash_str("err10"));
    // Odd sizes guarantee at least one `hi-lo==1` leaf; 2 and 3 are the minimal
    // cases; 0 is the only way to reach `hi-lo == 0`.
    for size in [0i32, 1, 2, 3, 5, 6, 7, 9, 11, 13, 21, 33, 63, 65] {
        for trial in 0..8 {
            let a = gen_input(K::Rand, T::Rand, P::Garbage, size as usize, &mut rng);
            let b = gen_scratch(F::Sentinel, size as usize);
            assert_same(&format!("err10_11 [size={size} trial={trial}]"), &a, &b, size);
        }
    }
}

/// `ERRORS.md` #12/#13 — `texture_id` is NEVER consulted (line 9 is dead code).
///
/// Property: two inputs that differ ONLY in `texture_id` must be permuted
/// identically. Both implementations must agree on that, and must agree with
/// each other. If either had a live `texture_id` tiebreak, the permutations
/// would differ where `sort_bits` ties.
#[test]
fn err12_13_texture_id_never_affects_order() {
    let mut rng = Rng::new(SEED() ^ hash_str("err12"));
    for size in [2i32, 3, 4, 5, 8, 16, 17, 64, 100, 257] {
        for trial in 0..16 {
            // Heavy ties: that is where a live line 9 would change the answer.
            let bits = gen_sort_bits(K::Few, size as usize, &mut rng);

            // Tag each element with its input index in the padding word so the
            // resulting permutation is directly observable.
            let mk = |texs: &[u64]| -> Vec<Sprite> {
                (0..size as usize)
                    .map(|i| Sprite {
                        texture_id: texs[i],
                        sort_bits: bits[i],
                        pad: i as u32,
                    })
                    .collect()
            };
            let asc: Vec<u64> = (0..size as u64).collect();
            let desc: Vec<u64> = (0..size as u64).map(|i| u64::MAX - i).collect();
            let rnd: Vec<u64> = (0..size).map(|_| rng.next_u64()).collect();

            let a1 = mk(&asc);
            let a2 = mk(&desc);
            let a3 = mk(&rnd);
            let b = gen_scratch(F::Sentinel, size as usize);

            let ctx = format!("err12_13 [size={size} trial={trial}]");
            assert_same(&ctx, &a1, &b, size);
            assert_same(&ctx, &a2, &b, size);
            assert_same(&ctx, &a3, &b, size);

            // The permutation (recovered from the padding tags) must be the same
            // for all three texture_id assignments, in BOTH implementations.
            let perm = |o: &Outcome| -> Vec<u32> { o.a.iter().map(|s| s.pad).collect() };
            let p1c = perm(&run_one(pair().c, &a1, &b, size));
            let p2c = perm(&run_one(pair().c, &a2, &b, size));
            let p3c = perm(&run_one(pair().c, &a3, &b, size));
            let p1r = perm(&run_one(pair().rust, &a1, &b, size));
            let p2r = perm(&run_one(pair().rust, &a2, &b, size));
            let p3r = perm(&run_one(pair().rust, &a3, &b, size));
            assert_eq!(p1c, p2c, "{ctx}: C order changed with texture_id (asc vs desc)");
            assert_eq!(p1c, p3c, "{ctx}: C order changed with texture_id (asc vs rand)");
            assert_eq!(p1r, p2r, "{ctx}: Rust order changed with texture_id (asc vs desc)");
            assert_eq!(p1r, p3r, "{ctx}: Rust order changed with texture_id (asc vs rand)");
            assert_eq!(p1c, p1r, "{ctx}: C and Rust permutations differ");
        }
    }
}

/// `ERRORS.md` #14 — signed `int` comparison at the `INT_MIN`/`INT_MAX` extremes.
#[test]
fn err14_signed_key_extremes() {
    let extremes: [i32; 8] =
        [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX, i32::MIN / 2];
    // Exhaustive over all ordered pairs at size 2 (the minimal merge), with
    // both texture_id orders, so every compare outcome is hit directly.
    for &x in &extremes {
        for &y in &extremes {
            for (tx, ty) in [(0u64, 0u64), (0, u64::MAX), (u64::MAX, 0)] {
                let a = vec![
                    Sprite { texture_id: tx, sort_bits: x, pad: 0x1111_1111 },
                    Sprite { texture_id: ty, sort_bits: y, pad: 0x2222_2222 },
                ];
                let b = gen_scratch(F::Sentinel, 2);
                assert_same(&format!("err14 [x={x} y={y} tx={tx:#x} ty={ty:#x}]"), &a, &b, 2);
            }
        }
    }
    // And a larger array built only from the extreme values.
    let mut rng = Rng::new(SEED() ^ hash_str("err14"));
    for size in [3i32, 4, 8, 17, 64, 129] {
        for _ in 0..16 {
            let a = gen_input(K::Ext, T::Ext, P::Garbage, size as usize, &mut rng);
            let b = gen_scratch(F::Sentinel, size as usize);
            assert_same(&format!("err14 bulk [size={size}]"), &a, &b, size);
        }
    }
}

/// `ERRORS.md` #15/#16 — the two short-circuit paths in `_iteration`
/// (`i >= split`, i.e. left run exhausted; and `i < split && j >= hi`, i.e.
/// right run exhausted, where `less_than_or_equal` is never called).
#[test]
fn err15_16_run_exhaustion_paths() {
    let mut rng = Rng::new(SEED() ^ hash_str("err15"));
    for size in [2i32, 3, 4, 5, 8, 9, 16, 17, 100, 1000] {
        for trial in 0..8 {
            // Ascending keys drain the left run last -> `j >= hi` path dominates.
            let asc = gen_input(K::Asc, T::Rand, P::Garbage, size as usize, &mut rng);
            // Descending keys drain the right run last -> `i >= split` dominates.
            let desc = gen_input(K::Desc, T::Rand, P::Garbage, size as usize, &mut rng);
            let b = gen_scratch(F::Sentinel, size as usize);
            assert_same(&format!("err15 asc [size={size} trial={trial}]"), &asc, &b, size);
            assert_same(&format!("err16 desc [size={size} trial={trial}]"), &desc, &b, size);
        }
    }
}

/// `ERRORS.md` #18 — aliased buffers `a == b`.
#[test]
fn err18_aliased_buffers() {
    let mut rng = Rng::new(SEED() ^ hash_str("err18"));
    for size in [0i32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 33, 100, 257] {
        for trial in 0..16 {
            let a = gen_input(K::Few, T::Rand, P::Garbage, size as usize, &mut rng);
            assert_same_aliased(&format!("err18 [size={size} trial={trial}]"), &a, size);
        }
    }
}

/// `ERRORS.md` #20 — struct padding bytes propagate identically.
#[test]
fn err20_padding_propagation() {
    let mut rng = Rng::new(SEED() ^ hash_str("err20"));
    for size in [1i32, 2, 3, 4, 5, 8, 9, 16, 17, 64, 257] {
        for trial in 0..16 {
            let mut a = gen_input(K::Few, T::Rand, P::Garbage, size as usize, &mut rng);
            // Every element gets a unique, fully non-zero padding word so a
            // dropped or mis-sourced padding copy cannot be masked.
            for (i, e) in a.iter_mut().enumerate() {
                e.pad = 0xC000_0000u32 | (i as u32).wrapping_mul(2_654_435_761) | 0x8181;
            }
            for f in ALL_F {
                let b = gen_scratch(f, size as usize);
                assert_same(&format!("err20 [size={size} trial={trial} F={f:?}]"), &a, &b, size);
            }
        }
    }
}

// ===========================================================================
// Generic boundary sweep: the full `int` domain of `size`
// ===========================================================================

/// The only scalar the API accepts is `int size`. `lib.h` declares no `enum`, so
/// there is no invalid-enum-variant input class; the analogous "value with no
/// valid meaning crossing the FFI boundary" is an out-of-range `size`. Every
/// non-crashing interesting value is swept here; the crashing ones are in the
/// subprocess tests below.
#[test]
fn boundary_size_domain_sweep_nonfatal() {
    let mut rng = Rng::new(SEED() ^ hash_str("sweep"));
    let cap = 2048usize;
    let a_full = gen_input(K::Rand, T::Rand, P::Garbage, cap, &mut rng);
    let b_full = gen_scratch(F::Sentinel, cap);
    // 0 and 1 are the guard boundaries; then one-step-past values around every
    // power of two up to the allocation cap.
    let mut sizes: Vec<i32> = vec![0, 1, 2];
    let mut p = 4i32;
    while (p as usize) <= cap {
        for d in [-1i32, 0, 1] {
            let v = p + d;
            if v >= 0 && (v as usize) <= cap {
                sizes.push(v);
            }
        }
        p *= 2;
    }
    sizes.push(cap as i32);
    sizes.sort_unstable();
    sizes.dedup();
    for size in sizes {
        let n = size as usize;
        assert_same(
            &format!("boundary sweep [size={size}]"),
            &a_full[..n],
            &b_full[..n],
            size,
        );
    }
}

// ===========================================================================
// Fatal rows — compared by exact child-process termination status
// ===========================================================================

/// How a child process ended, plus — when it survived — a digest of the final
/// contents of both buffers. Comparing the digest is what stops a "both
/// survived" outcome from being vacuous.
#[derive(Debug, PartialEq, Eq)]
struct Term {
    code: Option<i32>,
    signal: Option<i32>,
    digest: String,
}

impl Term {
    fn is_fatal(&self) -> bool {
        self.signal.is_some()
    }
}

fn run_crash_child(case: &str, impl_name: &str) -> Term {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "crash_child", "--ignored", "--test-threads=1", "--nocapture"])
        .env("CRASH_CASE", case)
        .env("CRASH_IMPL", impl_name)
        // Keep the child's own library discovery identical to the parent's.
        .env("C_LIB_PATH", c_lib_path())
        .env("RUST_LIB_PATH", rust_lib_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("spawn crash child");
    // The child prints exactly one `DIGEST <a> <b>` marker if the call returned.
    // libtest prefixes it with `test crash_child ... `, so scan for the marker
    // anywhere in stdout rather than at the start of a line.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let digest = match stdout.find("DIGEST ") {
        Some(i) => stdout[i..].lines().next().unwrap_or("").trim().to_string(),
        None => "<no-digest:process-died>".to_string(),
    };
    Term { code: out.status.code(), signal: out.status.signal(), digest }
}

/// Assert C and Rust behave identically for `case`: either both die with the
/// SAME signal, or both return normally with byte-identical buffers.
///
/// Also asserts the case is *conclusive* — a child that died from a Rust panic
/// (exit 101) rather than a real signal would otherwise silently pass.
fn assert_same_termination(case: &str) -> Term {
    let c = run_crash_child(case, "c");
    let r = run_crash_child(case, "rust");
    assert_eq!(
        c, r,
        "case `{case}`: behaviour differs — C={c:?} Rust={r:?}. Both must produce \
         the identical signal/exit status and identical buffer contents."
    );
    assert!(
        c.is_fatal() || c.code == Some(0),
        "case `{case}`: inconclusive outcome {c:?} — expected either a fatal signal \
         or a clean exit(0), not an abnormal exit code (e.g. a Rust panic)."
    );
    if !c.is_fatal() {
        assert!(
            c.digest.starts_with("DIGEST "),
            "case `{case}`: survived but produced no buffer digest ({c:?}); the \
             comparison would be vacuous."
        );
    }
    c
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// The child body. Never runs in a normal test session (it is `#[ignore]`d and
/// only selected by `--exact crash_child --ignored`).
#[test]
#[ignore = "spawned as a child process by the crash-parity tests"]
fn crash_child() {
    let case = match std::env::var("CRASH_CASE") {
        Ok(c) => c,
        Err(_) => return,
    };
    let f = match std::env::var("CRASH_IMPL").unwrap_or_default().as_str() {
        "c" => pair().c,
        "rust" => pair().rust,
        other => panic!("bad CRASH_IMPL={other}"),
    };

    let mut rng = Rng::new(0xC0FFEE);
    let mut a = gen_input(K::Rand, T::Rand, P::Garbage, 32, &mut rng);
    let mut b = gen_scratch(F::Sentinel, 32);
    let null = std::ptr::null_mut::<Sprite>();

    unsafe {
        match case.as_str() {
            // ERRORS.md #4 / #6
            "neg1" => f(a.as_mut_ptr(), b.as_mut_ptr(), -1),
            "neg2" => f(a.as_mut_ptr(), b.as_mut_ptr(), -2),
            "neg1000" => f(a.as_mut_ptr(), b.as_mut_ptr(), -1000),
            // ERRORS.md #5
            "intmin" => f(a.as_mut_ptr(), b.as_mut_ptr(), i32::MIN),
            "intmin_plus1" => f(a.as_mut_ptr(), b.as_mut_ptr(), i32::MIN + 1),
            // ERRORS.md #7
            "null_a" => f(null, b.as_mut_ptr(), 8),
            // ERRORS.md #8
            "null_b" => f(a.as_mut_ptr(), null, 8),
            // ERRORS.md #9
            "null_both" => f(null, null, 8),
            "null_both_size1" => f(null, null, 1),
            other => panic!("unknown CRASH_CASE={other}"),
        }
    }
    // The call returned, so the case was not fatal. Publish a digest of BOTH
    // buffers so the parent can verify the surviving outcomes match too.
    println!("DIGEST {:016x} {:016x}", fnv1a(bytes(&a)), fnv1a(bytes(&b)));
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::process::exit(0);
}

/// `ERRORS.md` #4 and #6 — negative `size`.
///
/// Observed on this platform: `sizeof(T)*size` sign-extends to a ~2^64 byte
/// count, and glibc's `memcpy` returns without copying for such a size rather
/// than faulting. Control then reaches `recurse(b, 0, size, a)` where
/// `hi - lo == size < 0 <= 1`, so the guard returns immediately — i.e. a
/// negative `size` is a silent no-op. This directly verifies row #6, and both
/// implementations must land on the identical (unchanged) buffers.
#[test]
fn err04_06_negative_size_behaves_identically() {
    for case in ["neg1", "neg2", "neg1000"] {
        let t = assert_same_termination(case);
        eprintln!("ERRORS.md #4/#6 case {case}: C and Rust agree -> {t:?}");
    }
}

/// `ERRORS.md` #5 — `size == INT_MIN` (and one step past).
#[test]
fn err05_int_min_size_behaves_identically() {
    for case in ["intmin", "intmin_plus1"] {
        let t = assert_same_termination(case);
        eprintln!("ERRORS.md #5 case {case}: C and Rust agree -> {t:?}");
    }
}

/// `ERRORS.md` #7 — `a == NULL` with `size > 0` (memcpy reads address 0).
#[test]
fn err07_null_source_behaves_identically() {
    let t = assert_same_termination("null_a");
    assert!(t.is_fatal(), "null source with size>0 should be fatal, got {t:?}");
    eprintln!("ERRORS.md #7 null_a: C and Rust agree -> {t:?}");
}

/// `ERRORS.md` #8 — `b == NULL` with `size > 0` (memcpy writes address 0).
#[test]
fn err08_null_dest_behaves_identically() {
    let t = assert_same_termination("null_b");
    assert!(t.is_fatal(), "null dest with size>0 should be fatal, got {t:?}");
    eprintln!("ERRORS.md #8 null_b: C and Rust agree -> {t:?}");
}

/// `ERRORS.md` #9 — both pointers NULL with `size > 0`.
#[test]
fn err09_both_null_behaves_identically() {
    for case in ["null_both", "null_both_size1"] {
        let t = assert_same_termination(case);
        assert!(t.is_fatal(), "both-null with size>0 should be fatal, got {t:?}");
        eprintln!("ERRORS.md #9 case {case}: C and Rust agree -> {t:?}");
    }
}
