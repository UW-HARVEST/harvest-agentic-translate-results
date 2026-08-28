//! Phase B — valid-path differential tests, one `#[test]` per row of
//! `CONFIGS.md`. Every test loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compares the exported `next_double` byte-for-byte
//! (returned `double` bits + full post-call `cn_rnd_t`).

mod common;

use common::*;

// ---------------------------------------------------------------------- row 1
#[test]
fn row01_zero_state_single_call() {
    for_each_pair(|c, r| {
        assert_one(c, r, CnRnd::new(0, 0), "row01 zero state, 1 call");
    });
}

// ---------------------------------------------------------------------- row 2
#[test]
fn row02_zero_state_long_run_stays_stuck() {
    for_each_pair(|c, r| {
        assert_seq(c, r, CnRnd::new(0, 0), 4096, "row02 zero state, 4096 calls");
        // and confirm the C really is a fixed point (documents the behaviour
        // the Rust must not "fix")
        let mut s = CnRnd::new(0, 0);
        for _ in 0..16 {
            let v = c.call(&mut s);
            assert_eq!(v.to_bits(), 0u64, "C zero-seed must return +0.0");
            assert_eq!(s, CnRnd::new(0, 0), "C zero-seed must stay stuck at zero");
        }
    });
}

// ------------------------------------------------------------------- rows 3-6
#[test]
fn row03_s0_zero_random_s1_single_call() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x03);
        for _ in 0..512 {
            assert_one(c, r, CnRnd::new(0, rng.next_nonzero()), "row03 {0, y}");
        }
    });
}

#[test]
fn row04_s0_zero_random_s1_long_run() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x04);
        for _ in 0..64 {
            assert_seq(
                c,
                r,
                CnRnd::new(0, rng.next_nonzero()),
                1024,
                "row04 {0, y} x1024",
            );
        }
    });
}

#[test]
fn row05_s1_zero_random_s0_single_call() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x05);
        for _ in 0..512 {
            assert_one(c, r, CnRnd::new(rng.next_nonzero(), 0), "row05 {x, 0}");
        }
    });
}

#[test]
fn row06_s1_zero_random_s0_long_run() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x06);
        for _ in 0..64 {
            assert_seq(
                c,
                r,
                CnRnd::new(rng.next_nonzero(), 0),
                1024,
                "row06 {x, 0} x1024",
            );
        }
    });
}

// ------------------------------------------------------------------- rows 7-8
#[test]
fn row07_all_ones_single_call() {
    for_each_pair(|c, r| {
        assert_one(c, r, CnRnd::new(u64::MAX, u64::MAX), "row07 {MAX, MAX}");
    });
}

#[test]
fn row08_all_ones_long_run() {
    for_each_pair(|c, r| {
        assert_seq(
            c,
            r,
            CnRnd::new(u64::MAX, u64::MAX),
            4096,
            "row08 {MAX, MAX} x4096",
        );
    });
}

// ------------------------------------------------------------------ rows 9-10
#[test]
fn row09_top_bit_only() {
    for_each_pair(|c, r| {
        assert_seq(
            c,
            r,
            CnRnd::new(1u64 << 63, 1u64 << 63),
            1024,
            "row09 {1<<63, 1<<63}",
        );
    });
}

#[test]
fn row10_minimal_nonzero() {
    for_each_pair(|c, r| {
        assert_seq(c, r, CnRnd::new(1, 1), 1024, "row10 {1, 1}");
    });
}

// ----------------------------------------------------------------- rows 11-15
#[test]
fn row11_s0_low41_bits_only() {
    // x << 23 loses nothing
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x11);
        for _ in 0..512 {
            let x = rng.next_u64() & ((1u64 << 41) - 1);
            assert_one(c, r, CnRnd::new(x, rng.next_u64()), "row11 x<2^41");
        }
    });
}

#[test]
fn row12_s0_high_bits_only() {
    // x << 23 is entirely truncated away
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x12);
        for _ in 0..512 {
            let x = rng.next_u64() & !((1u64 << 41) - 1);
            assert_one(c, r, CnRnd::new(x, rng.next_u64()), "row12 x>=2^41 bits");
        }
    });
}

#[test]
fn row13_s1_low26_bits_only() {
    // y >> 26 == 0
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x13);
        for _ in 0..512 {
            let y = rng.next_u64() & ((1u64 << 26) - 1);
            assert_one(c, r, CnRnd::new(rng.next_u64(), y), "row13 y<2^26");
        }
    });
}

