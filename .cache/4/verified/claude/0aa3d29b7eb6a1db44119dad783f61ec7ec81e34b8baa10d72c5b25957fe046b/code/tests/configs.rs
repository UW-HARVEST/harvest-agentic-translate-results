//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared libraries (C and Rust) through their exported
//! `tool_basename` symbol and requires byte-identical results: the returned
//! pointer's offset, the returned string's bytes, and the (unmodified) input
//! buffer.

mod common;

use common::*;

/// bytes that are neither separator nor NUL
fn plain_byte(rng: &mut Rng) -> u8 {
    loop {
        let b = rng.nonzero_byte();
        if b != b'/' && b != b'\\' {
            return b;
        }
    }
}

fn plain_string(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| plain_byte(rng)).collect()
}

// ------------------------------------------------- harness provenance / sanity
#[test]
fn c0_harness_loads_two_distinct_shared_objects() {
    let c = c_impl();
    let r = rust_impl();
    println!("C   .so: {}", c.path.display());
    println!("Rust.so: {}", r.path.display());
    assert!(c.path.exists() && r.path.exists());
    assert_ne!(
        c.path.canonicalize().unwrap(),
        r.path.canonicalize().unwrap(),
        "the two implementations must come from different shared objects"
    );
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        assert_eq!(
            r.path.canonicalize().unwrap(),
            std::path::PathBuf::from(p).canonicalize().unwrap(),
            "RUST_DRIVER_SO was not honoured"
        );
    }
    // Prove the calls really reach the libraries (a no-op harness would pass
    // every comparison trivially).
    let mut b1 = b"a/b\0".to_vec();
    let mut b2 = b1.clone();
    assert_eq!(run_one(c, &mut b1, 0).offset, 2);
    assert_eq!(run_one(r, &mut b2, 0).offset, 2);
    let mut b1 = b"a\\bc\0".to_vec();
    let mut b2 = b1.clone();
    assert_eq!(run_one(c, &mut b1, 0).string, b"bc".to_vec());
    assert_eq!(run_one(r, &mut b2, 0).string, b"bc".to_vec());
}

// ---------------------------------------------------------------- C1
#[test]
fn c1_empty_string() {
    diff(b"");
    diff_at(&[0], 0);
}

// ---------------------------------------------------------------- C2
#[test]
fn c2_no_separator_random_ascii() {
    let mut rng = Rng::new(SEED ^ 2);
    let alphabet: Vec<u8> = (0x20u8..=0x7e).filter(|&b| b != b'/' && b != b'\\').collect();
    for _ in 0..4000 {
        let len = 1 + rng.below(64);
        let s: Vec<u8> = (0..len).map(|_| rng.pick(&alphabet)).collect();
        diff(&s);
    }
}

// ---------------------------------------------------------------- C3
#[test]
fn c3_no_separator_length_sweep() {
    let mut rng = Rng::new(SEED ^ 3);
    for len in 0..=129usize {
        for _ in 0..20 {
            diff(&plain_string(&mut rng, len));
        }
    }
}

// ---------------------------------------------------------------- C4
#[test]
fn c4_high_bit_bytes_only() {
    let mut rng = Rng::new(SEED ^ 4);
    // Every byte has the sign bit set: `char` is signed on x86-64, so a naive
    // comparison against '/' (0x2f) could go wrong.
    for len in 0..=80usize {
        for _ in 0..10 {
            let s: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() as u8 & 0x7f)).collect();
            diff(&s);
        }
    }
    // and the exhaustive single-byte case
    for b in 0x80u8..=0xff {
        diff(&[b]);
        diff(&[b, b'/', b]);
        diff(&[b, b'\\', b]);
    }
}

// ---------------------------------------------------------------- C5 / C10
fn one_interior_separator(sep: u8, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..3000 {
        let len = 3 + rng.below(60);
        let mut s = plain_string(&mut rng, len);
        let pos = 1 + rng.below(len - 2);
        s[pos] = sep;
        diff(&s);
    }
}

#[test]
fn c5_single_slash_interior() {
    one_interior_separator(b'/', SEED ^ 5);
}

#[test]
fn c10_single_backslash_interior() {
    one_interior_separator(b'\\', SEED ^ 10);
}

// ---------------------------------------------------------------- C6 / C11
fn many_separators(sep: u8, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..4000 {
        let len = 1 + rng.below(80);
        let mut s = plain_string(&mut rng, len);
        let count = 1 + rng.below(8);
        for _ in 0..count {
            let p = rng.below(len);
            s[p] = sep;
        }
        diff(&s);
    }
}

#[test]
fn c6_many_slashes() {
    many_separators(b'/', SEED ^ 6);
}

#[test]
fn c11_many_backslashes() {
    many_separators(b'\\', SEED ^ 11);
}

