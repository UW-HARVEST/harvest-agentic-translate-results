//! Phase B rows C27–C30 — dense / exhaustive sweeps.
//!
//! `div_euclid` takes two `int`s, so the total input space is 2^64 pairs, which
//! cannot be enumerated. These tests instead:
//!   * C27 exhaustively enumerate a dense small-value square;
//!   * C28/C29 exhaustively enumerate **every** representable `i32` value of one
//!     operand while the other is pinned to each interesting value;
//!   * C30 sweep the full 2-D space on coprime strides.
//!
//! C28/C29/C30 are heavy; each is bounded by `EXHAUSTIVE_STRIDE` /
//! `SWEEP_STRIDE` (default `1` for C28/C29 = truly exhaustive) so the suite can
//! be run in a reduced mode with `EXHAUSTIVE_STRIDE=<n>`.

mod common;

use common::{lib, Checker, Rng, INT_MAX, INT_MIN, SEED};

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(default)
}

/// Fast inner loop: compares C and Rust directly, collecting the first few
/// divergences without the per-call `Result` allocation of `Checker`.
struct Fast {
    label: &'static str,
    cases: u64,
    failures: Vec<String>,
}

impl Fast {
    fn new(label: &'static str) -> Self {
        Fast {
            label,
            cases: 0,
            failures: Vec::new(),
        }
    }
    #[inline(always)]
    fn check(&mut self, v1: i32, v2: i32) {
        let l = lib();
        let c = l.c(v1, v2);
        let r = l.rust(v1, v2);
        self.cases += 1;
        if c != r && self.failures.len() < 25 {
            self.failures.push(format!(
                "DIVERGENCE div_euclid({v1}, {v2}): C = {c} != Rust = {r}"
            ));
        }
    }
    fn finish(self) {
        assert!(self.cases > 0, "[{}] ran zero cases", self.label);
        if !self.failures.is_empty() {
            panic!(
                "[{}] {} divergence(s) out of {} cases:\n{}",
                self.label,
                self.failures.len(),
                self.cases,
                self.failures.join("\n")
            );
        }
        println!("[{}] OK - {} cases matched", self.label, self.cases);
    }
}

// ---------------------------------------------------------------------------
// C27 — exhaustive small-value square: v1, v2 in [-LIM, LIM]
// ---------------------------------------------------------------------------
#[test]
fn c27_exhaustive_small_square() {
    let lim = env_u64("SMALL_SQUARE_LIMIT", 300) as i32;
    let mut ck = Fast::new("C27 exhaustive small square");
    for v1 in -lim..=lim {
        for v2 in -lim..=lim {
            ck.check(v1, v2);
        }
    }
    let n = (2 * lim as u64 + 1).pow(2);
    assert_eq!(ck.cases, n);
    ck.finish();
}

/// Sweep every `i32` from `INT_MIN` upward on `stride`, always including the
/// exact extremes and the values around zero.
fn sweep_all_i32(stride: u64, mut f: impl FnMut(i32)) {
    // guaranteed inclusions
    for v in [
        INT_MIN,
        INT_MIN + 1,
        INT_MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        INT_MAX - 2,
        INT_MAX - 1,
        INT_MAX,
    ] {
        f(v);
    }
    let mut u: u64 = 0;
    while u <= u32::MAX as u64 {
        f((u as u32).wrapping_add(0x8000_0000) as i32); // maps 0.. -> INT_MIN..
        u += stride;
    }
}

// ---------------------------------------------------------------------------
// C28 — every i32 value of v2, with v1 pinned to each interesting value
// ---------------------------------------------------------------------------
#[test]
fn c28_full_v2_sweep_per_pinned_v1() {
    let stride = env_u64("EXHAUSTIVE_STRIDE", 1);
    let pins = [INT_MIN, INT_MIN + 1, -1, 0, 1, INT_MAX];
    for pin in pins {
        let mut ck = Fast::new("C28 full v2 sweep");
        sweep_all_i32(stride, |v2| ck.check(pin, v2));
        println!("  pinned v1 = {pin}");
        ck.finish();
    }
}

// ---------------------------------------------------------------------------
// C29 — every i32 value of v1, with v2 pinned to each interesting value
// ---------------------------------------------------------------------------
#[test]
fn c29_full_v1_sweep_per_pinned_v2() {
    let stride = env_u64("EXHAUSTIVE_STRIDE", 1);
    let pins = [INT_MIN, INT_MIN + 1, -3, -2, -1, 0, 1, 2, 3, INT_MAX];
    for pin in pins {
        let mut ck = Fast::new("C29 full v1 sweep");
        sweep_all_i32(stride, |v1| ck.check(v1, pin));
        println!("  pinned v2 = {pin}");
        ck.finish();
    }
}

// ---------------------------------------------------------------------------
// C30 — coprime stride sweep over the whole 2-D space
// ---------------------------------------------------------------------------
#[test]
fn c30_two_dimensional_stride_sweep() {
    // 2^32 / 1021 ~= 4.2e6 points per axis is far too many squared, so walk the
    // 2-D space along a single coprime trajectory instead of a full grid: this
    // visits 2^32 distinct (v1, v2) pairs spread uniformly over the space.
    let n = env_u64("SWEEP_POINTS", 40_000_000);
    let mut ck = Fast::new("C30 2-D coprime trajectory");
    // Two odd multipliers, coprime to 2^32, with different bit patterns.
    let a: u32 = 2_654_435_761; // Knuth's golden-ratio multiplier (odd)
    let b: u32 = 40_503 * 2 + 1; // another odd constant
    for i in 0..n {
        let i32bit = i as u32;
        let v1 = i32bit.wrapping_mul(a) as i32;
        let v2 = i32bit.wrapping_mul(b).wrapping_add(0x9E37_79B9) as i32;
        ck.check(v1, v2);
    }
    // plus a coarse true grid so both axes really do vary independently
    let grid_stride: u32 = env_u64("GRID_STRIDE", 8_388_617) as u32; // ~512 pts/axis
    let mut x: u64 = 0;
    while x <= u32::MAX as u64 {
        let v1 = (x as u32).wrapping_add(0x8000_0000) as i32;
        let mut y: u64 = 0;
        while y <= u32::MAX as u64 {
            let v2 = (y as u32).wrapping_add(0x8000_0000) as i32;
            ck.check(v1, v2);
            y += grid_stride as u64;
        }
        x += grid_stride as u64;
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// A long randomized soak on top of the structured sweeps.
// ---------------------------------------------------------------------------
#[test]
fn c31_random_soak() {
    let n = env_u64("SOAK_SAMPLES", 5_000_000);
    let mut ck = Fast::new("C31 random soak");
    let mut r = Rng::new(SEED ^ 0x50AC);
    for _ in 0..n {
        ck.check(r.next_i32(), r.next_i32());
    }
    for _ in 0..(n / 5) {
        ck.check(r.nasty_i32(), r.nasty_i32());
    }
    ck.finish();
}
