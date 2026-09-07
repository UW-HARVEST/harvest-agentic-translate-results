//! Phase B / Phase C differential tests: every call goes through `dlsym` on
//! BOTH the C `.so` and the Rust `.so`.
//!
//! Covers CONFIGS.md rows 1-16 and 21, and ERRORS.md rows 8-15.
//! The `long_exec` end-to-end rows (CONFIGS 17-20, ERRORS 1-7) live in
//! `tests/long_exec_full.rs` because a single call is ~470 s in the C build.

mod common;

use common::*;
use std::ffi::c_void;

const SEED: u64 = 0x5EED_1234;

fn both() -> (Impl, Impl) {
    (Impl::c(), Impl::rust())
}

// ---------------------------------------------------------------- CONFIGS 1
#[test]
fn cfg01_fresh_array_is_zeroed_in_both() {
    let (c, r) = both();
    assert!(
        c.array().iter().all(|&v| v == 0),
        "C: freshly dlopen'ed `array` is not zeroed"
    );
    assert!(
        r.array().iter().all(|&v| v == 0),
        "Rust: freshly dlopen'ed `array` is not zeroed"
    );
    assert_eq!(c.array_bytes(), r.array_bytes());
    assert_eq!(c.array_bytes().len(), 0x10_0000, "array is 1 MiB");
}

// ---------------------------------------------------------------- CONFIGS 2
#[test]
fn cfg02_array_write_read_round_trip() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..4 {
        let input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
        c.set_array(&input);
        r.set_array(&input);
        assert_eq!(c.array(), &input[..], "C: array round trip");
        assert_eq!(r.array(), &input[..], "Rust: array round trip");
        assert_eq!(c.array_bytes(), r.array_bytes());
    }
}

// ------------------------------------------------- CONFIGS 3 / ERRORS 8
#[test]
fn cfg03_err08_all_zeros() {
    let (c, r) = both();
    diff_perform(&c, &r, &vec![0i32; ARRAY_SIZE], "all zeros");
}

// ------------------------------------------------- CONFIGS 4 / ERRORS 11,12
#[test]
fn cfg04_err11_err12_uniform_small_values() {
    let (c, r) = both();
    // Both signs, around the `% 7` and `/ 2` boundaries, plus multiples of 7.
    for v in [
        0i32, 1, -1, 2, -2, 3, -3, 6, -6, 7, -7, 8, -8, 13, -13, 14, -14, 49, -49, 100, -100,
    ] {
        diff_perform(&c, &r, &vec![v; ARRAY_SIZE], &format!("uniform {v}"));
    }
}

// ------------------------------------------------- CONFIGS 5 / ERRORS 9,10
#[test]
fn cfg05_err09_err10_uniform_overflow_edges() {
    let (c, r) = both();
    for v in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 7,
        i32::MIN / 3,
        i32::MAX / 3,
        i32::MIN / 2,
        i32::MAX / 2,
        -1 << 30,
        1 << 30,
        u32::MAX as i32,
        0x8000_0000u32 as i32,
        0x7FFF_FFFF,
    ] {
        diff_perform(&c, &r, &vec![v; ARRAY_SIZE], &format!("uniform {v}"));
    }
}

// ---------------------------------------------------------------- CONFIGS 6
#[test]
fn cfg06_ascending_ramp() {
    let (c, r) = both();
    let input: Vec<i32> = (0..ARRAY_SIZE).map(|i| i as i32).collect();
    diff_perform(&c, &r, &input, "ramp array[i] = i");
}

// ---------------------------------------------------------------- CONFIGS 7
#[test]
fn cfg07_descending_ramp() {
    let (c, r) = both();
    let input: Vec<i32> = (0..ARRAY_SIZE).map(|i| -(i as i32)).collect();
    diff_perform(&c, &r, &input, "ramp array[i] = -i");
}

// ---------------------------------------------------------------- CONFIGS 8
#[test]
fn cfg08_ramp_anchored_at_int_min() {
    let (c, r) = both();
    let input: Vec<i32> = (0..ARRAY_SIZE)
        .map(|i| i32::MIN.wrapping_add(i as i32))
        .collect();
    diff_perform(&c, &r, &input, "ramp from INT_MIN");
}

// ---------------------------------------------------------------- CONFIGS 9
#[test]
fn cfg09_ramp_straddling_int_max_wrap() {
    let (c, r) = both();
    let base = i32::MAX.wrapping_sub(131_072);
    let input: Vec<i32> = (0..ARRAY_SIZE).map(|i| base.wrapping_add(i as i32)).collect();
    diff_perform(&c, &r, &input, "ramp straddling INT_MAX wrap");
}

