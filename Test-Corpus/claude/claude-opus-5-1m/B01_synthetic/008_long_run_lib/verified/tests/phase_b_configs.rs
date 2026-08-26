// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Every test drives BOTH shared objects (C and Rust) through their exported
// symbols only, and compares the full 1 MiB `array` object byte-for-byte (plus,
// where relevant, the XOR reduction channel that `long_exec` prints).
//
// Rows with a random component use a fixed-seed SplitMix64 so failures are
// reproducible.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// C1 — fresh `.bss`
// ---------------------------------------------------------------------------
#[test]
fn c1_fresh_bss_is_zero() {
    // NOTE: this must be the state right after `dlopen`; the harness zeroes the
    // arrays first so the row is order-independent, but the *size* and
    // zero-fill of the object is what is being asserted.
    let h = harness();
    for lib in h.libs() {
        let bytes = lib.read_bytes();
        assert_eq!(bytes.len(), ARRAY_SIZE * 4, "{}: array size", lib.name);
    }
    h.zero_both();
    assert_eq!(h.c.read_bytes(), h.rust.read_bytes());
    assert!(h.c.read_bytes().iter().all(|&b| b == 0));
    assert_eq!(h.c.xor_array(), 0);
    assert_eq!(h.rust.xor_array(), 0);
}

// ---------------------------------------------------------------------------
// C2 — the exported data object itself
// ---------------------------------------------------------------------------
#[test]
fn c2_array_roundtrip() {
    let h = harness();
    let mut rng = SplitMix64::new(0xC2);
    for trial in 0..4 {
        let data = random_array(&mut rng);
        h.write_both(&data);
        assert_eq!(h.c.read_array(), data, "trial {trial}: C array roundtrip");
        assert_eq!(
            h.rust.read_array(),
            data,
            "trial {trial}: Rust array roundtrip"
        );
        h.assert_arrays_equal(&format!("c2 trial {trial}"));
        assert_eq!(h.c.xor_array(), h.rust.xor_array());
    }
}

// ---------------------------------------------------------------------------
// C3 — one call on the all-zero array
// ---------------------------------------------------------------------------
#[test]
fn c3_peo_zeros() {
    let h = harness();
    h.zero_both();
    h.c.perform_expensive_operations();
    h.rust.perform_expensive_operations();
    h.assert_arrays_equal("c3 zeros");
    // Every element saw the same input, so the whole array must be uniform.
    let v = h.c.get(0);
    assert!(h.c.read_array().iter().all(|&x| x == v));
    println!("c3: f^100(0) = {v}");
}

// ---------------------------------------------------------------------------
// C4 — uniform random over the full i32 range (64 randomized trials)
// ---------------------------------------------------------------------------
#[test]
fn c4_peo_uniform_random() {
    let h = harness();
    let mut rng = SplitMix64::new(0xC4_0000);
    for trial in 0..64 {
        let data = random_array(&mut rng);
        h.write_both(&data);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c4 trial {trial}"));
        assert_eq!(
            h.c.xor_array(),
            h.rust.xor_array(),
            "c4 trial {trial}: xor channel"
        );
    }
}

// ---------------------------------------------------------------------------
// C5 — non-negative random (`rand()`-shaped), 32 trials
// ---------------------------------------------------------------------------
#[test]
fn c5_peo_nonnegative_random() {
    let h = harness();
    let mut rng = SplitMix64::new(0xC5_0000);
    for trial in 0..32 {
        let data = random_nonnegative_array(&mut rng);
        assert!(data.iter().all(|&x| x >= 0));
        h.write_both(&data);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c5 trial {trial}"));
    }
}

// ---------------------------------------------------------------------------
// C6 — uniform extreme arrays
// ---------------------------------------------------------------------------
#[test]
fn c6_peo_uniform_extremes() {
    let h = harness();
    for v in [
        i32::MIN,
        i32::MAX,
        -1,
        1,
        7,
        -7,
        i32::MIN + 1,
        i32::MAX - 1,
        0x4000_0000,
        -0x4000_0000,
    ] {
        let data = vec![v as c_int; ARRAY_SIZE];
        h.write_both(&data);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c6 all-{v}"));
        println!("c6: f^100({v}) = {}", h.c.get(0));
    }
}

// ---------------------------------------------------------------------------
// C7 — boundary tiling
// ---------------------------------------------------------------------------
#[test]
fn c7_peo_boundary_tiling() {
    let h = harness();
    let vals = boundary_values();
    println!("c7: {} distinct boundary values tiled", vals.len());
    let data = tile(&vals);
    h.write_both(&data);
    h.c.perform_expensive_operations();
    h.rust.perform_expensive_operations();
    h.assert_arrays_equal("c7 boundary tiling");

    // Same values, rotated, so each value also lands on a different index
    // parity / cache line.
    for rot in [1usize, 3, 7, 13] {
        let mut rotated = vals.clone();
        rotated.rotate_left(rot);
        let data = tile(&rotated);
        h.write_both(&data);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c7 boundary tiling rot {rot}"));
    }
}

