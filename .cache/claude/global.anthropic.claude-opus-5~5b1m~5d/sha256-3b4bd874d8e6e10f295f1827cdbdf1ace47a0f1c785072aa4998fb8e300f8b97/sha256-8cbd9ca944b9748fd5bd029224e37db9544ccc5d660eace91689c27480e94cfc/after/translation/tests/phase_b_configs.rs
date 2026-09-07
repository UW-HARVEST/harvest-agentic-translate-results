//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Both implementations are loaded as `.so`s and compared byte-for-byte on the
//! raw bits of the returned `double` AND on the mutated `cn_rnd_t` state.

mod common;

use common::*;

// --- meta: the seed-construction helper must actually work -----------------

#[test]
fn meta_seed_for_value_is_exact() {
    let mut r = rng();
    for _ in 0..5_000 {
        let want = r.next_u64();
        let y = r.next_u64();
        let seed = seed_for_value(want, y);
        let (got, _) = model_step(seed);
        assert_eq!(got, want, "seed_for_value inverse is wrong (y = {y:#018x})");
    }
    // and the extremes used by rows 18/19
    for want in [0u64, 4095, 4096, u64::MAX, u64::MAX - 1] {
        let seed = seed_for_value(want, 0xDEAD_BEEF_CAFE_F00D);
        assert_eq!(model_step(seed).0, want);
    }
}

// --- row 1 / 2 : degenerate all-zero fixed point ---------------------------

#[test]
fn cfg_01_zero_state_single_call() {
    let p = pair();
    p.assert_same(CnRnd::new(0, 0), 1, "row1 zero state, 1 call");

    // and pin down the documented absolute value
    let mut s = CnRnd::new(0, 0);
    let bits = p.c.step(&mut s);
    assert_eq!(bits, 0x0000_0000_0000_0000, "C: zero state must yield +0.0");
    assert_eq!(s, CnRnd::new(0, 0), "C: zero state must be a fixed point");
}

#[test]
fn cfg_02_zero_state_1000_calls_stays_pinned() {
    let p = pair();
    p.assert_same(CnRnd::new(0, 0), 1000, "row2 zero state, 1000 calls");
    let (out, fin) = p.rs.run(CnRnd::new(0, 0), 1000);
    assert!(out.iter().all(|&b| b == 0), "zero state must stay at +0.0");
    assert_eq!(fin, CnRnd::new(0, 0));
}

// --- row 3 / 4 : all-ones state -------------------------------------------

#[test]
fn cfg_03_all_ones_single_call() {
    pair().assert_same(CnRnd::new(u64::MAX, u64::MAX), 1, "row3 all-ones, 1 call");
}

#[test]
fn cfg_04_all_ones_1000_calls() {
    pair().assert_same(CnRnd::new(u64::MAX, u64::MAX), 1000, "row4 all-ones, 1000 calls");
}

// --- row 5 : minimal non-zero seeds ---------------------------------------

#[test]
fn cfg_05_minimal_nonzero_seeds() {
    let p = pair();
    p.assert_same(CnRnd::new(1, 0), 1, "row5 {1,0}");
    p.assert_same(CnRnd::new(0, 1), 1, "row5 {0,1}");
    p.assert_same(CnRnd::new(1, 1), 1, "row5 {1,1}");
    // longer runs from the same minimal seeds
    p.assert_same(CnRnd::new(1, 0), 64, "row5 {1,0} x64");
    p.assert_same(CnRnd::new(0, 1), 64, "row5 {0,1} x64");
}

// --- rows 6 / 7 / 8 : single-bit walks ------------------------------------

#[test]
fn cfg_06_single_bit_walk_x() {
    let p = pair();
    for i in 0..64 {
        p.assert_same(CnRnd::new(1u64 << i, 0), 4, &format!("row6 x bit {i}"));
    }
}

#[test]
fn cfg_07_single_bit_walk_y() {
    let p = pair();
    for i in 0..64 {
        p.assert_same(CnRnd::new(0, 1u64 << i), 4, &format!("row7 y bit {i}"));
    }
}

#[test]
fn cfg_08_single_bit_walk_both_4096_pairs() {
    let p = pair();
    for i in 0..64 {
        for j in 0..64 {
            p.assert_same(
                CnRnd::new(1u64 << i, 1u64 << j),
                1,
                &format!("row8 x bit {i}, y bit {j}"),
            );
        }
    }
}

