//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through the `.so` exports
//! of BOTH libraries (loaded with `libloading`); nothing is called directly.

mod common;

use common::{c_siphash_stdout, rust_siphash_stdout, Libs, Rng, FIXED_SEED};
use std::os::raw::c_void;

/// How many randomized inputs per configuration row.
const ITERS: usize = 400;

// ---------------------------------------------------------------------------
// C1 / C2 — empty input
// ---------------------------------------------------------------------------

#[test]
fn c1_len0_nonnull() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED);
    let mut buf = [0u8; 64];
    for _ in 0..ITERS {
        rng.fill(&mut buf);
        libs.assert_hash_eq(&mut buf, 0, 0, "C1 len=0 non-null p");
    }
}

#[test]
fn c2_len0_null_pointer() {
    let libs = Libs::load();
    // len == 0 means the pointer is never dereferenced in the C, so NULL is a
    // legitimate input that both libraries must handle identically.
    for seed in [0usize, 1, usize::MAX, 0x0123_4567_89ab_cdef] {
        let (c, r) = unsafe {
            (
                (libs.c_hash_bytes)(std::ptr::null_mut(), 0, seed),
                (libs.rust_hash_bytes)(std::ptr::null_mut(), 0, seed),
            )
        };
        assert_eq!(c, r, "C2 NULL/len=0 seed={seed:#x}: C={c:#018x} Rust={r:#018x}");
    }
}

// ---------------------------------------------------------------------------
// C3..C10 — every tail `switch` case with len < 8
// ---------------------------------------------------------------------------

fn short_len_row(len: usize, force_d3_high: Option<bool>, ctx: &str) {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ (len as u64) << 32);
    let mut buf = [0u8; 8];
    for _ in 0..ITERS {
        rng.fill(&mut buf);
        if let Some(high) = force_d3_high {
            if len >= 4 {
                if high {
                    buf[3] |= 0x80;
                } else {
                    buf[3] &= 0x7f;
                }
            }
        }
        libs.assert_hash_eq(&mut buf, len, 0, ctx);
    }
}

#[test]
fn c3_len1() {
    // Sweep every possible byte value exhaustively as well as randomized.
    let libs = Libs::load();
    for b in 0u16..=255 {
        let mut buf = [b as u8; 8];
        libs.assert_hash_eq(&mut buf, 1, 0, "C3 len=1 exhaustive byte");
    }
    short_len_row(1, None, "C3 len=1 randomized");
}

#[test]
fn c4_len2() {
    short_len_row(2, None, "C4 len=2");
}

#[test]
fn c5_len3() {
    short_len_row(3, None, "C5 len=3");
}

#[test]
fn c6_len4_d3_low() {
    short_len_row(4, Some(false), "C6 len=4 d[3]<0x80");
}

#[test]
fn c7_len4_d3_high() {
    // The signed-int-overflow sign-extension path in tail `case 4`.
    short_len_row(4, Some(true), "C7 len=4 d[3]>=0x80");
    // Also pin each individual high byte value.
    let libs = Libs::load();
    for b in 0x80u16..=0xff {
        let mut buf = [0u8; 8];
        buf[3] = b as u8;
        libs.assert_hash_eq(&mut buf, 4, 0, "C7 len=4 d[3] sweep 0x80..0xff");
    }
}

#[test]
fn c8_len5() {
    short_len_row(5, Some(false), "C8 len=5 d[3] low");
    short_len_row(5, Some(true), "C8 len=5 d[3] high");
}

#[test]
fn c9_len6() {
    short_len_row(6, Some(false), "C9 len=6 d[3] low");
    short_len_row(6, Some(true), "C9 len=6 d[3] high");
}

#[test]
fn c10_len7() {
    short_len_row(7, Some(false), "C10 len=7 d[3] low");
    short_len_row(7, Some(true), "C10 len=7 d[3] high");
}

// ---------------------------------------------------------------------------
// C11..C15 — block-processing loop
// ---------------------------------------------------------------------------