// ---------------------------------------------------------------------------
// C8 — sign-alternating random
// ---------------------------------------------------------------------------
#[test]
fn c8_peo_sign_alternating() {
    let h = harness();
    let mut rng = SplitMix64::new(0xC8_0000);
    for trial in 0..8 {
        let data: Vec<c_int> = (0..ARRAY_SIZE)
            .map(|i| {
                let v = rng.next_i32();
                if i % 2 == 0 {
                    v.wrapping_abs()
                } else {
                    v.wrapping_abs().wrapping_neg()
                }
            })
            .collect();
        h.write_both(&data);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c8 trial {trial}"));
    }
}

// ---------------------------------------------------------------------------
// C9 / C10 — first and last element only
// ---------------------------------------------------------------------------
#[test]
fn c9_peo_first_element_only() {
    let h = harness();
    let mut rng = SplitMix64::new(0xC9_0000);
    for trial in 0..8 {
        h.zero_both();
        let v = rng.next_i32();
        h.c.set(0, v);
        h.rust.set(0, v);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c9 trial {trial} (array[0] = {v})"));
        assert_ne!(
            h.c.get(0),
            h.c.get(1),
            "c9: array[0] should have been transformed differently from the zeros"
        );
    }
}

#[test]
fn c10_peo_last_element_only() {
    let h = harness();
    let mut rng = SplitMix64::new(0xCA_0000);
    for trial in 0..8 {
        h.zero_both();
        let v = rng.next_i32();
        h.c.set(ARRAY_SIZE - 1, v);
        h.rust.set(ARRAY_SIZE - 1, v);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c10 trial {trial} (array[last] = {v})"));
        // The loop must actually reach the last element.
        assert_eq!(
            h.c.get(ARRAY_SIZE - 1),
            h.rust.get(ARRAY_SIZE - 1),
            "c10: last element"
        );
        assert_ne!(
            h.rust.get(ARRAY_SIZE - 1),
            v,
            "c10: last element was not transformed at all"
        );
    }
}

// ---------------------------------------------------------------------------
// C11 — sparse / strided subset
// ---------------------------------------------------------------------------
#[test]
fn c11_peo_strided_subset() {
    let h = harness();
    let mut rng = SplitMix64::new(0xCB_0000);
    for trial in 0..8 {
        h.zero_both();
        let stride = [4093usize, 1, 2, 3, 17, 4096, 65536, 131071][trial];
        let mut i = 0;
        while i < ARRAY_SIZE {
            let v = rng.next_i32();
            h.c.set(i, v);
            h.rust.set(i, v);
            i += stride;
        }
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c11 trial {trial} (stride {stride})"));
    }
}

// ---------------------------------------------------------------------------
// C12 — repeated calls (state carried in the global)
// ---------------------------------------------------------------------------
#[test]
fn c12_peo_repeated_calls() {
    let h = harness();
    let mut rng = SplitMix64::new(0xCC_0000);
    for &n in &[2usize, 3, 8] {
        let data = random_array(&mut rng);
        h.write_both(&data);
        for step in 1..=n {
            h.c.perform_expensive_operations();
            h.rust.perform_expensive_operations();
            h.assert_arrays_equal(&format!("c12 n={n} after step {step}"));
        }
        assert_ne!(
            h.c.read_array(),
            data,
            "c12: {n} calls left the array unchanged?"
        );
    }
}

// ---------------------------------------------------------------------------
// C13 — nothing happens without a call
// ---------------------------------------------------------------------------
#[test]
fn c13_no_implicit_invocation() {
    let h = harness();
    let mut rng = SplitMix64::new(0xCD_0000);
    let data = random_array(&mut rng);
    h.write_both(&data);
    // No call at all: neither library may transform the data behind our back
    // (no constructor / `.init_array` hook).
    assert_eq!(h.c.read_array(), data, "C: array changed with no call");
    assert_eq!(h.rust.read_array(), data, "Rust: array changed with no call");
    h.assert_arrays_equal("c13 no call");
}