// ---------------------------------------------------------------- C7 / C12
fn leading_separator(sep: u8, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..2000 {
        let len = 1 + rng.below(48);
        let mut s = plain_string(&mut rng, len);
        s[0] = sep;
        diff(&s);
    }
    diff(&[sep]);
    diff(&[sep, b'a']);
}

#[test]
fn c7_leading_slash() {
    leading_separator(b'/', SEED ^ 7);
}

#[test]
fn c12_leading_backslash() {
    leading_separator(b'\\', SEED ^ 12);
}

// ---------------------------------------------------------------- C8 / C13
fn trailing_separator(sep: u8, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..2000 {
        let len = 1 + rng.below(48);
        let mut s = plain_string(&mut rng, len);
        *s.last_mut().unwrap() = sep;
        diff(&s);
    }
    diff(&[sep]);
    diff(&[b'a', sep]);
}

#[test]
fn c8_trailing_slash() {
    trailing_separator(b'/', SEED ^ 8);
}

#[test]
fn c13_trailing_backslash() {
    trailing_separator(b'\\', SEED ^ 13);
}

// ---------------------------------------------------------------- C9 / C14
fn only_separator_runs(sep: u8) {
    for len in 1..=257usize {
        diff(&vec![sep; len]);
    }
}

#[test]
fn c9_only_slashes() {
    only_separator_runs(b'/');
}

#[test]
fn c14_only_backslashes() {
    only_separator_runs(b'\\');
}

// ---------------------------------------------------------------- C15 / C16
/// `first` occurs last in the string (so it decides the result).
fn both_separators_ordered(last_sep: u8, other_sep: u8, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..4000 {
        let len = 4 + rng.below(60);
        let mut s = plain_string(&mut rng, len);
        // put `other_sep` somewhere in the first half, `last_sep` after it
        let a = rng.below(len / 2);
        let b = len / 2 + rng.below(len - len / 2);
        s[a] = other_sep;
        s[b] = last_sep;
        assert!(a < b);
        diff(&s);
    }
}

#[test]
fn c15_both_slash_last() {
    both_separators_ordered(b'/', b'\\', SEED ^ 15);
}

#[test]
fn c16_both_backslash_last() {
    both_separators_ordered(b'\\', b'/', SEED ^ 16);
}

// ---------------------------------------------------------------- C17
#[test]
fn c17_adjacent_pair_at_end() {
    let mut rng = Rng::new(SEED ^ 17);
    for pair in [[b'/', b'\\'], [b'\\', b'/'], [b'/', b'/'], [b'\\', b'\\']] {
        for len in 0..=40usize {
            let mut s = plain_string(&mut rng, len);
            s.extend_from_slice(&pair);
            diff(&s);
            // …and with one trailing character after the pair
            let mut t = s.clone();
            t.push(b'x');
            diff(&t);
        }
    }
}

// ---------------------------------------------------------------- C18
#[test]
fn c18_separators_at_both_ends() {
    let mut rng = Rng::new(SEED ^ 18);
    for first in [b'/', b'\\'] {
        for last in [b'/', b'\\'] {
            for _ in 0..600 {
                let len = 2 + rng.below(40);
                let mut s = plain_string(&mut rng, len);
                s[0] = first;
                *s.last_mut().unwrap() = last;
                diff(&s);
            }
        }
    }
}

// ---------------------------------------------------------------- C19
#[test]
fn c19_dense_separator_alphabet() {
    let mut rng = Rng::new(SEED ^ 19);
    let alphabet = [b'/', b'\\', b'a'];
    for _ in 0..20000 {
        let len = rng.below(49);
        let s: Vec<u8> = (0..len).map(|_| rng.pick(&alphabet)).collect();
        diff(&s);
    }
}

// ---------------------------------------------------------------- C20
#[test]
fn c20_full_random_alphabet() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..8000 {
        let len = rng.below(257);
        let s: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        diff(&s);
    }
    // biased variant: 25% chance of a separator at each position
    for _ in 0..8000 {
        let len = rng.below(129);
        let s: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'/',
                1 => b'\\',
                _ => plain_byte(&mut rng),
            })
            .collect();
        diff(&s);
    }
}

// ---------------------------------------------------------------- C21
#[test]
fn c21_large_inputs_with_separators() {
    let mut rng = Rng::new(SEED ^ 21);
    for size in [1usize << 20, 4usize << 20] {
        let base = plain_string(&mut rng, size);
        for &pos in &[0usize, 1, 7, size / 2, size - 2, size - 1] {
            for sep in [b'/', b'\\'] {
                let mut s = base.clone();
                s[pos] = sep;
                diff(&s);
            }
        }
        // both separators, far apart, in both orders
        for (a, b) in [(1usize, size - 2), (size - 2, 1)] {
            let mut s = base.clone();
            s[a] = b'/';
            s[b] = b'\\';
            diff(&s);
        }
    }
}

