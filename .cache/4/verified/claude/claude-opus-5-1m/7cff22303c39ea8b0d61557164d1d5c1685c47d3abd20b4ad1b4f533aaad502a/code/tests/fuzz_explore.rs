//! Broad exploratory differential fuzz. Kept in the suite as a wide net that
//! is not tied to a single `CONFIGS.md` row.

mod common;
use common::*;

#[test]
fn wide_random_fuzz() {
    let mut rng = Rng::new(SEED ^ 0xf00d);
    for i in 0..20_000 {
        let s = [rng.pool_f32(), rng.pool_f32(), rng.pool_f32()];
        diff(&format!("wide_random_fuzz #{i}"), &s, 1, 2);
    }
}

#[test]
fn wide_uniform_bits_fuzz() {
    let mut rng = Rng::new(SEED ^ 0xbeef);
    for i in 0..20_000 {
        let s = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
        diff(&format!("wide_uniform_bits_fuzz #{i}"), &s, 1, 2);
    }
}

#[test]
fn wide_multi_element_fuzz() {
    let mut rng = Rng::new(SEED ^ 0xcafe);
    for i in 0..2_000 {
        let n = 1 + rng.below(12);
        let s: Vec<u32> = (0..3 * n).map(|_| rng.pool_f32()).collect();
        diff(&format!("wide_multi_element_fuzz #{i} n={n}"), &s, n as i32, 2 * n);
    }
}