#[test]
fn row14_s1_high_bits_only() {
    // y >> 26 != 0, low 26 bits clear
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x14);
        for _ in 0..512 {
            let y = rng.next_u64() & !((1u64 << 26) - 1);
            assert_one(c, r, CnRnd::new(rng.next_u64(), y), "row14 y>=2^26 bits");
        }
    });
}

#[test]
fn row15_s0_low17_bits_only() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x15);
        for _ in 0..512 {
            let x = rng.next_u64() & ((1u64 << 17) - 1);
            assert_one(c, r, CnRnd::new(x, rng.next_u64()), "row15 x<2^17");
        }
    });
}

// ----------------------------------------------------------------- rows 16-17
#[test]
fn row16_fully_random_single_call_bulk() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x16);
        for _ in 0..20_000 {
            let seed = CnRnd::new(rng.next_u64(), rng.next_u64());
            assert_one(c, r, seed, "row16 random x1");
        }
    });
}

#[test]
fn row17_fully_random_long_sequences() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x17);
        for _ in 0..200 {
            let seed = CnRnd::new(rng.next_u64(), rng.next_u64());
            assert_seq(c, r, seed, 1024, "row17 random x1024");
        }
    });
}

// ----------------------------------------------------------------- rows 18-19
#[test]
fn row18_low12_bits_of_value_all_set() {
    // the 12 bits discarded by `value >> 12` are all ones
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x18);
        for _ in 0..512 {
            let value = (rng.next_u64() & !0xFFFu64) | 0xFFF;
            let seed = seed_for_value(value, rng.next_u64());
            assert_one(c, r, seed, "row18 value low12 = 0xFFF");
        }
    });
}

#[test]
fn row19_low12_bits_of_value_all_clear() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x19);
        for _ in 0..512 {
            let value = rng.next_u64() & !0xFFFu64;
            let seed = seed_for_value(value, rng.next_u64());
            assert_one(c, r, seed, "row19 value low12 = 0");
        }
    });
}

// ----------------------------------------------------------------- rows 20-21
#[test]
fn row20_add_wraps() {
    // `return x + y;` carries out of bit 63
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x20);
        let mut tested = 0usize;
        for _ in 0..2048 {
            let y = rng.next_nonzero();
            // pick x_final in (u64::MAX - y, u64::MAX] so x_final + y wraps
            let span = y; // number of x_final values that wrap
            let x_final = u64::MAX - (rng.next_u64() % span);
            assert!(x_final.checked_add(y).is_none(), "test setup: must wrap");
            let value = x_final.wrapping_add(y);
            let seed = seed_for_value(value, y);
            // verify the construction really wraps in the model
            let mut s = seed.state;
            let _ = ref_next(&mut s);
            assert_one(c, r, seed, "row20 x+y wraps");
            tested += 1;
        }
        assert!(tested > 2000);
    });
}

#[test]
fn row21_add_does_not_wrap() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x21);
        for _ in 0..2048 {
            let y = rng.next_u64() >> 1; // < 2^63
            let x_final = rng.next_u64() >> 1; // < 2^63  => sum < 2^64
            assert!(x_final.checked_add(y).is_some(), "test setup: must not wrap");
            let seed = seed_for_value(x_final + y, y);
            assert_one(c, r, seed, "row21 x+y no wrap");
        }
    });
}

// ----------------------------------------------------------------- rows 22-23
#[test]
fn row22_mantissa_zero_returns_positive_zero() {
    for_each_pair(|c, r| {
        for value in 0u64..4096 {
            let seed = seed_for_value(value, 0);
            assert_one(c, r, seed, "row22 mantissa == 0");
            // and the value really is +0.0 (bits 0x0, *not* -0.0 = 0x8000..)
            let mut s = seed;
            let v = c.call(&mut s);
            assert_eq!(
                v.to_bits(),
                0u64,
                "C: mantissa==0 must yield +0.0 for value={value}"
            );
        }
        // also with a non-zero y
        let mut rng = SplitMix64::new(SEED ^ 0x22);
        for _ in 0..256 {
            let value = rng.next_u64() % 4096;
            let seed = seed_for_value(value, rng.next_nonzero());
            assert_one(c, r, seed, "row22 mantissa == 0, y != 0");
        }
    });
}

