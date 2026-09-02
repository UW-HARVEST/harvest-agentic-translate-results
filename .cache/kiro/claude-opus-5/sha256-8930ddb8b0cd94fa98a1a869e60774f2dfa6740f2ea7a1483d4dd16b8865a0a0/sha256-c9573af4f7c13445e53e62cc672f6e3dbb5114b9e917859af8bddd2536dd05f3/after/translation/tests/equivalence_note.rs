//! Cross-check that the 8-byte block loop is algebraically identical to eight
//! single-byte tail steps *in the C itself*. This is needed to interpret
//! mutation testing: changing `len >= 8` to `len >= 9` is an EQUIVALENT mutant
//! (it only reroutes work between two loops that compute the same function),
//! not a surviving bug.

mod harness;

use harness::{Impls, Rng, SEED};

#[test]
fn block_loop_equals_byte_at_a_time_in_c() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0xB10C);
    for _ in 0..500 {
        let n = rng.below(200);
        let data = rng.bytes(n);
        let seed = rng.next_u16();

        let one_shot = im.c_call(&data, n as u32, seed);
        let mut byte_at_a_time = seed;
        for b in &data {
            byte_at_a_time = im.c_call(std::slice::from_ref(b), 1, byte_at_a_time);
        }
        assert_eq!(
            one_shot, byte_at_a_time,
            "C's block loop is NOT equivalent to byte-at-a-time for n={n}; \
             `len >= 8` -> `len >= 9` would then be a detectable mutation"
        );

        // And the Rust must have the same internal equivalence.
        let one_shot_r = im.rust_call(&data, n as u32, seed);
        let mut byte_r = seed;
        for b in &data {
            byte_r = im.rust_call(std::slice::from_ref(b), 1, byte_r);
        }
        assert_eq!(one_shot_r, byte_r);
        assert_eq!(one_shot, one_shot_r);
    }
}
