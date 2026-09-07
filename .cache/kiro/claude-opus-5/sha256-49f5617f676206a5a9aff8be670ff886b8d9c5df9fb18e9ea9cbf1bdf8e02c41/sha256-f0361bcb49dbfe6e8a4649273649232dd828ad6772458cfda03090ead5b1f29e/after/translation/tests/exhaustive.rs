//! Exhaustive small-input differential sweep.
//!
//! Randomized fuzzing can miss a divergence that only shows up for one exact
//! arrangement of nibble parity, buffer capacity and ignore-set membership. This
//! module enumerates ALL strings up to a small length over an alphabet with one
//! representative of every byte class the C distinguishes, crossed with every
//! `bin_maxlen` / `ignore` / `hex_end_p` variant. That makes the cross-product
//! of code paths complete rather than merely sampled.

mod common;
use common::*;

/// One representative per byte class in `c_src/src/lib.c`:
/// digit, lower alpha, upper alpha, plain non-hex, ignorable, NUL, high-bit.
const ALPHABET7: &[u8] = &[b'0', b'f', b'F', b'g', b' ', 0x00, 0x80];
/// Reduced alphabet for the deeper (length 5-6) sweep.
const ALPHABET4: &[u8] = &[b'0', b'f', b' ', b'g'];

fn sweep(alphabet: &[u8], max_len: usize, maxlens: &[usize], label: &str) {
    let ignores: [Option<&[u8]>; 3] = [None, Some(b""), Some(b" ")];
    let k = alphabet.len();
    for len in 0..=max_len {
        let total = k.pow(len as u32);
        for idx in 0..total {
            let mut h = Vec::with_capacity(len);
            let mut n = idx;
            for _ in 0..len {
                h.push(alphabet[n % k]);
                n /= k;
            }
            for &maxlen in maxlens {
                for ig in ignores.iter() {
                    for &he in [false, true].iter() {
                        let mut c = Case::new(&h)
                            .bin_cap(maxlen + 1)
                            .bin_maxlen(maxlen)
                            .hex_end(he);
                        c = match ig {
                            None => c.no_ignore(),
                            Some(s) => c.ignore(s),
                        };
                        assert_same(
                            &format!("{label}/len={len} idx={idx} max={maxlen} ig={ig:?} he={he}"),
                            &c,
                        );
                    }
                }
            }
        }
    }
}

/// All strings of length 0..=4 over 7 byte classes x bin_maxlen 0..=3
/// x ignore in {NULL, "", " "} x hex_end_p in {NULL, non-NULL}.
#[test]
fn exhaustive_len0_4_all_byte_classes() {
    sweep(ALPHABET7, 4, &[0, 1, 2, 3], "ex7");
}

/// Deeper: all strings of length 0..=6 over 4 byte classes, same cross-product.
/// Reaches parities and buffer-exhaustion points the shorter sweep cannot.
#[test]
fn exhaustive_len0_6_reduced_alphabet() {
    sweep(ALPHABET4, 6, &[0, 1, 2, 3, 4], "ex4");
}

/// Exhaustive over BOTH nibbles of a single byte: all 256x256 two-byte inputs,
/// across every ignore / hex_end_p / bin_maxlen variant. This pins down the
/// nibble-combining arithmetic (`c_acc | c_val`, `c_val * 16`) completely.
#[test]
fn exhaustive_all_two_byte_inputs() {
    let ignores: [Option<&[u8]>; 3] = [None, Some(b""), Some(b" \t")];
    for a in 0u16..256 {
        for b in 0u16..256 {
            let h = [a as u8, b as u8];
            for ig in ignores.iter() {
                for &he in [false, true].iter() {
                    for maxlen in [0usize, 1, 2] {
                        let mut c = Case::new(&h)
                            .bin_cap(maxlen + 1)
                            .bin_maxlen(maxlen)
                            .hex_end(he);
                        c = match ig {
                            None => c.no_ignore(),
                            Some(s) => c.ignore(s),
                        };
                        assert_same(
                            &format!("ex2/{a:#04x},{b:#04x} ig={ig:?} he={he} max={maxlen}"),
                            &c,
                        );
                    }
                }
            }
        }
    }
}

/// Every single-byte input, every ignore variant, every hex_end variant, every
/// small bin_maxlen -- the minimal-length boundary of the state machine.
#[test]
fn exhaustive_all_single_byte_inputs() {
    let ignores: [Option<&[u8]>; 6] = [
        None,
        Some(b""),
        Some(b" "),
        Some(b"0"),
        Some(b"\x00"),
        Some(b"\x80\xff"),
    ];
    for a in 0u16..256 {
        let h = [a as u8];
        for ig in ignores.iter() {
            for &he in [false, true].iter() {
                for maxlen in [0usize, 1, 2, usize::MAX] {
                    let cap = if maxlen == usize::MAX { 4 } else { maxlen + 1 };
                    let mut c = Case::new(&h).bin_cap(cap).bin_maxlen(maxlen).hex_end(he);
                    c = match ig {
                        None => c.no_ignore(),
                        Some(s) => c.ignore(s),
                    };
                    assert_same(&format!("ex1/{a:#04x} ig={ig:?} he={he} max={maxlen}"), &c);
                }
            }
        }
    }
}
