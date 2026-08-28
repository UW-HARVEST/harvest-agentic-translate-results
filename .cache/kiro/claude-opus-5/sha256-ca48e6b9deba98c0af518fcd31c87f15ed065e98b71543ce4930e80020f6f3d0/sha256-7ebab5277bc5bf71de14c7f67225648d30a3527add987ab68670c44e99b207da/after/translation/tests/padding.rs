//! Probes the only observable behaviour the main suite deliberately holds
//! constant: the 4 bytes of tail padding in `spritebatch_sprite_t`.
//!
//! `merge_sort` starts with `memcpy(b, a, sizeof(spritebatch_sprite_t) * size)`,
//! which copies padding, and then propagates elements with struct assignment
//! (`b[k] = a[i]`), which may or may not. A caller comparing raw bytes can see
//! the result, so the Rust translation has to move padding the same way the C
//! compiler does.

mod common;

use common::{Impls, Rng, Sprite, as_bytes};

fn with_padding(texture_id: u64, sort_bits: i32, pad: u32) -> Sprite {
    Sprite {
        texture_id,
        sort_bits,
        _pad: pad,
    }
}

#[test]
fn nonzero_padding_propagates_identically() {
    let impls = Impls::load();
    let mut rng = Rng::new(0x5AD0_BEEF);

    for size in 0..=48i32 {
        let n = size as usize;
        for round in 0..4 {
            let input: Vec<Sprite> = (0..n)
                .map(|i| {
                    let bits = match round {
                        0 => (n - i) as i32, // reverse: maximum element movement
                        1 => 0,              // all ties
                        2 => i as i32,
                        _ => rng.below(8) as i32 - 4,
                    };
                    // Distinct, non-zero padding per element makes any
                    // divergence in padding movement visible.
                    with_padding(i as u64, bits, 0xA5A5_0000 | (i as u32 + 1))
                })
                .collect();

            impls.assert_matches(&input, size, &format!("padding size={size} round={round}"));
        }
    }
}

#[test]
fn nonzero_padding_in_destination_buffer() {
    let impls = Impls::load();

    // Pre-poison the scratch buffer `b` as well, so untouched padding bytes in
    // the destination are also part of the comparison.
    for size in 0..=33i32 {
        let n = size as usize;
        let input: Vec<Sprite> = (0..n)
            .map(|i| with_padding(0xFFFF_FFFF_0000_0000 + i as u64, -(i as i32), 0x1234_5678))
            .collect();

        let mut c_a = input.clone();
        let mut r_a = input.clone();
        let mut c_b: Vec<Sprite> = (0..n)
            .map(|i| with_padding(0xDEAD_0000 + i as u64, 0x7BAD_BAD0u32 as i32, 0xCAFE_BABE))
            .collect();
        let mut r_b = c_b.clone();

        unsafe {
            (impls.c_merge_sort)(c_a.as_mut_ptr(), c_b.as_mut_ptr(), size);
            (impls.rust_merge_sort)(r_a.as_mut_ptr(), r_b.as_mut_ptr(), size);
        }

        assert_eq!(
            as_bytes(&c_a),
            as_bytes(&r_a),
            "poisoned-dest `a` mismatch (size={size})"
        );
        assert_eq!(
            as_bytes(&c_b),
            as_bytes(&r_b),
            "poisoned-dest `b` mismatch (size={size})"
        );
    }
}
