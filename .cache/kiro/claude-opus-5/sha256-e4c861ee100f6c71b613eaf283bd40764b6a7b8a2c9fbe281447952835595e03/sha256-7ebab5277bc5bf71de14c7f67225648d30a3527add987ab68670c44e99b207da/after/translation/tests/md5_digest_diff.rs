//! Differential tests for `md5_digest`, the only public API in
//! `c_src/include/lib.h`. C and Rust are both invoked through their exported
//! `md5_digest` symbol loaded with `libloading`.

mod common;

use common::{Impls, Rng, TflacMd5};

/// Interesting 32-bit values: zeros, all-ones, byte-boundary and sign edges.
const EDGE_WORDS: &[u32] = &[
    0x0000_0000,
    0x0000_0001,
    0x0000_00FF,
    0x0000_0100,
    0x0000_FFFF,
    0x0001_0000,
    0x00FF_FFFF,
    0x0100_0000,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
    0x1234_5678,
    0xDEAD_BEEF,
    0xCAFE_BABE,
    0x0123_4567,
    0x89AB_CDEF,
    0xAAAA_AAAA,
    0x5555_5555,
    0x80FF_007F,
];

#[test]
fn both_libraries_load_and_export_md5_digest() {
    let impls = Impls::load();
    println!("C   .so: {}", impls.c_path.display());
    println!("Rust.so: {}", impls.rust_path.display());
}

#[test]
fn zero_state() {
    let impls = Impls::load();
    impls.assert_matches(&TflacMd5::default());
}

/// The canonical MD5 initialization vector, plus its byte-swapped twin.
#[test]
fn md5_initial_vector() {
    let impls = Impls::load();
    impls.assert_matches(&TflacMd5 {
        a: 0x6745_2301,
        b: 0xEFCD_AB89,
        c: 0x98BA_DCFE,
        d: 0x1032_5476,
    });
    impls.assert_matches(&TflacMd5 {
        a: 0x0123_4567,
        b: 0x89AB_CDEF,
        c: 0xFEDC_BA98,
        d: 0x7654_3210,
    });
}

/// Exercise one field at a time so a swapped/duplicated field is caught.
#[test]
fn single_field_isolation() {
    let impls = Impls::load();
    for &w in EDGE_WORDS {
        for field in 0..4 {
            let mut m = TflacMd5::default();
            match field {
                0 => m.a = w,
                1 => m.b = w,
                2 => m.c = w,
                _ => m.d = w,
            }
            impls.assert_matches(&m);
        }
    }
}

/// Every single-bit-set value in each of the four words.
#[test]
fn walking_one_bits() {
    let impls = Impls::load();
    for bit in 0..32u32 {
        let w = 1u32 << bit;
        impls.assert_matches(&TflacMd5 {
            a: w,
            b: 0,
            c: 0,
            d: 0,
        });
        impls.assert_matches(&TflacMd5 {
            a: 0,
            b: w,
            c: 0,
            d: 0,
        });
        impls.assert_matches(&TflacMd5 {
            a: 0,
            b: 0,
            c: w,
            d: 0,
        });
        impls.assert_matches(&TflacMd5 {
            a: 0,
            b: 0,
            c: 0,
            d: w,
        });
        // And single-bit-clear.
        impls.assert_matches(&TflacMd5 {
            a: !w,
            b: !w,
            c: !w,
            d: !w,
        });
    }
}

/// Distinct values in every field, including the all-ones state.
#[test]
fn distinct_and_saturated_fields() {
    let impls = Impls::load();
    impls.assert_matches(&TflacMd5 {
        a: 0xFFFF_FFFF,
        b: 0xFFFF_FFFF,
        c: 0xFFFF_FFFF,
        d: 0xFFFF_FFFF,
    });
    impls.assert_matches(&TflacMd5 {
        a: 0x0403_0201,
        b: 0x0807_0605,
        c: 0x0C0B_0A09,
        d: 0x100F_0E0D,
    });
}

/// Full cartesian product over the edge words (20^4 = 160_000 cases) is more
/// than needed; sweep pairwise combinations instead, which still covers every
/// (field, value) pairing across the four slots.
#[test]
fn edge_word_pairwise_sweep() {
    let impls = Impls::load();
    for (i, &wa) in EDGE_WORDS.iter().enumerate() {
        for (j, &wb) in EDGE_WORDS.iter().enumerate() {
            impls.assert_matches(&TflacMd5 {
                a: wa,
                b: wb,
                c: EDGE_WORDS[(i + j) % EDGE_WORDS.len()],
                d: EDGE_WORDS[(i * 3 + j * 7) % EDGE_WORDS.len()],
            });
        }
    }
}

#[test]
fn randomized_states() {
    let impls = Impls::load();
    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);
    for _ in 0..20_000 {
        impls.assert_matches(&TflacMd5 {
            a: rng.next_u32(),
            b: rng.next_u32(),
            c: rng.next_u32(),
            d: rng.next_u32(),
        });
    }
}

/// Confirm the output really is little-endian per word, matching the C shifts.
/// This pins the expected bytes independently of both implementations.
#[test]
fn output_is_little_endian_per_word() {
    let impls = Impls::load();
    let mut rng = Rng::new(42);
    for _ in 0..2_000 {
        let m = TflacMd5 {
            a: rng.next_u32(),
            b: rng.next_u32(),
            c: rng.next_u32(),
            d: rng.next_u32(),
        };
        let mut expected = [0u8; 16];
        expected[0..4].copy_from_slice(&m.a.to_le_bytes());
        expected[4..8].copy_from_slice(&m.b.to_le_bytes());
        expected[8..12].copy_from_slice(&m.c.to_le_bytes());
        expected[12..16].copy_from_slice(&m.d.to_le_bytes());

        let (c_buf, rust_buf) = impls.digest_both(&m, 0x77);
        assert_eq!(&c_buf[..16], &expected[..], "C deviates for {m:?}");
        assert_eq!(&rust_buf[..16], &expected[..], "Rust deviates for {m:?}");
    }
}

/// Writing into the middle of a larger buffer: verifies neither side assumes
/// alignment of the `out` pointer.
#[test]
fn unaligned_output_pointer() {
    let impls = Impls::load();
    let mut rng = Rng::new(7);
    for offset in 0..8usize {
        for _ in 0..256 {
            let m = TflacMd5 {
                a: rng.next_u32(),
                b: rng.next_u32(),
                c: rng.next_u32(),
                d: rng.next_u32(),
            };
            let mut c_buf = [0x5Au8; 40];
            let mut rust_buf = [0x5Au8; 40];
            unsafe {
                (impls.c)(&m as *const TflacMd5, c_buf.as_mut_ptr().add(offset));
                (impls.rust)(&m as *const TflacMd5, rust_buf.as_mut_ptr().add(offset));
            }
            assert_eq!(c_buf, rust_buf, "mismatch at offset {offset} for {m:?}");
        }
    }
}

/// Repeated calls with the same input must be idempotent and identical.
#[test]
fn repeated_calls_are_stable() {
    let impls = Impls::load();
    let m = TflacMd5 {
        a: 0x1122_3344,
        b: 0x5566_7788,
        c: 0x99AA_BBCC,
        d: 0xDDEE_FF00,
    };
    let (first_c, first_rust) = impls.digest_both(&m, 0);
    for _ in 0..64 {
        let (c_buf, rust_buf) = impls.digest_both(&m, 0);
        assert_eq!(c_buf, first_c);
        assert_eq!(rust_buf, first_rust);
        assert_eq!(c_buf, rust_buf);
    }
}