// --- row 9 : randomized, single call --------------------------------------

#[test]
fn cfg_09_random_single_call_20000() {
    let p = pair();
    let mut r = rng();
    for k in 0..20_000 {
        p.assert_same(r.seed_pair(), 1, &format!("row9 iter {k}"));
    }
}

// --- row 10 : randomized, two calls (state rotation visible) --------------

#[test]
fn cfg_10_random_two_calls_5000() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 10);
    for k in 0..5_000 {
        p.assert_same(r.seed_pair(), 2, &format!("row10 iter {k}"));
    }
}

// --- row 11 : randomized long streams ------------------------------------

#[test]
fn cfg_11_random_1000_step_streams_200_seeds() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 11);
    for k in 0..200 {
        let seed = r.seed_pair();
        let (co, cf) = p.c.run(seed, 1000);
        let (ro, rf) = p.rs.run(seed, 1000);
        assert_eq!(co, ro, "row11 seed {k} ({seed:?}): output stream differs");
        assert_eq!(cf, rf, "row11 seed {k} ({seed:?}): final state differs");
    }
}

// --- rows 12..15 : shift-sensitive shapes --------------------------------

fn shaped_rows(tag: &str, salt: u64, n: usize, shape: impl Fn(u64, u64) -> CnRnd) {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ salt);
    for k in 0..n {
        let a = r.next_u64();
        let b = r.next_u64();
        p.assert_same(shape(a, b), 3, &format!("{tag} iter {k}"));
    }
}

#[test]
fn cfg_12_x_low_17_bits_only() {
    // x >> 17 == 0
    shaped_rows("row12 x<2^17", 12, 5_000, |a, b| {
        CnRnd::new(a & 0x1_FFFF, b)
    });
}

#[test]
fn cfg_13_x_high_23_bits_only() {
    // x << 23 == 0  (only the top 23 bits populated)
    shaped_rows("row13 x high23", 13, 5_000, |a, b| {
        CnRnd::new(a & 0xFFFF_FE00_0000_0000, b)
    });
}

#[test]
fn cfg_14_y_low_26_bits_only() {
    // y >> 26 == 0
    shaped_rows("row14 y<2^26", 14, 5_000, |a, b| {
        CnRnd::new(a, b & 0x3FF_FFFF)
    });
}

#[test]
fn cfg_15_y_high_38_bits_only() {
    // y >> 26 fully populated, low 26 bits clear
    shaped_rows("row15 y high38", 15, 5_000, |a, b| {
        CnRnd::new(a, b & !0x3FF_FFFFu64)
    });
}

// --- rows 16 / 17 : x + y wraparound classification ----------------------

#[test]
fn cfg_16_sum_wraps_modulo_2_64() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 16);
    let mut hits = 0usize;
    let mut tried = 0usize;
    while hits < 2_000 {
        tried += 1;
        assert!(tried < 10_000_000, "could not find wrapping seeds");
        let seed = r.seed_pair();
        // classify with the test-local model (C .so remains the oracle)
        let x_final = model_step(seed).1.state[1];
        let y = seed.state[1];
        if x_final.checked_add(y).is_none() {
            p.assert_same(seed, 1, &format!("row16 wrap hit {hits}"));
            hits += 1;
        }
    }
    assert_eq!(hits, 2_000);
}

#[test]
fn cfg_17_sum_does_not_wrap() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 17);
    let mut hits = 0usize;
    let mut tried = 0usize;
    while hits < 2_000 {
        tried += 1;
        assert!(tried < 10_000_000, "could not find non-wrapping seeds");
        let seed = r.seed_pair();
        let x_final = model_step(seed).1.state[1];
        let y = seed.state[1];
        if x_final.checked_add(y).is_some() {
            p.assert_same(seed, 1, &format!("row17 nowrap hit {hits}"));
            hits += 1;
        }
    }
    assert_eq!(hits, 2_000);
}

// --- row 18 : mantissa == 0 -> exactly +0.0 ------------------------------

#[test]
fn cfg_18_mantissa_zero_is_exactly_plus_zero() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 18);
    for k in 0..2_000 {
        let y = r.next_u64();
        // any value < 4096 gives mantissa == 0
        let v = r.next_u64() % 4096;
        let seed = seed_for_value(v, y);
        assert_eq!(model_step(seed).0 >> 12, 0);

        let mut cs = seed;
        let mut rs = seed;
        let cv = p.c.step(&mut cs);
        let rv = p.rs.step(&mut rs);
        assert_eq!(cv, rv, "row18 iter {k}: bits differ");
        assert_eq!(cs, rs, "row18 iter {k}: state differs");
        assert_eq!(cv, 0x0000_0000_0000_0000, "row18 iter {k}: C must return +0.0");
    }
}

