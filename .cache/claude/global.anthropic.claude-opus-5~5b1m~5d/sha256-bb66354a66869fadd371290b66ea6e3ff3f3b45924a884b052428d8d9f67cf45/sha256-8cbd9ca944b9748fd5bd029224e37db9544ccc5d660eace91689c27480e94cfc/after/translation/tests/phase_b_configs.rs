//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//! Every call goes through the `.so` export of BOTH libraries via `libloading`.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// C01 / C02 — empty input
// ---------------------------------------------------------------------------

#[test]
fn cfg_c01_len0_seed0() {
    let mut buf = [0u8; 8];
    assert_hash_eq(&mut buf, 0, 0, "C01");
}

#[test]
fn cfg_c02_len0_seed_sweep() {
    let mut buf = [0u8; 8];
    for seed in [0usize, 1, usize::MAX, usize::MAX / 2, usize::MAX - 1, 0xdead_beef] {
        assert_hash_eq(&mut buf, 0, seed, &format!("C02 seed={:#x}", seed));
    }
    let mut rng = Rng::new(SEED ^ 0x02);
    for _ in 0..N {
        let seed = rng.next_u64() as usize;
        assert_hash_eq(&mut buf, 0, seed, "C02 random seed");
    }
}

// ---------------------------------------------------------------------------
// C03..C09 — L=0 blocks, each tail arm (len 1..7)
// ---------------------------------------------------------------------------

fn tail_arm_row(len: usize, row: &str) {
    let mut rng = Rng::new(SEED ^ (len as u64) << 8);
    let mut buf = [0u8; 8];
    for _ in 0..N {
        rng.fill(&mut buf);
        let seed = rng.next_u64() as usize;
        assert_hash_eq(&mut buf, len, 0, row);
        assert_hash_eq(&mut buf, len, seed, row);
    }
    // exhaustive over the single significant byte when len == 1
    if len == 1 {
        for b in 0u16..=255 {
            buf[0] = b as u8;
            assert_hash_eq(&mut buf, 1, 0, "C03 exhaustive byte");
        }
    }
}

#[test]
fn cfg_c03_len1() {
    tail_arm_row(1, "C03");
}
#[test]
fn cfg_c04_len2() {
    tail_arm_row(2, "C04");
}
#[test]
fn cfg_c05_len3() {
    tail_arm_row(3, "C05");
}
#[test]
fn cfg_c06_len4() {
    tail_arm_row(4, "C06");
}
#[test]
fn cfg_c07_len5() {
    tail_arm_row(5, "C07");
}
#[test]
fn cfg_c08_len6() {
    tail_arm_row(6, "C08");
}
#[test]
fn cfg_c09_len7() {
    tail_arm_row(7, "C09");
}

// ---------------------------------------------------------------------------
// C10 / C11 / C12 — block-count paths
// ---------------------------------------------------------------------------

#[test]
fn cfg_c10_len8() {
    let mut rng = Rng::new(SEED ^ 0x10);
    let mut buf = [0u8; 8];
    for _ in 0..N {
        rng.fill(&mut buf);
        let seed = rng.next_u64() as usize;
        assert_hash_eq(&mut buf, 8, 0, "C10");
        assert_hash_eq(&mut buf, 8, seed, "C10 seeded");
    }
}

#[test]
fn cfg_c11_one_block_plus_each_tail() {
    let mut rng = Rng::new(SEED ^ 0x11);
    let mut buf = [0u8; 16];
    for len in 9..=15usize {
        for _ in 0..N {
            rng.fill(&mut buf);
            let seed = rng.next_u64() as usize;
            let ctx = format!("C11 len={}", len);
            assert_hash_eq(&mut buf, len, 0, &ctx);
            assert_hash_eq(&mut buf, len, seed, &ctx);
        }
    }
}

#[test]
fn cfg_c12_len16() {
    let mut rng = Rng::new(SEED ^ 0x12);
    let mut buf = [0u8; 16];
    for _ in 0..N {
        rng.fill(&mut buf);
        let seed = rng.next_u64() as usize;
        assert_hash_eq(&mut buf, 16, 0, "C12");
        assert_hash_eq(&mut buf, 16, seed, "C12 seeded");
    }
}