// --------------------------------------------------------------- CONFIGS 10
#[test]
fn cfg10_randomized_full_range() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 10);
    for round in 0..24 {
        let input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
        diff_perform(&c, &r, &input, &format!("uniform-random round {round}"));
    }
}

// --------------------------------------------------------------- CONFIGS 11
#[test]
fn cfg11_randomized_hard_value_mixture() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 11);
    for round in 0..24 {
        let input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.hard_i32()).collect();
        diff_perform(&c, &r, &input, &format!("hard-mixture round {round}"));
    }
}

// --------------------------------------------------------------- CONFIGS 12
#[test]
fn cfg12_sparse_single_element() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 12);
    let mut idxs = vec![0usize, 1, 6, 7, 8, 9, 15, 16, ARRAY_SIZE - 1, ARRAY_SIZE - 8];
    for _ in 0..8 {
        idxs.push((rng.next_u64() % ARRAY_SIZE as u64) as usize);
    }
    for k in idxs {
        for v in [1i32, -1, i32::MIN, i32::MAX] {
            let mut input = vec![0i32; ARRAY_SIZE];
            input[k] = v;
            diff_perform(&c, &r, &input, &format!("sparse idx {k} = {v}"));
        }
    }
}

// --------------------------------------------------------------- CONFIGS 13
#[test]
fn cfg13_chunk_remainder_tail() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 13);
    for tail in 1..=9usize {
        let mut input = vec![0i32; ARRAY_SIZE];
        for i in (ARRAY_SIZE - tail)..ARRAY_SIZE {
            input[i] = rng.next_i32();
        }
        diff_perform(&c, &r, &input, &format!("tail of {tail} non-zero"));
        // and the mirror case: only the leading `tail` elements non-zero
        let mut input = vec![0i32; ARRAY_SIZE];
        for i in 0..tail {
            input[i] = rng.next_i32();
        }
        diff_perform(&c, &r, &input, &format!("head of {tail} non-zero"));
    }
}

// --------------------------------------------------- CONFIGS 14 / ERRORS 13
#[test]
fn cfg14_err13_repeated_invocation_on_dirty_state() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 14);
    let input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.hard_i32()).collect();
    c.set_array(&input);
    r.set_array(&input);
    for call in 1..=25 {
        unsafe {
            c.perform();
            r.perform();
        }
        assert_arrays_eq(&c, &r, &format!("after back-to-back call #{call}"));
    }
}

// --------------------------------------------------------------- CONFIGS 15
#[test]
fn cfg15_contiguous_block_sweep_of_i32_domain() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 15);
    // Fixed blocks: the four quadrant anchors of the i32 range.
    let mut bases = vec![
        i32::MIN,
        i32::MIN / 2,
        -(ARRAY_SIZE as i32) / 2,
        0,
        i32::MAX / 2,
        i32::MAX - ARRAY_SIZE as i32 + 1,
    ];
    // Randomized block offsets covering the rest of the domain.
    for _ in 0..18 {
        bases.push(rng.next_i32());
    }
    for base in bases {
        let input: Vec<i32> = (0..ARRAY_SIZE).map(|i| base.wrapping_add(i as i32)).collect();
        diff_perform(&c, &r, &input, &format!("contiguous block at base {base}"));
    }
}

// --------------------------------------------------------------- CONFIGS 16
#[test]
fn cfg16_interleaved_caller_writes_between_calls() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 16);
    let input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
    c.set_array(&input);
    r.set_array(&input);
    for round in 0..12 {
        unsafe {
            c.perform();
            r.perform();
        }
        assert_arrays_eq(&c, &r, &format!("interleaved round {round}"));
        // Patch a handful of indices identically in both libraries.
        for _ in 0..64 {
            let k = (rng.next_u64() % ARRAY_SIZE as u64) as usize;
            let v = rng.hard_i32();
            c.array_mut()[k] = v;
            r.array_mut()[k] = v;
        }
        assert_arrays_eq(&c, &r, &format!("interleaved patch {round}"));
    }
}