#[test]
fn row23_mantissa_all_ones_largest_below_one() {
    for_each_pair(|c, r| {
        let base = 0xFFFF_FFFF_FFFF_F000u64;
        for k in 0u64..4096 {
            let seed = seed_for_value(base + k, 0);
            assert_one(c, r, seed, "row23 mantissa == 0xFFFFFFFFFFFFF");
            let mut s = seed;
            let v = c.call(&mut s);
            // The largest value this construction can ever return. Note the
            // C's `(1.0 + m/2^52) - 1.0` can never reach nextafter(1.0, 0.0)
            // = 0x3FEF_FFFF_FFFF_FFFF: with m = 2^52-1 the exact difference is
            // (2^52-1)/2^52, i.e. 0x3FEF_FFFF_FFFF_FFFE, one ULP lower. That
            // quirk of the C must be reproduced, not "fixed".
            assert_eq!(v.to_bits(), 0x3FEF_FFFF_FFFF_FFFEu64);
            assert_eq!(v.to_bits(), value_to_double_bits(base + k));
            assert!(v < 1.0);
            assert!(v < f64::from_bits(0x3FEF_FFFF_FFFF_FFFF));
        }
        let mut rng = SplitMix64::new(SEED ^ 0x23);
        for _ in 0..256 {
            let seed = seed_for_value(base + (rng.next_u64() % 4096), rng.next_nonzero());
            assert_one(c, r, seed, "row23 mantissa == max, y != 0");
        }
    });
}

// ----------------------------------------------------------------- rows 24-26
#[test]
fn row24_single_bit_sweep() {
    for_each_pair(|c, r| {
        for i in 0..64 {
            for j in 0..64 {
                assert_seq(
                    c,
                    r,
                    CnRnd::new(1u64 << i, 1u64 << j),
                    4,
                    "row24 single-bit sweep",
                );
            }
        }
    });
}

#[test]
fn row25_all_but_one_bit_sweep() {
    for_each_pair(|c, r| {
        for i in 0..64 {
            for j in 0..64 {
                assert_seq(
                    c,
                    r,
                    CnRnd::new(!(1u64 << i), !(1u64 << j)),
                    4,
                    "row25 all-but-one-bit sweep",
                );
            }
        }
    });
}

#[test]
fn row26_power_of_two_plus_minus_one_sweep() {
    for_each_pair(|c, r| {
        let mut vals: Vec<u64> = Vec::new();
        for k in 0..64 {
            let p = 1u64 << k;
            vals.push(p);
            vals.push(p.wrapping_sub(1));
            vals.push(p.wrapping_add(1));
        }
        vals.push(0);
        vals.push(u64::MAX);
        vals.push(u64::MAX - 1);
        vals.sort_unstable();
        vals.dedup();
        for &a in &vals {
            for &b in &vals {
                assert_seq(c, r, CnRnd::new(a, b), 2, "row26 2^k +/- 1 sweep");
            }
        }
    });
}

// -------------------------------------------------------------------- row 27
#[test]
fn row27_two_independent_instances_interleaved() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x27);
        for _ in 0..64 {
            let a0 = CnRnd::new(rng.next_u64(), rng.next_u64());
            let b0 = CnRnd::new(rng.next_u64(), rng.next_u64());

            let (mut ca, mut cb) = (a0, b0);
            let (mut ra, mut rb) = (a0, b0);
            for step in 0..512 {
                // interleave A/B on both libraries in the same order
                let (vca, vcb) = (c.call(&mut ca), c.call(&mut cb));
                let (vra, vrb) = (r.call(&mut ra), r.call(&mut rb));
                assert_eq!(
                    vca.to_bits(),
                    vra.to_bits(),
                    "row27 instance A diverged at step {step}"
                );
                assert_eq!(
                    vcb.to_bits(),
                    vrb.to_bits(),
                    "row27 instance B diverged at step {step}"
                );
                assert_eq!(ca, ra, "row27 state A diverged at step {step}");
                assert_eq!(cb, rb, "row27 state B diverged at step {step}");
                // A and B are independent generators: identical seeds would be
                // the only reason for them to agree.
                if a0 != b0 {
                    assert_ne!(
                        (ca, cb),
                        (cb, ca),
                        "row27 sanity: instances must not be aliased"
                    );
                }
            }
        }
    });
}