// --- row 19 : mantissa all ones -> 1.0 - 2^-53 ---------------------------

#[test]
fn cfg_19_mantissa_all_ones_is_one_minus_ulp() {
    let p = pair();
    // (1023<<52)|0xF_FFFF_FFFF_FFFF == 0x3FFFFFFFFFFFFFFF == 2 - 2^-52.
    // Minus 1.0 that is exactly 1 - 2^-52 == 0x3FEFFFFFFFFFFFFE.
    let expect = (1.0f64 - 2f64.powi(-52)).to_bits();
    let mut r = SplitMix64::new(FIXED_SEED ^ 19);
    for k in 0..2_000 {
        let y = r.next_u64();
        // value with all 52 high bits set: 0xFFFFFFFFFFFFF000 | anything low
        let v = 0xFFFF_FFFF_FFFF_F000u64 | (r.next_u64() & 0xFFF);
        let seed = seed_for_value(v, y);
        assert_eq!(model_step(seed).0 >> 12, 0x000F_FFFF_FFFF_FFFF);

        let mut cs = seed;
        let mut rs = seed;
        let cv = p.c.step(&mut cs);
        let rv = p.rs.step(&mut rs);
        assert_eq!(cv, rv, "row19 iter {k}: bits differ");
        assert_eq!(cs, rs, "row19 iter {k}: state differs");
        assert_eq!(cv, expect, "row19 iter {k}: C must return 1.0 - 2^-52");
        assert_eq!(cv, 0x3FEF_FFFF_FFFF_FFFE, "row19 iter {k}: raw bit pattern");
        assert!(f64::from_bits(cv) < 1.0);
    }
}

// --- row 20 : low 12 bits of `value` do not change the double ------------

#[test]
fn cfg_20_low_12_bits_change_state_not_value() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 20);
    for k in 0..2_000 {
        let y = r.next_u64();
        let base = r.next_u64() & !0xFFFu64;
        let alt = base | ((r.next_u64() & 0xFFF) | 1);
        let s1 = seed_for_value(base, y);
        let s2 = seed_for_value(alt, y);

        let (mut c1, mut c2) = (s1, s2);
        let (mut r1, mut r2) = (s1, s2);
        let (cv1, cv2) = (p.c.step(&mut c1), p.c.step(&mut c2));
        let (rv1, rv2) = (p.rs.step(&mut r1), p.rs.step(&mut r2));

        assert_eq!(cv1, rv1, "row20 iter {k}: bits differ (base)");
        assert_eq!(cv2, rv2, "row20 iter {k}: bits differ (alt)");
        assert_eq!(c1, r1, "row20 iter {k}: state differs (base)");
        assert_eq!(c2, r2, "row20 iter {k}: state differs (alt)");
        // The C's own invariant: identical mantissa => identical double,
        // yet the states diverge.
        assert_eq!(cv1, cv2, "row20 iter {k}: low 12 bits must not affect the double");
        assert_ne!(c1, c2, "row20 iter {k}: states must differ");
    }
}

// --- row 21 : output range invariant ------------------------------------

#[test]
fn cfg_21_output_range_invariant() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 21);
    for k in 0..20_000 {
        let seed = r.seed_pair();
        let mut cs = seed;
        let mut rs = seed;
        let cv = p.c.step_f(&mut cs);
        let rv = p.rs.step_f(&mut rs);
        assert_eq!(cv.to_bits(), rv.to_bits(), "row21 iter {k}: bits differ");
        assert_eq!(cs, rs, "row21 iter {k}: state differs");
        for (n, v) in [("C", cv), ("Rust", rv)] {
            assert!(v.is_finite(), "row21 iter {k}: {n} produced {v}");
            assert!(!v.is_nan(), "row21 iter {k}: {n} produced NaN");
            assert!((0.0..1.0).contains(&v), "row21 iter {k}: {n} produced {v} outside [0,1)");
        }
    }
}

// --- row 22 : several independent instances interleaved ------------------

