//! High-volume randomized cross-checks, independent of the CONFIGS.md /
//! ERRORS.md row tests. These use different seeds and exhaustive small grids
//! so a divergence that the per-row inputs happen to miss still shows up.

mod harness;

use harness::{buf_from, same, same_bytes, Pair, Rng};

/// Exhaustive: every `validate_and_normalize` input in a wide window plus
/// every power-of-two neighbourhood across the whole i32 range.
#[test]
fn stress_normalize_exhaustive_window_and_all_bit_patterns() {
    let p = Pair::fresh();
    for v in -2048..=2048 {
        same(&format!("stress: normalize({v})"), p.c.normalize(v), p.rust.normalize(v));
    }
    for shift in 0..32 {
        let base = 1i32.wrapping_shl(shift);
        for delta in -2..=2 {
            for v in [base.wrapping_add(delta), base.wrapping_neg().wrapping_add(delta)] {
                same(&format!("stress: normalize({v})"), p.c.normalize(v), p.rust.normalize(v));
            }
        }
    }
}

/// Exhaustive: `process_octal_string` over a wide window and every bit pattern
/// neighbourhood, comparing the full 64-byte destination buffer.
#[test]
fn stress_octal_exhaustive_window_and_all_bit_patterns() {
    let p = Pair::fresh();
    for v in -4096..=4096 {
        same_bytes(&format!("stress: octal({v})"), &p.c.octal(v), &p.rust.octal(v));
    }
    for shift in 0..32 {
        let base = 1i32.wrapping_shl(shift);
        for delta in -3..=3 {
            for v in [base.wrapping_add(delta), base.wrapping_neg().wrapping_add(delta)] {
                same_bytes(&format!("stress: octal({v})"), &p.c.octal(v), &p.rust.octal(v));
            }
        }
    }
    let mut r = Rng::new(0xF00D);
    for _ in 0..20000 {
        let v = r.i32_any();
        same_bytes(&format!("stress: octal({v})"), &p.c.octal(v), &p.rust.octal(v));
    }
}

/// Exhaustive: every (byte-in-string, search_char) pair for the full 0..=255
/// search space, on strings built from every byte value.
#[test]
fn stress_replace_full_byte_matrix() {
    let p = Pair::fresh();
    for b in 1u16..=255 {
        let body = vec![b'.', b as u8, b'.', b as u8, 0x7F];
        let s = buf_from(&body, 16);
        for ch in 0i32..=256 {
            same_bytes(
                &format!("stress: replace(byte={b}, ch={ch})"),
                &p.c.replace(&s, ch),
                &p.rust.replace(&s, ch),
            );
        }
        // Negative / sign-extended forms of the same search char.
        for ch in [-(b as i32), (b as u8 as i8) as i32, b as i32 - 256, b as i32 + 65536] {
            same_bytes(
                &format!("stress: replace(byte={b}, ch={ch})"),
                &p.c.replace(&s, ch),
                &p.rust.replace(&s, ch),
            );
        }
    }
}

/// Exhaustive `findrep` over a dense small grid, each on FRESH state so the
/// pure input->output mapping is fully covered for small parameters.
#[test]
fn stress_findrep_dense_small_grid_fresh() {
    for a in -2i32..=2 {
        for b in -2i32..=2 {
            let p = Pair::fresh();
            for c in -2i32..=2 {
                for d in -2i32..=2 {
                    same(
                        &format!("stress: findrep({a},{b},{c},{d})"),
                        p.c.findrep(a, b, c, d),
                        p.rust.findrep(a, b, c, d),
                    );
                }
            }
        }
    }
}

/// Very long randomized sequences over several independent seeds, mixing every
/// entry point. This is the deepest state-divergence probe.
#[test]
fn stress_long_mixed_sequences_many_seeds() {
    for seed in [1u64, 2, 3, 0xABCD, 0xDEADBEEF, 0x1234_5678_9ABC_DEF0] {
        let p = Pair::fresh();
        let mut r = Rng::new(seed);
        for i in 0..3000 {
            let a = match r.next_u32() % 3 {
                0 => r.i32_any(),
                1 => r.i32_in(-600, 600),
                _ => r.i32_in(-3, 3),
            };
            let b = match r.next_u32() % 3 {
                0 => r.i32_any(),
                1 => r.i32_in(-600, 600),
                _ => r.i32_in(-3, 3),
            };
            let tag = format!("stress/{seed:#x}: #{i} a={a} b={b}");
            match r.next_u32() % 8 {
                0 => same(&tag, p.c.add(a, b), p.rust.add(a, b)),
                1 => {
                    let m = r.i32_in(-3, 3);
                    same(&tag, p.c.mul(a, m), p.rust.mul(a, m));
                }
                2 => same(&tag, p.c.sub(a, b), p.rust.sub(a, b)),
                3 => {
                    // b == 0 included on purpose (guard path); INT_MIN/-1 avoided.
                    let d = r.i32_in(-6, 6);
                    same(&tag, p.c.div(a, d), p.rust.div(a, d));
                }
                4 => same(&tag, p.c.normalize(a), p.rust.normalize(a)),
                5 => same_bytes(&tag, &p.c.octal(a), &p.rust.octal(a)),
                6 => {
                    let len = (r.next_u32() % 20) as usize;
                    let body: Vec<u8> = (0..len)
                        .map(|_| {
                            let x = r.byte();
                            if x == 0 { 1 } else { x }
                        })
                        .collect();
                    let s = buf_from(&body, 32);
                    let ch = r.i32_any();
                    same_bytes(&tag, &p.c.replace(&s, ch), &p.rust.replace(&s, ch));
                }
                _ => {
                    let (c, d) = (r.i32_any(), r.i32_any());
                    same(&tag, p.c.findrep(a, b, c, d), p.rust.findrep(a, b, c, d));
                }
            }
        }
    }
}

/// Every `findrep` bucket combination, but on a SHARED library and repeated in
/// several different orders, so the state trajectory differs from row 34's.
#[test]
fn stress_findrep_bucket_orders() {
    for seed in [0x11u64, 0x22, 0x33] {
        let p = Pair::fresh();
        let mut r = Rng::new(seed);
        for _ in 0..2000 {
            let mut v = [0i32; 4];
            for slot in v.iter_mut() {
                let b = r.next_u32() % 5;
                *slot = r.bucket(b);
            }
            same(
                &format!("stress/{seed:#x}: findrep({},{},{},{})", v[0], v[1], v[2], v[3]),
                p.c.findrep(v[0], v[1], v[2], v[3]),
                p.rust.findrep(v[0], v[1], v[2], v[3]),
            );
        }
    }
}