// -------------------------------------------------------------------- row 28
#[test]
fn row28_stack_heap_and_offset_placement() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x28);
        for _ in 0..256 {
            let seed = CnRnd::new(rng.next_u64(), rng.next_u64());

            // (a) stack
            let mut stack_c = seed;
            let mut stack_r = seed;
            let vsc = c.call(&mut stack_c);
            let vsr = r.call(&mut stack_r);

            // (b) heap
            let mut heap_c = Box::new(seed);
            let mut heap_r = Box::new(seed);
            let vhc = unsafe { c.call_ptr(&mut *heap_c as *mut CnRnd) };
            let vhr = unsafe { r.call_ptr(&mut *heap_r as *mut CnRnd) };

            // (c) inside a larger, 8-byte-aligned buffer at a non-zero offset
            let mut buf_c = vec![0u64; 8];
            let mut buf_r = vec![0u64; 8];
            buf_c[2] = seed.state[0];
            buf_c[3] = seed.state[1];
            buf_r[2] = seed.state[0];
            buf_r[3] = seed.state[1];
            let voc = unsafe { c.call_ptr(buf_c.as_mut_ptr().add(2) as *mut CnRnd) };
            let vor = unsafe { r.call_ptr(buf_r.as_mut_ptr().add(2) as *mut CnRnd) };

            for (label, cv, rv) in [
                ("stack", vsc, vsr),
                ("heap", vhc, vhr),
                ("offset", voc, vor),
            ] {
                assert_eq!(
                    cv.to_bits(),
                    rv.to_bits(),
                    "row28 {label} placement diverged for seed {:#x?}",
                    seed.state
                );
            }
            // all three placements must give the same answer within each library
            assert_eq!(vsc.to_bits(), vhc.to_bits(), "row28 C stack vs heap");
            assert_eq!(vsc.to_bits(), voc.to_bits(), "row28 C stack vs offset");
            assert_eq!(vsr.to_bits(), vhr.to_bits(), "row28 R stack vs heap");
            assert_eq!(vsr.to_bits(), vor.to_bits(), "row28 R stack vs offset");

            assert_eq!(stack_c, stack_r);
            assert_eq!(*heap_c, *heap_r);
            assert_eq!(buf_c, buf_r);
            assert_eq!([buf_c[2], buf_c[3]], stack_c.state);
        }
    });
}

// -------------------------------------------------------------------- row 29
#[test]
fn row29_only_the_sixteen_bytes_are_touched() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x29);
        const N: usize = 6; // u64 slots; struct lives in slots 2..4
        for _ in 0..512 {
            let seed = CnRnd::new(rng.next_u64(), rng.next_u64());
            let canary: u64 = rng.next_u64();

            let run = |lib: &Lib| -> (f64, [u64; N]) {
                let mut buf = [canary; N];
                buf[2] = seed.state[0];
                buf[3] = seed.state[1];
                let v = unsafe { lib.call_ptr(buf.as_mut_ptr().add(2) as *mut CnRnd) };
                (v, buf)
            };
            let (vc, bc) = run(c);
            let (vr, br) = run(r);

            assert_eq!(vc.to_bits(), vr.to_bits(), "row29 return value");
            assert_eq!(bc, br, "row29 whole buffer must be identical");
            for (lbl, b) in [("C", bc), ("rust", br)] {
                assert_eq!(b[0], canary, "row29 {lbl} clobbered red zone below [0]");
                assert_eq!(b[1], canary, "row29 {lbl} clobbered red zone below [1]");
                assert_eq!(b[4], canary, "row29 {lbl} clobbered red zone above [4]");
                assert_eq!(b[5], canary, "row29 {lbl} clobbered red zone above [5]");
            }
        }
    });
}

// -------------------------------------------------------------------- row 30
#[test]
fn row30_cross_fed_lockstep_state() {
    // Feed the state produced by one library into the other and back, so any
    // drift in the low-level `cn_rnd_next` transition shows up immediately.
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0x30);
        for _ in 0..64 {
            let mut s = CnRnd::new(rng.next_u64(), rng.next_u64());
            for step in 0..1024 {
                let before = s;

                let mut sc = before;
                let vc = c.call(&mut sc);
                let mut sr = before;
                let vr = r.call(&mut sr);

                assert_eq!(
                    vc.to_bits(),
                    vr.to_bits(),
                    "row30 value drift at step {step}, state {:#x?}",
                    before.state
                );
                assert_eq!(
                    sc.bytes(),
                    sr.bytes(),
                    "row30 state drift at step {step}, state {:#x?}",
                    before.state
                );
                // alternate which library's output continues the chain
                s = if step % 2 == 0 { sc } else { sr };
            }
        }
    });
}