#[test]
fn cfg_22_eight_interleaved_instances() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 22);
    let seeds: Vec<CnRnd> = (0..8).map(|_| r.seed_pair()).collect();
    let mut cst = seeds.clone();
    let mut rst = seeds.clone();
    for round in 0..500 {
        for i in 0..8 {
            let cv = p.c.step(&mut cst[i]);
            let rv = p.rs.step(&mut rst[i]);
            assert_eq!(cv, rv, "row22 round {round} inst {i}: bits differ");
            assert_eq!(cst[i], rst[i], "row22 round {round} inst {i}: state differs");
        }
    }
    // No hidden global state: the 8 streams must remain mutually distinct.
    for i in 0..8 {
        for j in (i + 1)..8 {
            assert_ne!(cst[i], cst[j], "row22: instances {i}/{j} collided");
        }
    }
}

// --- row 23 : element of an array, neighbours untouched -----------------

#[test]
fn cfg_23_array_element_no_out_of_bounds_write() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 23);
    for k in 0..2_000 {
        let filler = CnRnd::new(0xA5A5_A5A5_A5A5_A5A5, 0x5A5A_5A5A_5A5A_5A5A);
        let seed = r.seed_pair();
        let idx = (r.next_u64() % 4) as usize;

        let mut carr = [filler; 4];
        let mut rarr = [filler; 4];
        carr[idx] = seed;
        rarr[idx] = seed;

        let cv = p.c.step(&mut carr[idx]);
        let rv = p.rs.step(&mut rarr[idx]);
        assert_eq!(cv, rv, "row23 iter {k}: bits differ");
        assert_eq!(carr, rarr, "row23 iter {k}: array contents differ");
        for n in 0..4 {
            if n != idx {
                assert_eq!(carr[n], filler, "row23 iter {k}: C wrote out of bounds at {n}");
                assert_eq!(rarr[n], filler, "row23 iter {k}: Rust wrote out of bounds at {n}");
            }
        }
    }
}

// --- row 24 : heap vs stack argument -----------------------------------

#[test]
fn cfg_24_heap_and_stack_arguments_agree() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 24);
    for k in 0..2_000 {
        let seed = r.seed_pair();

        let mut c_stack = seed;
        let mut r_stack = seed;
        let mut c_heap = Box::new(seed);
        let mut r_heap = Box::new(seed);

        let a = p.c.step(&mut c_stack);
        let b = unsafe { p.c.step_raw(&mut *c_heap as *mut CnRnd) };
        let c = p.rs.step(&mut r_stack);
        let d = unsafe { p.rs.step_raw(&mut *r_heap as *mut CnRnd) };

        assert_eq!(a, b, "row24 iter {k}: C stack vs heap differ");
        assert_eq!(c, d, "row24 iter {k}: Rust stack vs heap differ");
        assert_eq!(a, c, "row24 iter {k}: C vs Rust differ");
        assert_eq!(c_stack, r_stack, "row24 iter {k}: stack state differs");
        assert_eq!(*c_heap, *r_heap, "row24 iter {k}: heap state differs");
    }
}

// --- row 25 : misaligned pointer ---------------------------------------

#[test]
fn cfg_25_misaligned_pointer() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 25);
    for k in 0..1_000 {
        let seed = r.seed_pair();
        let off = 1 + (r.next_u64() % 7) as usize; // odd/unaligned offsets 1..7

        let mut cbuf = [0u8; 32];
        let mut rbuf = [0u8; 32];
        cbuf[off..off + 8].copy_from_slice(&seed.state[0].to_ne_bytes());
        cbuf[off + 8..off + 16].copy_from_slice(&seed.state[1].to_ne_bytes());
        rbuf.copy_from_slice(&cbuf);

        let cv = unsafe { p.c.step_raw(cbuf.as_mut_ptr().add(off) as *mut CnRnd) };
        let rv = unsafe { p.rs.step_raw(rbuf.as_mut_ptr().add(off) as *mut CnRnd) };

        assert_eq!(cv, rv, "row25 iter {k} off {off}: bits differ");
        assert_eq!(cbuf, rbuf, "row25 iter {k} off {off}: buffer bytes differ");
    }
}

// --- row 26 : state[0] <- old state[1] contract ------------------------

#[test]
fn cfg_26_state_rotation_contract() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 26);
    for k in 0..5_000 {
        let seed = r.seed_pair();
        let mut cs = seed;
        let mut rs = seed;
        p.c.step(&mut cs);
        p.rs.step(&mut rs);
        assert_eq!(cs, rs, "row26 iter {k}: state differs");
        assert_eq!(cs.state[0], seed.state[1], "row26 iter {k}: C rotation broken");
        assert_eq!(rs.state[0], seed.state[1], "row26 iter {k}: Rust rotation broken");
    }
}

