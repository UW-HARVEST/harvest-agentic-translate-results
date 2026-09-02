//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C22). Every call goes through the
//! `.so` exports of BOTH the C reference and the Rust translation.

mod common;

use common::*;
use std::ffi::c_void;

const SEED_CLASSES: [usize; 6] = [
    0,
    1,
    usize::MAX,
    usize::MAX - 1,
    0x0000_0000_FFFF_FFFF,
    0xFFFF_FFFF_0000_0000,
];

/// C1 — `len == 0`, no blocks, `case 0`.
#[test]
fn c1_empty_message() {
    for p in pairs() {
        let mut buf = [0u8; 8];
        assert_hash_eq(&p, &mut buf, 0, 0, "C1 len=0 seed=0");
        // and via a NULL pointer, which the C never dereferences at len==0
        unsafe {
            assert_hash_eq_ptr(&p, std::ptr::null_mut(), 0, 0, "C1 NULL len=0");
        }
    }
}

/// C2 — tail-only, every `switch` arm 1..7, randomized bytes.
#[test]
fn c2_tail_only_all_arms() {
    let mut rng = Rng::new(0xC2_0000_0001);
    for p in pairs() {
        for len in 1..8usize {
            for _ in 0..2000 {
                let mut buf = [0u8; 16];
                rng.fill(&mut buf);
                assert_hash_eq(&p, &mut buf, len, 0, &format!("C2 len={len}"));
            }
        }
    }
}

/// C3 — tail-only with `d[3]` forced below / at-or-above 0x80 (the `case 4`
/// signed-overflow sign-extension path), plus every other tail byte randomized.
#[test]
fn c3_tail_top_byte_classes() {
    let mut rng = Rng::new(0xC3_0000_0001);
    for p in pairs() {
        for len in 4..8usize {
            for high in [false, true] {
                for _ in 0..2000 {
                    let mut buf = [0u8; 16];
                    rng.fill(&mut buf);
                    buf[3] = if high {
                        0x80 | (rng.next_u8() & 0x7F)
                    } else {
                        rng.next_u8() & 0x7F
                    };
                    assert_hash_eq(
                        &p,
                        &mut buf,
                        len,
                        0,
                        &format!("C3 len={len} d3_high={high}"),
                    );
                }
            }
        }
    }
}

/// C4 — exactly one full block, `tail == 0`.
#[test]
fn c4_single_block_no_tail() {
    let mut rng = Rng::new(0xC4_0000_0001);
    for p in pairs() {
        for _ in 0..5000 {
            let mut buf = [0u8; 8];
            rng.fill(&mut buf);
            assert_hash_eq(&p, &mut buf, 8, 0, "C4 len=8");
        }
    }
}

/// C5 — one block plus every tail arm (`len` 9..15).
#[test]
fn c5_single_block_with_tail() {
    let mut rng = Rng::new(0xC5_0000_0001);
    for p in pairs() {
        for len in 9..16usize {
            for _ in 0..2000 {
                let mut buf = [0u8; 24];
                rng.fill(&mut buf);
                assert_hash_eq(&p, &mut buf, len, 0, &format!("C5 len={len}"));
            }
        }
    }
}

/// C6 — many blocks, all residues mod 8, `len` 16..520.
#[test]
fn c6_many_blocks_all_residues() {
    let mut rng = Rng::new(0xC6_0000_0001);
    for p in pairs() {
        let mut buf = vec![0u8; 600];
        for _ in 0..6000 {
            rng.fill(&mut buf);
            let len = 16 + rng.below(505) as usize;
            let seed = SEED_CLASSES[rng.below(SEED_CLASSES.len() as u64) as usize];
            assert_hash_eq(&p, &mut buf, len, seed, &format!("C6 len={len}"));
        }
        // guarantee every residue is hit deterministically too
        for r in 0..8usize {
            for nb in 2..12usize {
                rng.fill(&mut buf);
                let len = nb * 8 + r;
                assert_hash_eq(&p, &mut buf, len, 0, &format!("C6 nb={nb} r={r}"));
            }
        }
    }
}

/// C7..C10 — block-path byte-value classes for `d[3]` and `d[7]` of EVERY block.
#[test]
fn c7_c10_block_top_byte_classes() {
    let mut rng = Rng::new(0xC7_0000_0001);
    for p in pairs() {
        for (row, d3_high, d7_high) in [
            ("C7 lo/lo", false, false),
            ("C8 hi/lo", true, false),
            ("C9 lo/hi", false, true),
            ("C10 hi/hi", true, true),
        ] {
            for nblocks in 1..9usize {
                for tail in 0..8usize {
                    for _ in 0..60 {
                        let len = nblocks * 8 + tail;
                        let mut buf = vec![0u8; len + 8];
                        rng.fill(&mut buf);
                        for b in 0..nblocks {
                            let base = b * 8;
                            buf[base + 3] = if d3_high {
                                0x80 | (rng.next_u8() & 0x7F)
                            } else {
                                rng.next_u8() & 0x7F
                            };
                            buf[base + 7] = if d7_high {
                                0x80 | (rng.next_u8() & 0x7F)
                            } else {
                                rng.next_u8() & 0x7F
                            };
                        }
                        assert_hash_eq(
                            &p,
                            &mut buf,
                            len,
                            0,
                            &format!("{row} nb={nblocks} tail={tail}"),
                        );
                    }
                }
            }
        }
    }
}

/// C11 — all-zero content, isolates the `len << 56` length term.
#[test]
fn c11_all_zero_buffer() {
    for p in pairs() {
        let mut buf = vec![0u8; 96];
        for len in 0..81usize {
            for &seed in SEED_CLASSES.iter() {
                assert_hash_eq(&p, &mut buf, len, seed, &format!("C11 len={len}"));
            }
        }
    }
}

