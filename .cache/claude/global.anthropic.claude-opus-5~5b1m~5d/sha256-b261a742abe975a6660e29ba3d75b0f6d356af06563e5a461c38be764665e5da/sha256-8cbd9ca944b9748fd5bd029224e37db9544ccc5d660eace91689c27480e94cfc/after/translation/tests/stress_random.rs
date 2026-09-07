//! Broad property-style stress test: hundreds of thousands of randomized
//! configurations, all compared byte-for-byte between the C `.so` and the Rust
//! `.so`. Fixed seeds, so any failure is reproducible.
//!
//! This is the test that originally caught the struct-padding divergence, which
//! the per-row `CONFIGS.md` tests with zeroed padding would not have exposed on
//! their own.

mod common;

use common::*;

/// One randomized trial. Returns `Err(message)` on divergence.
fn trial(rng: &mut Rng, max_elems: usize) -> Result<(), String> {
    let n = rng.below(max_elems + 1);
    let guard = rng.below(4);
    let total = (n + guard).max(1);

    let mut a = Arena::new(total * ELEM_SIZE);
    let mut b = Arena::new(total * ELEM_SIZE);
    // Fully random bytes: this randomizes the 4 padding bytes of every element
    // as well as the guard region and `b`'s pre-existing contents.
    a.fill_random(rng);
    b.fill_random(rng);

    // Choose a key distribution for this trial.
    let mode = rng.below(6);
    for i in 0..n {
        let key = match mode {
            0 => rng.next_i32(),                                    // full range
            1 => (rng.next_u32() % 2) as i32,                        // binary
            2 => (rng.next_u32() % 4) as i32,                        // tiny alphabet
            3 => i as i32,                                           // ascending
            4 => (n - i) as i32,                                     // descending
            _ => [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX]
                [rng.below(7)],                                      // extremes
        };
        let tid = match rng.below(3) {
            0 => rng.next_u64(),
            1 => rng.next_u64() % 3,
            _ => [0u64, 1, u64::MAX / 2, u64::MAX][rng.below(4)],
        };
        a.write_fields(i, tid, key);
    }

    let (mut ac, mut bc) = (a.clone(), b.clone());
    let (mut ar, mut br) = (a.clone(), b.clone());
    unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), n as i32) };
    unsafe { rust_merge_sort()(ar.ptr(), br.ptr(), n as i32) };

    if ac.bytes() != ar.bytes() {
        return Err(format!(
            "arena `a` differs (n={n}, mode={mode})\nC   : {:02x?}\nRust: {:02x?}",
            ac.bytes(),
            ar.bytes()
        ));
    }
    if bc.bytes() != br.bytes() {
        return Err(format!(
            "arena `b` differs (n={n}, mode={mode})\nC   : {:02x?}\nRust: {:02x?}",
            bc.bytes(),
            br.bytes()
        ));
    }
    Ok(())
}

#[test]
fn stress_small_inputs_200k_trials() {
    let mut rng = Rng::new(0x5EED_0001);
    for t in 0..200_000u64 {
        if let Err(msg) = trial(&mut rng, 8) {
            panic!("stress_small: divergence at trial {t}: {msg}");
        }
    }
}

#[test]
fn stress_medium_inputs_50k_trials() {
    let mut rng = Rng::new(0x5EED_0002);
    for t in 0..50_000u64 {
        if let Err(msg) = trial(&mut rng, 40) {
            panic!("stress_medium: divergence at trial {t}: {msg}");
        }
    }
}

#[test]
fn stress_large_inputs_2k_trials() {
    let mut rng = Rng::new(0x5EED_0003);
    for t in 0..2_000u64 {
        if let Err(msg) = trial(&mut rng, 600) {
            panic!("stress_large: divergence at trial {t}: {msg}");
        }
    }
}

#[test]
fn stress_aliased_50k_trials() {
    let mut rng = Rng::new(0x5EED_0004);
    for t in 0..50_000u64 {
        let n = rng.below(17);
        let guard = rng.below(3);
        let mut a = Arena::new((n + guard).max(1) * ELEM_SIZE);
        a.fill_random(&mut rng);
        for i in 0..n {
            a.write_fields(i, rng.next_u64() % 5, (rng.next_u32() % 4) as i32);
        }
        let mut ac = a.clone();
        let mut ar = a.clone();
        unsafe {
            let p = ac.ptr();
            c_merge_sort()(p, p, n as i32);
        }
        unsafe {
            let p = ar.ptr();
            rust_merge_sort()(p, p, n as i32);
        }
        assert_eq!(
            ac.bytes(),
            ar.bytes(),
            "stress_aliased: divergence at trial {t} (n={n})"
        );
    }
}

/// Sanity property that both implementations must share: the multiset of
/// 16-byte elements is preserved, and the winning buffer is sorted by
/// `sort_bits`. Checked on the C output and then required of the Rust output
/// byte-for-byte, so a "both are equally wrong" pass is still detected as a
/// change in behaviour if the C ever moves.
#[test]
fn stress_permutation_invariant() {
    let mut rng = Rng::new(0x5EED_0005);
    for t in 0..20_000u64 {
        let n = 1 + rng.below(24);
        let mut a = Arena::new(n * ELEM_SIZE);
        let mut b = Arena::new(n * ELEM_SIZE);
        a.fill_random(&mut rng);
        b.fill_random(&mut rng);
        for i in 0..n {
            a.write_fields(i, rng.next_u64(), (rng.next_u32() % 6) as i32);
        }
        let mut input: Vec<[u8; 16]> = (0..n)
            .map(|i| {
                let mut e = [0u8; 16];
                e.copy_from_slice(&a.bytes()[i * ELEM_SIZE..(i + 1) * ELEM_SIZE]);
                e
            })
            .collect();
        input.sort_unstable();

        let (mut ac, mut bc) = (a.clone(), b.clone());
        let (mut ar, mut br) = (a.clone(), b.clone());
        unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), n as i32) };
        unsafe { rust_merge_sort()(ar.ptr(), br.ptr(), n as i32) };
        assert_eq!(ac.bytes(), ar.bytes(), "perm: `a` differs at trial {t}");
        assert_eq!(bc.bytes(), br.bytes(), "perm: `b` differs at trial {t}");

        // Whichever buffer holds the final run is a permutation of the input and
        // is non-decreasing in `sort_bits`.
        let sorted_by_key = |arena: &Arena| -> bool {
            (1..n).all(|i| {
                let k = |j: usize| {
                    i32::from_le_bytes(
                        arena.bytes()[j * ELEM_SIZE + 8..j * ELEM_SIZE + 12]
                            .try_into()
                            .unwrap(),
                    )
                };
                k(i - 1) <= k(i)
            })
        };
        let is_perm = |arena: &Arena| -> bool {
            let mut got: Vec<[u8; 16]> = (0..n)
                .map(|i| {
                    let mut e = [0u8; 16];
                    e.copy_from_slice(&arena.bytes()[i * ELEM_SIZE..(i + 1) * ELEM_SIZE]);
                    e
                })
                .collect();
            got.sort_unstable();
            got == input
        };
        let winner_is_a = sorted_by_key(&ac) && is_perm(&ac);
        let winner_is_b = sorted_by_key(&bc) && is_perm(&bc);
        assert!(
            winner_is_a || winner_is_b,
            "perm: neither buffer holds a sorted permutation at trial {t} (n={n})"
        );
    }
}