// --- row 27 : interoperable state (C step then Rust step, alternating) --

#[test]
fn cfg_27_alternating_c_and_rust_on_shared_state() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 27);
    for s in 0..50 {
        let seed = r.seed_pair();
        // reference: all-C stream
        let (want, want_fin) = p.c.run(seed, 400);

        // alternating stream on a single shared struct
        let mut shared = seed;
        let mut got = Vec::with_capacity(400);
        for i in 0..400 {
            let v = if i % 2 == 0 {
                p.c.step(&mut shared)
            } else {
                p.rs.step(&mut shared)
            };
            got.push(v);
        }
        assert_eq!(want, got, "row27 seed {s}: alternating C/Rust stream diverged");
        assert_eq!(want_fin, shared, "row27 seed {s}: final shared state diverged");

        // and the mirror order (Rust first)
        let mut shared2 = seed;
        let mut got2 = Vec::with_capacity(400);
        for i in 0..400 {
            let v = if i % 2 == 0 {
                p.rs.step(&mut shared2)
            } else {
                p.c.step(&mut shared2)
            };
            got2.push(v);
        }
        assert_eq!(want, got2, "row27 seed {s}: Rust-first stream diverged");
        assert_eq!(want_fin, shared2, "row27 seed {s}: final shared state diverged");
    }
}

// --- row 28 : very long single run (drift detection) -------------------

#[test]
fn cfg_28_one_hundred_thousand_step_run() {
    let p = pair();
    let seed = CnRnd::new(0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210);
    let mut cs = seed;
    let mut rs = seed;
    for i in 0..100_000u32 {
        let cv = p.c.step(&mut cs);
        let rv = p.rs.step(&mut rs);
        if cv != rv || cs != rs {
            panic!(
                "row28: drift at step {i}: C bits {cv:#018x} state {:?} vs Rust bits {rv:#018x} state {:?}",
                cs, rs
            );
        }
    }
}

// --- row 29 : no driver binary exists ---------------------------------

#[test]
fn cfg_29_no_driver_binary_in_either_project() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let cmake = std::fs::read_to_string(root.join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "CMakeLists.txt now builds an executable; a stdout differential test is required"
    );

    let toml = std::fs::read_to_string(root.join("translation/Cargo.toml")).unwrap();
    assert!(!toml.contains("[[bin]]"), "crate now declares a [[bin]] target");
    assert!(
        !root.join("translation/src/main.rs").exists(),
        "crate now has src/main.rs"
    );
}

// --- guard : the loaded .so must not be stale -------------------------
//
// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact, so a
// stale `libnext_double_lib.so` can silently be validated instead of the
// current source. Run `cargo build` (same profile) before `cargo test`.

#[test]
fn guard_loaded_shared_objects_are_not_stale() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mtime = |p: &std::path::Path| {
        std::fs::metadata(p)
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
            .modified()
            .unwrap()
    };

    let rs_src = mtime(&root.join("translation/src/lib.rs"));
    let rs_so = rust_so_path();
    assert!(
        mtime(&rs_so) >= rs_src,
        "STALE Rust .so: {} is older than src/lib.rs. \
         `cargo test` does not rebuild a cdylib -- run `cargo build` \
         (profile `{}`) first.",
        rs_so.display(),
        profile_dir()
    );

    let c_src = mtime(&root.join("c_src/src/lib.c"));
    let c_so = c_so_path();
    assert!(
        mtime(&c_so) >= c_src,
        "STALE C .so: {} is older than c_src/src/lib.c; re-run cmake --build",
        c_so.display()
    );
}

// --- row 30 : the loaded objects really are two distinct files --------

#[test]
fn cfg_30_two_distinct_shared_objects_are_loaded() {
    let c = c_so_path();
    let r = rust_so_path();
    assert!(c.exists(), "C .so missing: {}", c.display());
    assert!(r.exists(), "Rust .so missing: {}", r.display());
    assert_ne!(c, r, "the same file was loaded twice");
    eprintln!("C   : {}", c.display());
    eprintln!("Rust: {}", r.display());
    // sanity: the Rust object must be the cdylib built from this crate
    assert!(r.file_name().unwrap().to_string_lossy().contains("next_double_lib"));
}