/// C12 — all-0xFF content, maximal sign-extension in every position.
#[test]
fn c12_all_ones_buffer() {
    for p in pairs() {
        let mut buf = vec![0xFFu8; 96];
        for len in 0..81usize {
            for &seed in SEED_CLASSES.iter() {
                assert_hash_eq(&p, &mut buf, len, seed, &format!("C12 len={len}"));
            }
        }
    }
}

/// C13 — seed classes crossed with randomized shapes.
#[test]
fn c13_seed_classes_cross_shapes() {
    let mut rng = Rng::new(0xC13_0000_0001);
    for p in pairs() {
        for &seed in SEED_CLASSES.iter() {
            let mut buf = vec![0u8; 256];
            for _ in 0..3000 {
                rng.fill(&mut buf);
                let len = rng.below(201) as usize;
                assert_hash_eq(&p, &mut buf, len, seed, &format!("C13 seed={seed:#x}"));
            }
        }
    }
}

/// C14 — broad property-style sweep: random seed × random len × random bytes.
#[test]
fn c14_broad_random_sweep() {
    let mut rng = Rng::new(0xC14_0000_0001);
    for p in pairs() {
        let mut buf = vec![0u8; 1100];
        for _ in 0..20000 {
            rng.fill(&mut buf);
            let len = rng.below(1025) as usize;
            let seed = rng.next_u64() as usize;
            assert_hash_eq(&p, &mut buf, len, seed, "C14 random");
        }
    }
}

/// C15 — `len & 0xFF` aliasing across the `len << 56` term.
#[test]
fn c15_length_byte_aliasing() {
    let mut rng = Rng::new(0xC15_0000_0001);
    for p in pairs() {
        // repeating content so only `len` differs materially
        let mut buf: Vec<u8> = (0..600u32).map(|i| (i % 251) as u8).collect();
        for group in [
            vec![0usize, 256, 512],
            vec![1, 257, 513],
            vec![255, 511],
            vec![7, 263, 519],
            vec![8, 264, 520],
        ] {
            for &len in group.iter() {
                for &seed in SEED_CLASSES.iter() {
                    assert_hash_eq(&p, &mut buf, len, seed, &format!("C15 len={len}"));
                }
            }
        }
        for _ in 0..2000 {
            let len = rng.below(600) as usize;
            assert_hash_eq(&p, &mut buf, len, rng.next_u64() as usize, "C15 rand");
        }
    }
}

/// C16 — unaligned buffer starts; the C reads byte-wise so alignment is irrelevant.
#[test]
fn c16_unaligned_starts() {
    let mut rng = Rng::new(0xC16_0000_0001);
    for p in pairs() {
        let mut backing = vec![0u8; 128];
        for off in 0..8usize {
            for len in 0..65usize {
                rng.fill(&mut backing);
                let ptr = unsafe { backing.as_mut_ptr().add(off) } as *mut c_void;
                unsafe {
                    assert_hash_eq_ptr(&p, ptr, len, 0, &format!("C16 off={off} len={len}"));
                }
            }
        }
    }
}

/// C17 — multi-byte element types hashed as raw bytes (endianness row).
#[test]
fn c17_element_types() {
    let mut rng = Rng::new(0xC17_0000_0001);
    for p in pairs() {
        for _ in 0..500 {
            let u16s: Vec<u16> = (0..17).map(|_| rng.next_u64() as u16).collect();
            let u32s: Vec<u32> = (0..17).map(|_| rng.next_u64() as u32).collect();
            let u64s: Vec<u64> = (0..17).map(|_| rng.next_u64()).collect();
            let f64s: Vec<f64> = (0..17)
                .map(|_| f64::from_bits(rng.next_u64()))
                .collect();

            unsafe {
                for n in 0..=u16s.len() {
                    assert_hash_eq_ptr(
                        &p,
                        u16s.as_ptr() as *mut c_void,
                        n * 2,
                        0,
                        "C17 u16",
                    );
                }
                for n in 0..=u32s.len() {
                    assert_hash_eq_ptr(
                        &p,
                        u32s.as_ptr() as *mut c_void,
                        n * 4,
                        0,
                        "C17 u32",
                    );
                }
                for n in 0..=u64s.len() {
                    assert_hash_eq_ptr(
                        &p,
                        u64s.as_ptr() as *mut c_void,
                        n * 8,
                        0,
                        "C17 u64",
                    );
                }
                for n in 0..=f64s.len() {
                    assert_hash_eq_ptr(
                        &p,
                        f64s.as_ptr() as *mut c_void,
                        n * 8,
                        0,
                        "C17 f64",
                    );
                }
            }
        }
    }
}

/// C18 — exact-size heap allocations, so any over-read is an allocator violation.
#[test]
fn c18_exact_size_allocations() {
    let mut rng = Rng::new(0xC18_0000_0001);
    for p in pairs() {
        for len in 0..25usize {
            for _ in 0..500 {
                let mut v: Vec<u8> = (0..len).map(|_| rng.next_u8()).collect();
                assert_eq!(v.len(), len);
                let ptr = if len == 0 {
                    std::ptr::null_mut()
                } else {
                    v.as_mut_ptr() as *mut c_void
                };
                unsafe {
                    assert_hash_eq_ptr(&p, ptr, len, 0, &format!("C18 exact len={len}"));
                }
            }
        }
    }
}

// C19..C22 (the `siphash` stdout-comparison rows) live in
// `tests/siphash_stdout.rs`, which must be a single-test binary so that no other
// test thread writes to fd 1 while it is redirected.