#[test]
fn c11_len8_one_block() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xb10c);
    let mut buf = [0u8; 8];
    for _ in 0..ITERS {
        rng.fill(&mut buf);
        libs.assert_hash_eq(&mut buf, 8, 0, "C11 len=8 random");
    }
    // All four high-bit polarity combinations of d[3] and d[7].
    for (hi3, hi7) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut rng = Rng::new(FIXED_SEED ^ 0xc11);
        for _ in 0..ITERS {
            rng.fill(&mut buf);
            if hi3 { buf[3] |= 0x80 } else { buf[3] &= 0x7f }
            if hi7 { buf[7] |= 0x80 } else { buf[7] &= 0x7f }
            libs.assert_hash_eq(&mut buf, 8, 0, "C11 len=8 polarity combo");
        }
    }
}

#[test]
fn c12_len8_d3_sign_extension() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc12);
    let mut buf = [0u8; 8];
    for _ in 0..ITERS {
        rng.fill(&mut buf);
        buf[3] |= 0x80; // main-loop low-word signed overflow
        buf[7] &= 0x7f;
        libs.assert_hash_eq(&mut buf, 8, 0, "C12 main loop d[3]>=0x80");
    }
    for b in 0x80u16..=0xff {
        let mut buf = [0u8; 8];
        buf[3] = b as u8;
        libs.assert_hash_eq(&mut buf, 8, 0, "C12 d[3] sweep");
    }
}

#[test]
fn c13_len8_d7_sign_extension() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc13);
    let mut buf = [0u8; 8];
    for _ in 0..ITERS {
        rng.fill(&mut buf);
        buf[3] &= 0x7f;
        buf[7] |= 0x80; // high word: sign bits get shifted out by << 16 << 16
        libs.assert_hash_eq(&mut buf, 8, 0, "C13 main loop d[7]>=0x80");
    }
    for b in 0x80u16..=0xff {
        let mut buf = [0u8; 8];
        buf[7] = b as u8;
        libs.assert_hash_eq(&mut buf, 8, 0, "C13 d[7] sweep");
    }
}

#[test]
fn c14_len8_both_sign_extensions() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc14);
    let mut buf = [0u8; 8];
    for _ in 0..ITERS {
        rng.fill(&mut buf);
        buf[3] |= 0x80;
        buf[7] |= 0x80;
        libs.assert_hash_eq(&mut buf, 8, 0, "C14 both d[3] and d[7] >= 0x80");
    }
}

#[test]
fn c15_len16_two_blocks() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc15);
    let mut buf = [0u8; 16];
    for _ in 0..ITERS {
        rng.fill(&mut buf);
        libs.assert_hash_eq(&mut buf, 16, 0, "C15 len=16");
    }
    // Force the sign-extension paths in both blocks independently.
    for idx in [3usize, 7, 11, 15] {
        let mut rng = Rng::new(FIXED_SEED ^ idx as u64);
        for _ in 0..ITERS {
            rng.fill(&mut buf);
            buf[idx] |= 0x80;
            libs.assert_hash_eq(&mut buf, 16, 0, "C15 len=16 forced high byte");
        }
    }
}

// ---------------------------------------------------------------------------
// C16..C18 — length ranges
// ---------------------------------------------------------------------------

#[test]
fn c16_len_9_to_15() {
    let libs = Libs::load();
    for len in 9usize..=15 {
        let mut rng = Rng::new(FIXED_SEED ^ (len as u64) << 8);
        let mut buf = vec![0u8; 16];
        for _ in 0..ITERS {
            rng.fill(&mut buf);
            libs.assert_hash_eq(&mut buf, len, 0, &format!("C16 len={len}"));
        }
    }
}

#[test]
fn c17_len_17_to_255() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc17);
    let mut buf = vec![0u8; 256];
    // Every length in the range, several random buffers each: covers all
    // (block count x len%8) combinations.
    for len in 17usize..=255 {
        for _ in 0..8 {
            rng.fill(&mut buf);
            libs.assert_hash_eq(&mut buf, len, 0, &format!("C17 len={len}"));
        }
    }
}