// ---------------------------------------------------------------------------
// C13 — full cross-product blocks x tails
// ---------------------------------------------------------------------------

#[test]
fn cfg_c13_blocks_x_tails_crossproduct() {
    let mut rng = Rng::new(SEED ^ 0x13);
    let mut buf = vec![0u8; 128];
    for blocks in 2..=8usize {
        for tail in 0..=7usize {
            let len = blocks * 8 + tail;
            for _ in 0..64 {
                rng.fill(&mut buf);
                let seed = rng.next_u64() as usize;
                let ctx = format!("C13 blocks={} tail={} len={}", blocks, tail, len);
                assert_hash_eq(&mut buf, len, 0, &ctx);
                assert_hash_eq(&mut buf, len, seed, &ctx);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C14 / C15 — tail byte-3 high bit (the `d[3] << 24` int-overflow path)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c14_tail_highbit_offset3() {
    let mut rng = Rng::new(SEED ^ 0x14);
    let mut buf = [0u8; 8];
    for len in 4..=7usize {
        for hi in 0x80u16..=0xff {
            rng.fill(&mut buf);
            buf[3] = hi as u8;
            let ctx = format!("C14 len={} d[3]={:#02x}", len, hi);
            assert_hash_eq(&mut buf, len, 0, &ctx);
        }
    }
}

#[test]
fn cfg_c15_tail_lowbit_offset3() {
    let mut rng = Rng::new(SEED ^ 0x15);
    let mut buf = [0u8; 8];
    for len in 4..=7usize {
        for lo in 0x00u16..=0x7f {
            rng.fill(&mut buf);
            buf[3] = lo as u8;
            let ctx = format!("C15 len={} d[3]={:#02x}", len, lo);
            assert_hash_eq(&mut buf, len, 0, &ctx);
        }
    }
}

// ---------------------------------------------------------------------------
// C16 — body-block bytes 3 and 7 high-bit combinations
// ---------------------------------------------------------------------------

#[test]
fn cfg_c16_body_highbit_combos() {
    let mut rng = Rng::new(SEED ^ 0x16);
    for blocks in 1..=4usize {
        let mut buf = vec![0u8; blocks * 8 + 8];
        for b3_high in [false, true] {
            for b7_high in [false, true] {
                for _ in 0..64 {
                    rng.fill(&mut buf);
                    for blk in 0..blocks {
                        let base = blk * 8;
                        buf[base + 3] = if b3_high {
                            0x80 | (buf[base + 3] & 0x7f)
                        } else {
                            buf[base + 3] & 0x7f
                        };
                        buf[base + 7] = if b7_high {
                            0x80 | (buf[base + 7] & 0x7f)
                        } else {
                            buf[base + 7] & 0x7f
                        };
                    }
                    for tail in 0..=7usize {
                        let len = blocks * 8 + tail;
                        let seed = rng.next_u64() as usize;
                        let ctx = format!(
                            "C16 blocks={} tail={} b3_high={} b7_high={}",
                            blocks, tail, b3_high, b7_high
                        );
                        assert_hash_eq(&mut buf, len, 0, &ctx);
                        assert_hash_eq(&mut buf, len, seed, &ctx);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C17 / C18 — degenerate buffers
// ---------------------------------------------------------------------------

#[test]
fn cfg_c17_all_zero_buffer() {
    let mut buf = vec![0u8; 80];
    for len in 0..=71usize {
        assert_hash_eq(&mut buf, len, 0, &format!("C17 len={}", len));
    }
}

#[test]
fn cfg_c18_all_ff_buffer() {
    let mut buf = vec![0xffu8; 80];
    for len in 0..=71usize {
        assert_hash_eq(&mut buf, len, 0, &format!("C18 len={}", len));
        assert_hash_eq(&mut buf, len, usize::MAX, &format!("C18 len={} maxseed", len));
    }
}

// ---------------------------------------------------------------------------
// C19 — len >= 256 (`len << 56` keeps only the low byte)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c19_large_len() {
    let mut rng = Rng::new(SEED ^ 0x19);
    for len in [255usize, 256, 257, 263, 511, 512, 1024, 4096] {
        let mut buf = vec![0u8; len + 8];
        for _ in 0..16 {
            rng.fill(&mut buf);
            let seed = rng.next_u64() as usize;
            let ctx = format!("C19 len={}", len);
            assert_hash_eq(&mut buf, len, 0, &ctx);
            assert_hash_eq(&mut buf, len, seed, &ctx);
        }
    }
    // len == 0 and len == 256 inject the same length byte but differ in data.
    let mut buf = vec![0u8; 264];
    let a = assert_hash_eq(&mut buf, 0, 0, "C19 len0");
    let b = assert_hash_eq(&mut buf, 256, 0, "C19 len256");
    // Both libraries agree (asserted above); note they are *different* values.
    assert_ne!(a, b, "sanity: len 0 vs 256 should differ on nonempty mixing");
}

// ---------------------------------------------------------------------------
// C20 — full property sweep (random len x random data x random seed)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c20_property_full_random() {
    let mut rng = Rng::new(SEED ^ 0x20);
    let mut buf = vec![0u8; 256];
    for it in 0..4096 {
        rng.fill(&mut buf);
        let len = rng.below(201) as usize;
        let seed = rng.next_u64() as usize;
        assert_hash_eq(&mut buf, len, seed, &format!("C20 it={} len={}", it, len));
    }
}

// ---------------------------------------------------------------------------
// C21 — unaligned pointers
// ---------------------------------------------------------------------------

#[test]
fn cfg_c21_unaligned_pointer() {
    let mut rng = Rng::new(SEED ^ 0x21);
    let mut backing = vec![0u8; 64];
    for off in 0..8usize {
        for len in 0..=40usize {
            for _ in 0..8 {
                rng.fill(&mut backing);
                let seed = rng.next_u64() as usize;
                let p = unsafe { backing.as_mut_ptr().add(off) } as *mut c_void;
                let ctx = format!("C21 off={} len={}", off, len);
                assert_hash_eq_raw(p, len, 0, &ctx);
                assert_hash_eq_raw(p, len, seed, &ctx);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C22 / C23 — `siphash` stdout, byte-for-byte
// ---------------------------------------------------------------------------

#[test]
fn cfg_c22_siphash_init0_stdout() {
    assert_siphash_stdout_eq(0);
}

#[test]
fn cfg_c23_siphash_init_sweep_stdout() {
    let fixed: [c_int; 14] = [
        1, 7, 42, 127, 128, 200, 255, 256, 1000, -1, -128, -1000, c_int::MIN, c_int::MAX,
    ];
    for init in fixed {
        assert_siphash_stdout_eq(init);
    }
    let mut rng = Rng::new(SEED ^ 0x23);
    for _ in 0..24 {
        assert_siphash_stdout_eq(rng.next_u64() as u32 as c_int);
    }
}

// ---------------------------------------------------------------------------
// C24 — composition: siphash's table must equal driving the low-level
//       stbds_hash_bytes on the same `mem` fill, for BOTH libraries.
// ---------------------------------------------------------------------------

#[test]
fn cfg_c24_siphash_vs_lowlevel_composition() {
    for init in [0i32, 1, 5, 200, -3, c_int::MAX, c_int::MIN] {
        // Reproduce the C fill loop: for (i=0;i<64;++i,z++) mem[i] = z;
        let mut mem = [0u8; 64];
        let mut z: c_int = init;
        for i in 0..64usize {
            mem[i] = z as u8;
            z = z.wrapping_add(1);
        }

        // Expected table, computed independently via each .so's low-level entry.
        let mut expect = String::new();
        for len in 0..64usize {
            let h = assert_hash_eq(&mut mem, len, 0, &format!("C24 init={} len={}", init, len));
            expect.push_str("  { ");
            for j in 0..8 {
                expect.push_str(&format!("0x{:02x}, ", ((h >> (j * 8)) & 255) as u8));
            }
            expect.push_str(" },\n");
        }

        let c_out = driver_stdout(&c_so(), init);
        let r_out = driver_stdout(&rust_so(), init);
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            expect,
            "C24: C siphash({}) output != low-level composition",
            init
        );
        assert_eq!(
            String::from_utf8_lossy(&r_out),
            expect,
            "C24: Rust siphash({}) output != low-level composition",
            init
        );
        assert_eq!(c_out, r_out, "C24: C vs Rust siphash({}) stdout", init);
    }
}
