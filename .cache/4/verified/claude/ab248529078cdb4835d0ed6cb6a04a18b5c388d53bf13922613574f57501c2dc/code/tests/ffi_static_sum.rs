//! Phase B — differential tests for the lowest-level entry point,
//! `int static_sum(int)`, called through `dlopen`/`dlsym` on both the C `.so`
//! and the Rust `.so` (CONFIGS.md rows 1–9).

mod common;

use common::{fresh_pair, Pair, Rng, SEED};

const TAG: &str = "ffi_static_sum";

/// Runs the same call sequence on both libraries and compares every result.
///
/// A shared pair of library instances is used on purpose: both implementations
/// see the identical call order, so their `static int sum` state must stay in
/// lock-step call by call (a difference in accumulation shows up immediately).
fn compare_sequence(pair: &Pair, updates: &[i32]) {
    for (i, &u) in updates.iter().enumerate() {
        let c = pair.c.static_sum(u);
        let r = pair.rust.static_sum(u);
        assert_eq!(
            c, r,
            "static_sum divergence at call #{i} (update = {u}) in sequence {updates:?}"
        );
    }
}

// ---------------------------------------------------------------- row 1, 2 ---

#[test]
fn ffi_static_sum_single() {
    let pair = fresh_pair(TAG);

    // row 1: a single call with 0 on a pristine instance.
    assert_eq!(pair.c.static_sum(0), 0);
    assert_eq!(pair.rust.static_sum(0), 0);

    // row 2: small/typical values plus randomized ones.
    for u in [1, -1, 2, -2, 7, -7, 100, -100, 12345, -12345] {
        compare_sequence(&pair, &[u]);
    }
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..500 {
        let small = (rng.below(2001) as i64 - 1000) as i32;
        compare_sequence(&pair, &[small]);
    }
}

// ------------------------------------------------------------------- row 3 ---

#[test]
fn ffi_static_sum_boundaries() {
    let pair = fresh_pair(TAG);

    let mut values = vec![
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0,
        1,
        -1,
        i32::MAX / 2,
        i32::MIN / 2,
    ];
    for k in 0..32 {
        values.push(1i32.wrapping_shl(k));
        values.push((1i32.wrapping_shl(k)).wrapping_neg());
    }
    for v in values {
        compare_sequence(&pair, &[v]);
        compare_sequence(&pair, &[v, v]);
        compare_sequence(&pair, &[v, 1, v, -1]);
    }
}

// ------------------------------------------------------------- rows 4, 5, 6 ---

#[test]
fn ffi_static_sum_sequences() {
    let pair = fresh_pair(TAG);
    let mut rng = Rng::new(SEED ^ 0x22);

    // row 4: two calls, mixed signs.
    for _ in 0..200 {
        let a = (rng.below(200_001) as i64 - 100_000) as i32;
        let b = (rng.below(200_001) as i64 - 100_000) as i32;
        compare_sequence(&pair, &[a, b]);
    }

    // row 5: exactly ten calls, mirroring what `main` does.
    for _ in 0..200 {
        let stride = rng.next_i32();
        let seq: Vec<i32> = (0..10).map(|i| (i as i32).wrapping_mul(stride)).collect();
        compare_sequence(&pair, &seq);
    }

    // row 6: 1..=64 calls of full-range random i32.
    for len in 1..=64usize {
        let seq: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        compare_sequence(&pair, &seq);
    }
    for _ in 0..200 {
        let len = 1 + rng.below(64) as usize;
        let seq: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        compare_sequence(&pair, &seq);
    }
}

// ------------------------------------------------------------------- row 7 ---

#[test]
fn ffi_static_sum_overflow() {
    let pair = fresh_pair(TAG);

    // Deliberate signed overflow of `sum += update` (UB in C, wraps on the
    // target ABI). Both builds must wrap identically instead of trapping.
    compare_sequence(&pair, &[i32::MAX, i32::MAX]);
    compare_sequence(&pair, &[i32::MAX, 1]);
    compare_sequence(&pair, &[i32::MIN, -1]);
    compare_sequence(&pair, &[i32::MIN, i32::MIN]);
    compare_sequence(&pair, &[i32::MAX; 8]);
    compare_sequence(&pair, &[i32::MIN; 8]);
    compare_sequence(&pair, &[1 << 30; 16]);
    compare_sequence(&pair, &[-(1 << 30); 16]);
    compare_sequence(&pair, &[i32::MAX, i32::MIN, i32::MAX, i32::MIN, 1, -1]);

    let mut rng = Rng::new(SEED ^ 0x33);
    for _ in 0..300 {
        // Bias towards huge magnitudes so the accumulator keeps wrapping.
        let seq: Vec<i32> = (0..16)
            .map(|_| {
                let v = rng.next_i32();
                if v % 2 == 0 {
                    v | (1 << 30)
                } else {
                    v
                }
            })
            .collect();
        compare_sequence(&pair, &seq);
    }
}

// ------------------------------------------------------------------- row 8 ---

#[test]
fn ffi_state_is_per_instance() {
    // Two independently loaded copies must each start from sum == 0, in both
    // implementations (this validates the test methodology *and* that Rust's
    // static has the same storage duration semantics as C's).
    let a = fresh_pair(TAG);
    assert_eq!(a.c.static_sum(10), 10);
    assert_eq!(a.rust.static_sum(10), 10);
    assert_eq!(a.c.static_sum(5), 15);
    assert_eq!(a.rust.static_sum(5), 15);

    let b = fresh_pair(TAG);
    assert_eq!(b.c.static_sum(7), 7, "C instance must start at 0");
    assert_eq!(b.rust.static_sum(7), 7, "Rust instance must start at 0");

    // ... and the first pair kept its own state.
    assert_eq!(a.c.static_sum(0), 15);
    assert_eq!(a.rust.static_sum(0), 15);
}

// CONFIGS.md row 9 (state shared between `static_sum` and `main`) lives in
// `tests/ffi_main.rs`, because capturing `main`'s stdout requires a test binary
// with a single `#[test]` function.

/// Calling `static_sum` from several threads is *not* what the C program does,
/// but it pins down the storage duration: the C `static int` is one instance
/// per process, so the totals must keep growing across threads in both builds.
#[test]
fn ffi_static_sum_is_process_wide_not_thread_local() {
    let pair = fresh_pair(TAG);
    let c_first = pair.c.static_sum(100);
    let r_first = pair.rust.static_sum(100);
    assert_eq!(c_first, r_first);

    let (c_other, r_other) = std::thread::scope(|s| {
        s.spawn(|| (pair.c.static_sum(1), pair.rust.static_sum(1)))
            .join()
            .unwrap()
    });
    assert_eq!(
        c_other, r_other,
        "the running total must behave identically when a different thread calls in"
    );
    assert_eq!(c_other, 101, "C keeps one process-wide `sum`");
}