// --------------------------------------------------------------- CONFIGS 21
/// The `srand(seed)` / `rand()` fill step: both `.so`s import the *same*
/// glibc symbols (same version), so the PRNG sequence is shared. Assert the
/// import parity mechanically, and check that a locally reproduced
/// `srand`+`rand` fill (262144 draws, ascending) feeds both libraries
/// identically through one transform pass.
#[test]
fn cfg21_srand_rand_fill_prefix() {
    use std::process::Command;

    let imports = |p: &std::path::Path| -> Vec<String> {
        let out = Command::new("nm").arg("-D").arg(p).output().expect("nm");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.contains(" U "))
            .filter(|l| l.contains("rand") || l.contains("printf"))
            .map(|l| l.split_whitespace().last().unwrap().to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    let ci = imports(&c_so_path());
    let ri = imports(&rust_so_path());
    assert!(!ci.is_empty(), "nm found no libc imports in the C .so");
    assert_eq!(ci, ri, "libc import parity (srand/rand/printf)");

    // Reproduce the exact fill `long_exec` performs and diff one pass.
    extern "C" {
        fn srand(seed: u32);
        fn rand() -> i32;
    }
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..6 {
        let seed = rng.next_u32();
        let input: Vec<i32> = unsafe {
            srand(seed);
            (0..ARRAY_SIZE).map(|_| rand()).collect()
        };
        diff_perform(&c, &r, &input, &format!("srand({seed}) fill + one pass"));
    }
}

// ---------------------------------------------------------------- ERRORS 14
/// `perform_expensive_operations` takes no parameters. The FFI-boundary
/// equivalent of an out-of-range enum is calling it through mis-shaped
/// function-pointer types with junk in the argument registers: the callee must
/// ignore them, identically in C and Rust.
#[test]
fn err14_junk_arguments_are_ignored() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 140);
    let input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.hard_i32()).collect();

    // Baseline: the honest zero-argument call.
    c.set_array(&input);
    r.set_array(&input);
    unsafe {
        c.perform();
        r.perform();
    }
    assert_arrays_eq(&c, &r, "baseline zero-arg perform");
    let expected: Vec<i32> = c.array().to_vec();

    type Junk6 = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64) -> i64;
    let cj: Junk6 = unsafe { std::mem::transmute(c.perform_addr) };
    let rj: Junk6 = unsafe { std::mem::transmute(r.perform_addr) };

    for round in 0..4 {
        let a = [
            rng.next_u64(),
            rng.next_u64(),
            u64::MAX,
            0,
            0xDEAD_BEEF_DEAD_BEEF,
            rng.next_u64(),
        ];
        c.set_array(&input);
        r.set_array(&input);
        unsafe {
            cj(a[0], a[1], a[2], a[3], a[4], a[5]);
            rj(a[0], a[1], a[2], a[3], a[4], a[5]);
        }
        assert_arrays_eq(&c, &r, &format!("junk-argument call round {round}"));
        assert_eq!(
            c.array(),
            &expected[..],
            "C: junk arguments changed the result (round {round})"
        );
        assert_eq!(
            r.array(),
            &expected[..],
            "Rust: junk arguments changed the result (round {round})"
        );
    }
}

// ---------------------------------------------------------------- ERRORS 15
/// Neither entry point takes a pointer, so there is no null-pointer path in the
/// library itself. What a caller *can* do is resolve the exported `array`
/// object and observe that it is a real, non-null, correctly sized `.bss`
/// object in both builds - assert that, plus that no extra symbol lookup
/// succeeds in one library but not the other.
#[test]
fn err15_no_pointer_parameters_symbol_shapes_match() {
    use libloading::{Library, Symbol};
    let (c, r) = both();
    assert_eq!(c.array_bytes().len(), r.array_bytes().len());

    unsafe {
        for path in [c_so_path(), rust_so_path()] {
            let lib = Library::new(&path).unwrap();
            let a: Symbol<*mut i32> = lib.get(b"array\0").unwrap();
            assert!(!(*a).is_null(), "{}: array is NULL", path.display());
            // A symbol that exists in neither library must fail in both.
            assert!(
                lib.get::<*mut c_void>(b"no_such_symbol_xyz\0").is_err(),
                "{}: bogus symbol resolved",
                path.display()
            );
        }
    }
}

// -------------------------------------------------- exhaustive-ish extra sweep
/// Wider randomized sweep of the element domain (property-style, fixed seed).
/// Kept separate so the mandatory rows above stay fast.
#[test]
#[ignore = "wide sweep; run with --ignored"]
fn sweep_wide_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0xA5A5_1234_5678_9ABC);
    for round in 0..200 {
        let input: Vec<i32> = (0..ARRAY_SIZE)
            .map(|_| if round % 2 == 0 { rng.next_i32() } else { rng.hard_i32() })
            .collect();
        diff_perform(&c, &r, &input, &format!("wide sweep round {round}"));
    }
}