#[test]
fn c18_len_256_to_1024_top_byte_aliasing() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc18);
    let mut buf = vec![0u8; 1024];
    rng.fill(&mut buf);
    for len in 256usize..=1024 {
        libs.assert_hash_eq(&mut buf, len, 0, &format!("C18 len={len}"));
    }
    // `len << 56` keeps only len & 0xff: verify C and Rust agree that
    // len and len+256 differ only through the bytes consumed.
    for len in [256usize, 257, 511, 512, 768, 1000, 1024] {
        rng.fill(&mut buf);
        libs.assert_hash_eq(&mut buf, len, 0, &format!("C18 alias len={len}"));
    }
}

// ---------------------------------------------------------------------------
// C19 — extreme value shapes
// ---------------------------------------------------------------------------

#[test]
fn c19_all_zero_and_all_ff() {
    let libs = Libs::load();
    for len in 0usize..=64 {
        let mut zeros = vec![0x00u8; 64];
        let mut ones = vec![0xffu8; 64];
        libs.assert_hash_eq(&mut zeros, len, 0, &format!("C19 zeros len={len}"));
        libs.assert_hash_eq(&mut ones, len, 0, &format!("C19 0xff len={len}"));
    }
    // Alternating and single-bit patterns too.
    for pat in [0x01u8, 0x7f, 0x80, 0xaa, 0x55, 0xfe] {
        for len in 0usize..=17 {
            let mut buf = vec![pat; 32];
            libs.assert_hash_eq(&mut buf, len, 0, &format!("C19 pat={pat:#04x} len={len}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — seed sweep (the C cancels the seed; verified, not assumed)
// ---------------------------------------------------------------------------

#[test]
fn c20_seed_sweep() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc20);
    let mut buf = vec![0u8; 40];
    for _ in 0..ITERS {
        rng.fill(&mut buf);
        let len = (rng.below(41)) as usize;
        for seed in [
            0usize,
            1,
            usize::MAX,
            usize::MAX - 1,
            1usize << 63,
            rng.next_u64() as usize,
            rng.next_u64() as usize,
        ] {
            libs.assert_hash_eq(&mut buf, len, seed, "C20 seed sweep");
        }
    }
}

// ---------------------------------------------------------------------------
// C21 — unaligned pointers
// ---------------------------------------------------------------------------

#[test]
fn c21_unaligned_pointer() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc21);
    let mut backing = vec![0u8; 128];
    for _ in 0..ITERS {
        rng.fill(&mut backing);
        for off in 0usize..=7 {
            for len in [0usize, 1, 7, 8, 9, 16, 33] {
                let p = unsafe { backing.as_mut_ptr().add(off) } as *mut c_void;
                let (c, r) = unsafe {
                    (
                        (libs.c_hash_bytes)(p, len, 0),
                        (libs.rust_hash_bytes)(p, len, 0),
                    )
                };
                assert_eq!(c, r, "C21 unaligned off={off} len={len}: C={c:#018x} Rust={r:#018x}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C22 / C23 — `siphash` stdout, byte-for-byte
// ---------------------------------------------------------------------------

fn assert_siphash_stdout_eq(libs: &Libs, init: i32) {
    let _ = libs;
    let c_out = c_siphash_stdout(init);
    let r_out = rust_siphash_stdout(init);
    assert!(!c_out.is_empty(), "C siphash({init}) produced no output");
    if c_out != r_out {
        // Report the first differing line for a readable failure.
        let cl: Vec<&[u8]> = c_out.split(|b| *b == b'\n').collect();
        let rl: Vec<&[u8]> = r_out.split(|b| *b == b'\n').collect();
        for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
            if a != b {
                panic!(
                    "siphash({init}) stdout differs at line {i}:\n  C   : {}\n  Rust: {}",
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b)
                );
            }
        }
        panic!(
            "siphash({init}) stdout differs in length: C={} bytes, Rust={} bytes",
            c_out.len(),
            r_out.len()
        );
    }
}

#[test]
fn c22_siphash_init_zero() {
    let libs = Libs::load();
    assert_siphash_stdout_eq(&libs, 0);
    // Sanity: 64 rows of 8 hex bytes.
    let out = c_siphash_stdout(0);
    let lines = out.split(|b| *b == b'\n').filter(|l| !l.is_empty()).count();
    assert_eq!(lines, 64, "C siphash prints 64 rows");
}

#[test]
fn c23_siphash_init_sweep() {
    let libs = Libs::load();
    let mut fixed = vec![1i32, 42, 200, 255, 256, -1, i32::MIN, i32::MAX, i32::MAX - 1, -255, -256];
    let mut rng = Rng::new(FIXED_SEED ^ 0xc23);
    for _ in 0..40 {
        fixed.push(rng.next_u64() as i32);
    }
    for init in fixed {
        assert_siphash_stdout_eq(&libs, init);
    }
}

// ---------------------------------------------------------------------------
// C24 — the composed pipeline: siphash() vs 64 direct hash_bytes calls
// ---------------------------------------------------------------------------

#[test]
fn c24_composition_matches_low_level_calls() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xc24);
    let mut inits = vec![0i32, 1, 255, -1, i32::MAX, i32::MIN];
    for _ in 0..10 {
        inits.push(rng.next_u64() as i32);
    }

    for init in inits {
        // Rebuild what the C's `siphash` builds: mem[i] = (unsigned char)(init + i).
        let mut mem = [0u8; 64];
        let mut z: i32 = init;
        for i in 0..64 {
            mem[i] = z as u8;
            z = z.wrapping_add(1);
        }

        // Expected stdout, assembled from the low-level export of EACH library.
        let mut expected_c = Vec::new();
        let mut expected_r = Vec::new();
        for len in 0..64usize {
            let (ch, rh) = libs.both_hash(&mut mem, len, 0);
            assert_eq!(ch, rh, "C24 low-level divergence init={init} len={len}");
            for target in [&mut expected_c, &mut expected_r] {
                target.extend_from_slice(b"  { ");
            }
            for j in 0..8usize {
                expected_c.extend_from_slice(
                    format!("0x{:02x}, ", (ch >> (j * 8)) & 255).as_bytes(),
                );
                expected_r.extend_from_slice(
                    format!("0x{:02x}, ", (rh >> (j * 8)) & 255).as_bytes(),
                );
            }
            for target in [&mut expected_c, &mut expected_r] {
                target.extend_from_slice(b" },\n");
            }
        }

        let c_out = c_siphash_stdout(init);
        let r_out = rust_siphash_stdout(init);
        assert_eq!(
            c_out, expected_c,
            "C24: C siphash({init}) stdout != composition of C stbds_hash_bytes"
        );
        assert_eq!(
            r_out, expected_r,
            "C24: Rust siphash({init}) stdout != composition of Rust stbds_hash_bytes"
        );
        assert_eq!(c_out, r_out, "C24: siphash({init}) stdout differs");
    }
}

// ---------------------------------------------------------------------------
// Broad randomized sweep across the whole (len x data x seed) space.
// Not a CONFIGS.md row on its own — a safety net over all of them.
// ---------------------------------------------------------------------------

#[test]
fn broad_random_fuzz() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xf0f0_f0f0);
    let mut buf = vec![0u8; 2048];

    // Short lengths get the most iterations: that is where the tail-`switch`
    // sign-extension paths live.
    for _ in 0..30_000 {
        let len = rng.below(33) as usize;
        rng.fill(&mut buf[..len.max(1)]);
        let seed = rng.next_u64() as usize;
        libs.assert_hash_eq(&mut buf, len, seed, "fuzz short");
    }

    // Medium and long lengths, including many-block inputs.
    for _ in 0..8_000 {
        let len = rng.below(2049) as usize;
        rng.fill(&mut buf[..len.max(1)]);
        let seed = rng.next_u64() as usize;
        libs.assert_hash_eq(&mut buf, len, seed, "fuzz long");
    }

    // Sparse/structured data: mostly zero with a few high bytes, which is the
    // shape that isolates the signed-overflow branches.
    for _ in 0..20_000 {
        let len = rng.below(65) as usize;
        for b in buf[..len.max(1)].iter_mut() {
            *b = 0;
        }
        if len > 0 {
            let hits = rng.below(4) + 1;
            for _ in 0..hits {
                let pos = rng.below(len as u64) as usize;
                buf[pos] = if rng.next_u64() & 1 == 0 { 0x80 } else { 0xff };
            }
        }
        libs.assert_hash_eq(&mut buf, len, 0, "fuzz sparse-high");
    }
}