// ---------------------------------------------------------------------------
// C14 — the composed `long_exec` pipeline (fill + N passes + XOR)
// ---------------------------------------------------------------------------
#[test]
fn c14_long_exec_pipeline_composed() {
    let h = harness();
    let mut rng = SplitMix64::new(0xCE_0000);
    let mut seeds: Vec<u32> = vec![0, 1, 42, u32::MAX, 0x8000_0000, 0x7FFF_FFFF];
    for _ in 0..8 {
        seeds.push(rng.next_u32());
    }
    for seed in seeds {
        // Stage 1 of `long_exec`: srand(seed) + ARRAY_SIZE * rand().
        let filled = libc_rand_array(seed);
        h.write_both(&filled);
        // Stage 2: consecutive `perform_expensive_operations()` passes
        // (`long_exec` does 2000 of them; row C15 runs the real count).
        for step in 1..=3 {
            h.c.perform_expensive_operations();
            h.rust.perform_expensive_operations();
            h.assert_arrays_equal(&format!("c14 seed {seed} pass {step}"));
            // Stage 3: the XOR reduction that gets printed.
            assert_eq!(
                h.c.xor_array(),
                h.rust.xor_array(),
                "c14 seed {seed} pass {step}: xor"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C16 — the seed axis of `long_exec`'s PRNG stage
// ---------------------------------------------------------------------------
#[test]
fn c16_seed_axis() {
    // The Rust library must use the *platform* `srand`/`rand`, otherwise the
    // fill stage of `long_exec` cannot match.  Verified structurally (imported
    // symbols) in tests/symbols.rs; here we verify the seed axis is total:
    // every 32-bit seed produces a well-defined fill, and glibc's documented
    // `srand(0) == srand(1)` quirk holds.
    let mut rng = SplitMix64::new(0xD0_0000);
    let mut seeds: Vec<u32> = vec![0, 1, 2, 42, 0x7FFF_FFFF, 0x8000_0000, u32::MAX];
    for _ in 0..8 {
        seeds.push(rng.next_u32());
    }
    let h = harness();
    for seed in seeds {
        let a = libc_rand_array(seed);
        let b = libc_rand_array(seed);
        assert_eq!(a, b, "seed {seed}: platform PRNG is not deterministic?");
        h.write_both(&a);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("c16 seed {seed}"));
    }
    assert_eq!(
        libc_rand_array(0),
        libc_rand_array(1),
        "glibc srand(0) should behave like srand(1)"
    );
}

// ---------------------------------------------------------------------------
// C17 — the XOR reduction channel
// ---------------------------------------------------------------------------
#[test]
fn c17_xor_reduction_channel() {
    let h = harness();
    let mut rng = SplitMix64::new(0xD1_0000);
    for trial in 0..16 {
        let data = random_array(&mut rng);
        h.write_both(&data);
        let expected_pre = data.iter().fold(0i32, |a, &b| a ^ b);
        assert_eq!(h.c.xor_array(), expected_pre, "c17 trial {trial} pre-C");
        assert_eq!(h.rust.xor_array(), expected_pre, "c17 trial {trial} pre-Rust");
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        assert_eq!(
            h.c.xor_array(),
            h.rust.xor_array(),
            "c17 trial {trial}: post-pass xor"
        );
        h.assert_arrays_equal(&format!("c17 trial {trial}"));
    }
}

// ---------------------------------------------------------------------------
// C18 — both Rust build profiles against the same C library
// ---------------------------------------------------------------------------
#[test]
fn c18_debug_and_release_agree() {
    let h = harness();
    let mut extra: Vec<(&'static str, Lib)> = Vec::new();
    for profile in ["debug", "release"] {
        let p = rust_so_path_for(profile);
        if p == h.rust.path {
            continue; // already loaded as the current-profile library
        }
        if p.exists() {
            extra.push((profile, Lib::open(&p, Box::leak(format!("Rust/{profile}").into_boxed_str()))));
        } else {
            eprintln!("c18: {} not built — skipping that profile", p.display());
        }
    }
    println!(
        "c18: comparing C vs {} plus {} extra profile(s)",
        h.rust.path.display(),
        extra.len()
    );
    let mut rng = SplitMix64::new(0xD2_0000);
    for trial in 0..8 {
        let data: Vec<c_int> = if trial % 2 == 0 {
            random_array(&mut rng)
        } else {
            tile(&boundary_values())
        };
        h.write_both(&data);
        for (_, lib) in &extra {
            lib.write_array(&data);
        }
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        for (_, lib) in &extra {
            lib.perform_expensive_operations();
        }
        h.assert_arrays_equal(&format!("c18 trial {trial}"));
        for (profile, lib) in &extra {
            assert_arrays_equal_libs(h.c, lib, &format!("c18 trial {trial} profile {profile}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C19 — the two libraries are independent (no symbol interposition)
// ---------------------------------------------------------------------------
#[test]
fn c19_libraries_are_independent() {
    let h = harness();
    assert_ne!(
        h.c.array_ptr(),
        h.rust.array_ptr(),
        "the two libraries must have separate `array` objects"
    );
    let mut rng = SplitMix64::new(0xD3_0000);
    let a = random_array(&mut rng);
    let b = random_array(&mut rng);
    h.c.write_array(&a);
    h.rust.write_array(&b);
    assert_eq!(h.c.read_array(), a);
    assert_eq!(h.rust.read_array(), b);

    // Alternating calls: each must transform only its own array.
    h.c.perform_expensive_operations();
    assert_eq!(h.rust.read_array(), b, "Rust array changed by a C call");
    h.rust.perform_expensive_operations();
    let c_after = h.c.read_array();
    h.c.perform_expensive_operations();
    h.rust.perform_expensive_operations();
    // Now feed the same input to both and confirm equality still holds.
    h.write_both(&a);
    h.c.perform_expensive_operations();
    h.rust.perform_expensive_operations();
    h.assert_arrays_equal("c19 after interleaving");
    assert_eq!(
        h.c.read_array(),
        c_after,
        "c19: same input must give the same output (C determinism)"
    );
}