// ---------------------------------------------------------------- C22
#[test]
fn c22_large_input_no_separator() {
    let mut rng = Rng::new(SEED ^ 22);
    let s = plain_string(&mut rng, 1 << 20);
    diff(&s);
}

// ---------------------------------------------------------------- C23
#[test]
fn c23_unaligned_string_start() {
    let mut rng = Rng::new(SEED ^ 23);
    for start in 0..16usize {
        for len in 0..=40usize {
            for _ in 0..6 {
                // `start` bytes of unrelated data (may contain separators!) come
                // first; they must not influence the result.
                let mut buf: Vec<u8> = (0..start)
                    .map(|_| match rng.below(3) {
                        0 => b'/',
                        1 => b'\\',
                        _ => plain_byte(&mut rng),
                    })
                    .collect();
                let s: Vec<u8> = (0..len)
                    .map(|_| match rng.below(4) {
                        0 => b'/',
                        1 => b'\\',
                        _ => plain_byte(&mut rng),
                    })
                    .collect();
                buf.extend_from_slice(&s);
                buf.push(0);
                diff_at(&buf, start);
            }
        }
    }
}

// ---------------------------------------------------------------- C24
#[test]
fn c24_garbage_after_nul() {
    let mut rng = Rng::new(SEED ^ 24);
    for len in 0..=40usize {
        for _ in 0..20 {
            let mut buf: Vec<u8> = (0..len)
                .map(|_| match rng.below(4) {
                    0 => b'/',
                    1 => b'\\',
                    _ => plain_byte(&mut rng),
                })
                .collect();
            buf.push(0);
            // trailing junk full of separators after the terminator
            let tail_len = 1 + rng.below(32);
            for _ in 0..tail_len {
                buf.push(rng.pick(&[b'/', b'\\', b'z']));
            }
            buf.push(0);
            diff_at(&buf, 0);
        }
    }
}

// ---------------------------------------------------------------- C25
#[test]
fn c25_result_fed_back_in() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..4000 {
        let len = rng.below(64);
        let mut buf: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'/',
                1 => b'\\',
                _ => plain_byte(&mut rng),
            })
            .collect();
        buf.push(0);
        diff_twice(&buf, 0);
    }
    // degenerate cases: empty result, all separators
    for s in [&b""[..], &b"/"[..], &b"\\"[..], &b"a/"[..], &b"a\\"[..], &b"//\\\\"[..]] {
        let mut v = s.to_vec();
        v.push(0);
        diff_twice(&v, 0);
    }
}

// ---------------------------------------------------------------- C26
#[test]
fn c26_exhaustive_single_separator_placement() {
    let mut rng = Rng::new(SEED ^ 26);
    for sep in [b'/', b'\\'] {
        for i in 0..=64usize {
            // minimal length: separator is the last byte
            let mut s = plain_string(&mut rng, i);
            s.push(sep);
            diff(&s);
            // fixed length 65: separator at every position
            let mut t = plain_string(&mut rng, 65);
            t[i] = sep;
            diff(&t);
        }
    }
}

// ---------------------------------------------------------------- C27
#[test]
fn c27_exhaustive_pairwise_placement() {
    let mut rng = Rng::new(SEED ^ 27);
    const N: usize = 8;
    for i in 0..N {
        for j in 0..N {
            let mut s = plain_string(&mut rng, N);
            s[i] = b'/';
            s[j] = b'\\'; // when i == j the backslash wins
            diff(&s);
        }
    }
    // same, but at a 16/32-byte block boundary crossing
    for n in [15usize, 16, 17, 31, 32, 33] {
        for i in 0..n {
            for j in 0..n {
                let mut s = plain_string(&mut rng, n);
                s[i] = b'/';
                s[j] = b'\\';
                diff(&s);
            }
        }
    }
}

// ---------------------------------------------------------------- C28
#[test]
fn c28_pointer_identity_property() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..20000 {
        let len = rng.below(96);
        let s: Vec<u8> = (0..len)
            .map(|_| match rng.below(5) {
                0 => b'/',
                1 => b'\\',
                _ => rng.nonzero_byte(),
            })
            .collect();
        let mut b_c = s.clone();
        b_c.push(0);
        let mut b_r = b_c.clone();
        let out_c = run_one(c_impl(), &mut b_c, 0);
        let out_r = run_one(rust_impl(), &mut b_r, 0);
        assert_eq!(out_c, out_r, "divergence for {}", Esc(&s));
        assert_eq!(out_c.offset, model(&s), "C vs model for {}", Esc(&s));
        assert!(out_c.offset >= 0 && out_c.offset <= len as isize);
        assert_eq!(out_c.string, s[out_c.offset as usize..].to_vec());
    }
}