// -------------------------------------------------------------------- row 31
#[test]
fn row31_overflow_checks_profile_must_not_panic() {
    // Explicitly exercise the dev-profile (overflow-checks = on) cdylib with
    // inputs that overflow `x + y` and shift the top bits out.
    let c = c_lib();
    let r = rust_lib("dev");
    let mut rng = SplitMix64::new(SEED ^ 0x31);
    let mut wrapping_seen = 0usize;
    for _ in 0..4096 {
        let y = rng.next_nonzero();
        let x_final = u64::MAX - (rng.next_u64() % y);
        let seed = seed_for_value(x_final.wrapping_add(y), y);
        assert_one(&c, &r, seed, "row31 overflow-checks build, wrapping add");
        wrapping_seen += 1;
    }
    assert!(wrapping_seen >= 4096);
    // saturated-bit inputs on the dev build
    assert_seq(
        &c,
        &r,
        CnRnd::new(u64::MAX, u64::MAX),
        4096,
        "row31 MAX/MAX dev build",
    );
    for i in 0..64 {
        assert_seq(
            &c,
            &r,
            CnRnd::new(u64::MAX << i, u64::MAX >> i),
            64,
            "row31 shifted saturation dev build",
        );
    }
}

// -------------------------------------------------------------------- row 32
#[test]
fn row32_release_profile_matches_too() {
    // Mirror of row 31 against the release (opt-level = 3, overflow-checks =
    // off, panic = abort) cdylib, so every profile is covered by name.
    let c = c_lib();
    let r = rust_lib("release");
    let mut rng = SplitMix64::new(SEED ^ 0x32);
    for _ in 0..4096 {
        let seed = CnRnd::new(rng.next_u64(), rng.next_u64());
        assert_seq(&c, &r, seed, 4, "row32 release build random");
    }
    assert_seq(
        &c,
        &r,
        CnRnd::new(u64::MAX, u64::MAX),
        4096,
        "row32 MAX/MAX release build",
    );
}

// -------------------------------------------------------------------- row 33
#[test]
fn row33_ubcheck_profile_matches_on_all_valid_inputs() {
    // `ubcheck` turns Rust's optional UB checks back on. On every *valid*
    // input (including misaligned structs, which the C accepts) it must still
    // agree with the C bit-for-bit and never abort.
    let c = c_lib();
    let r = rust_lib("ubcheck");
    let mut rng = SplitMix64::new(SEED ^ 0x33);
    for _ in 0..4096 {
        let seed = CnRnd::new(rng.next_u64(), rng.next_u64());
        assert_seq(&c, &r, seed, 4, "row33 ubcheck build random");
    }
    assert_seq(
        &c,
        &r,
        CnRnd::new(u64::MAX, u64::MAX),
        4096,
        "row33 MAX/MAX ubcheck build",
    );
    assert_seq(&c, &r, CnRnd::new(0, 0), 256, "row33 zero seed ubcheck build");
    // forced wrapping add under overflow-checks = on
    for _ in 0..2048 {
        let y = rng.next_nonzero();
        let x_final = u64::MAX - (rng.next_u64() % y);
        let seed = seed_for_value(x_final.wrapping_add(y), y);
        assert_one(&c, &r, seed, "row33 wrapping add, ubcheck build");
    }
    // misaligned placement, which the C accepts
    for offset in 1usize..8 {
        for _ in 0..64 {
            let seed = CnRnd::new(rng.next_u64(), rng.next_u64());
            let run = |lib: &Lib| -> (u64, [u8; 16]) {
                let mut buf = [0u8; 32];
                buf[offset..offset + 8].copy_from_slice(&seed.state[0].to_ne_bytes());
                buf[offset + 8..offset + 16].copy_from_slice(&seed.state[1].to_ne_bytes());
                let v = unsafe { lib.call_ptr(buf.as_mut_ptr().add(offset) as *mut CnRnd) };
                let mut after = [0u8; 16];
                after.copy_from_slice(&buf[offset..offset + 16]);
                (v.to_bits(), after)
            };
            assert_eq!(run(&c), run(&r), "row33 misaligned, ubcheck build");
        }
    }
}
