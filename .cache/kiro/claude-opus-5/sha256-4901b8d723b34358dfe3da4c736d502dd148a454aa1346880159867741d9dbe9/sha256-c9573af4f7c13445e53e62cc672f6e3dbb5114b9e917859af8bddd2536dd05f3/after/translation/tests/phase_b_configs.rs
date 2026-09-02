//! Phase B — valid-path differential tests, one `#[test]` per row of CONFIGS.md.
//!
//! Every test drives BOTH `.so` files through their exported C symbols and
//! compares the returned values, the mutated `struct ConfigFlags` storage bytes,
//! and the captured stdout/stderr bytes.

mod common;

use common::*;
use std::ffi::CString;

// ---------------------------------------------------------------------------
// Small call wrappers. Each returns an `i64` "observation" so a whole batch of
// calls can be compared as one sequence.
// ---------------------------------------------------------------------------

fn call_parse(lib: &Lib, name: &str, default_val: i32) -> i64 {
    let n = CString::new(name).unwrap();
    unsafe { (lib.parse_env_numeric)(n.as_ptr(), default_val) as i64 }
}

/// Runs `init_config_from_env` on a storage word and returns the full 4-byte
/// result, so padding-bit preservation is observed too.
fn call_init(lib: &Lib, initial: u32) -> i64 {
    let mut storage: u32 = initial;
    unsafe { (lib.init_config_from_env)(&mut storage) };
    storage as i64
}

fn call_perform(lib: &Lib, val1: i32, val2: i32, flags: u32) -> i64 {
    let mut storage = flags;
    let r = unsafe { (lib.perform_operation)(val1, val2, &mut storage) };
    // Fold the (expected unchanged) flags word in as well, so an accidental
    // write through the pointer would be caught.
    ((storage as i64) << 32) | (r as u32 as i64)
}

fn call_apply(lib: &Lib, value: i32, flags: u32) -> i64 {
    let mut storage = flags;
    let r = unsafe { (lib.apply_bit_operations)(value, &mut storage) };
    ((storage as i64) << 32) | (r as u32 as i64)
}

fn call_envy(lib: &Lib, p1: i32, p2: i32, p3: i32, p4: i32) -> i64 {
    unsafe { (lib.envy)(p1, p2, p3, p4) as i64 }
}

/// The three states `PROG_VERBOSE` / `PROG_DEBUG` are distinguished by
/// (`strchr(v, '1')`), and the two states `PROG_OPTIMIZE` is distinguished by
/// (presence only).
const TRISTATE: [Option<&str>; 3] = [None, Some("0"), Some("1")];
const OPT_STATE: [Option<&str>; 2] = [None, Some("")];

fn apply_env(verbose: Option<&str>, debug: Option<&str>, optimize: Option<&str>) {
    env_clear_all();
    if let Some(v) = verbose {
        env_set("PROG_VERBOSE", v);
    }
    if let Some(v) = debug {
        env_set("PROG_DEBUG", v);
    }
    if let Some(v) = optimize {
        env_set("PROG_OPTIMIZE", v);
    }
}

// ===========================================================================
// Rows 1–4: parse_env_numeric
// ===========================================================================

#[test]
fn row01_parse_env_numeric_variable_absent() {
    let _g = lock();
    env_clear_all();
    diff("CONFIGS row 1", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 1);
        for d in BOUNDS {
            out.push(call_parse(lib, "PROG_BASE_OFFSET", d));
            out.push(call_parse(lib, "PROG_MULTIPLIER", d));
            out.push(call_parse(lib, "DEFINITELY_NOT_SET_XYZZY_42", d));
        }
        for _ in 0..256 {
            out.push(call_parse(lib, "DEFINITELY_NOT_SET_XYZZY_42", rng.next_i32()));
        }
        out
    });
}

#[test]
fn row02_parse_env_numeric_accepted_values() {
    let _g = lock();
    diff("CONFIGS row 2", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 2);
        // Hand-picked lexical shapes atoi must handle identically.
        for s in [
            "+7", " 42", "007", "-0", "0", "1", "-1", "2147483647", "-2147483648", "  -00123",
            "\t9", "\n5", "3.9", "5e3",
        ] {
            env_set("PROG_BASE_OFFSET", s);
            for d in BOUNDS {
                out.push(call_parse(lib, "PROG_BASE_OFFSET", d));
            }
        }
        // Randomised round-trip: every i32 written as decimal must come back.
        for _ in 0..256 {
            let v = rng.next_i32();
            env_set("PROG_BASE_OFFSET", &v.to_string());
            out.push(call_parse(lib, "PROG_BASE_OFFSET", rng.next_i32()));
        }
        env_clear_all();
        out
    });
}

#[test]
fn row03_parse_env_numeric_comma_rejected() {
    let _g = lock();
    diff("CONFIGS row 3", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 3);
        for s in [",", "1,2", "abc,", ",99", "1,,2", "-5,"] {
            env_set("PROG_MULTIPLIER", s);
            for d in BOUNDS {
                out.push(call_parse(lib, "PROG_MULTIPLIER", d));
            }
            for _ in 0..16 {
                out.push(call_parse(lib, "PROG_MULTIPLIER", rng.next_i32()));
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row04_parse_env_numeric_semicolon_rejected() {
    let _g = lock();
    diff("CONFIGS row 4", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 4);
        for s in [";", "1;2", "abc;", ";99", "1;;2", "-5;"] {
            env_set("PROG_MULTIPLIER", s);
            for d in BOUNDS {
                out.push(call_parse(lib, "PROG_MULTIPLIER", d));
            }
            for _ in 0..16 {
                out.push(call_parse(lib, "PROG_MULTIPLIER", rng.next_i32()));
            }
        }
        env_clear_all();
        out
    });
}

// ===========================================================================
// Rows 5–7: init_config_from_env
// ===========================================================================

#[test]
fn row05_init_config_all_18_env_states_zeroed_storage() {
    let _g = lock();
    diff("CONFIGS row 5", |lib| {
        let mut out = Vec::new();
        for v in TRISTATE {
            for d in TRISTATE {
                for o in OPT_STATE {
                    apply_env(v, d, o);
                    out.push(call_init(lib, 0));
                }
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row06_init_config_all_18_env_states_garbage_padding() {
    let _g = lock();
    diff("CONFIGS row 6", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 6);
        for v in TRISTATE {
            for d in TRISTATE {
                for o in OPT_STATE {
                    apply_env(v, d, o);
                    for pad in GARBAGE_PADDING {
                        // Pre-existing garbage in every bit, including the low
                        // byte, so the overwrite semantics are pinned down.
                        out.push(call_init(lib, pad));
                        out.push(call_init(lib, pad | 0xFF));
                        out.push(call_init(lib, rng.next_u32()));
                    }
                }
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row07_init_config_one_detection_shapes() {
    let _g = lock();
    diff("CONFIGS row 7", |lib| {
        let mut out = Vec::new();
        let shapes = [
            "1", "01", "10", "a1", "1a", "111", "21", "", "0", "true", "one", "yes", "  1  ",
            "\u{31}", "2", "\x01",
        ];
        for s in shapes {
            for t in shapes {
                env_clear_all();
                env_set("PROG_VERBOSE", s);
                env_set("PROG_DEBUG", t);
                out.push(call_init(lib, 0));
                env_set("PROG_OPTIMIZE", s);
                out.push(call_init(lib, 0xFFFF_FFFF));
            }
        }
        env_clear_all();
        out
    });
}

// ===========================================================================
// Rows 8–14: perform_operation (low-level entry point, driven directly)
// ===========================================================================

fn perform_sweep(row: &str, flag_words: Vec<u32>, random_pairs: usize, seed: u64) {
    diff(row, move |lib| {
        let mut out = Vec::new();
        for &fw in &flag_words {
            for a in BOUNDS {
                for b in BOUNDS {
                    out.push(call_perform(lib, a, b, fw));
                }
            }
            let mut rng = Rng::new(seed ^ fw as u64);
            for _ in 0..random_pairs {
                let (a, b) = (rng.next_i32(), rng.next_i32());
                out.push(call_perform(lib, a, b, fw));
            }
        }
        out
    });
}

#[test]
fn row08_perform_optimize_on() {
    let _g = lock();
    let words = (0..8)
        .map(|ll| flags_word(0, 0, 1, 1, ll, 0, 0))
        .collect::<Vec<_>>();
    perform_sweep("CONFIGS row 8", words, 512, SEED ^ 8);
}

#[test]
fn row09_perform_optimize_off_log_level_zero() {
    let _g = lock();
    perform_sweep(
        "CONFIGS row 9",
        vec![
            flags_word(0, 0, 0, 0, 0, 0, 0),
            flags_word(0, 0, 0, 1, 0, 0, 0),
        ],
        512,
        SEED ^ 9,
    );
}

#[test]
fn row10_perform_optimize_off_log_level_three() {
    let _g = lock();
    perform_sweep(
        "CONFIGS row 10",
        vec![
            flags_word(0, 0, 0, 1, 3, 0, 0),
            flags_word(1, 0, 0, 1, 3, 0, 0),
        ],
        512,
        SEED ^ 10,
    );
}

#[test]
fn row11_perform_optimize_off_every_other_log_level() {
    let _g = lock();
    let words = [1u32, 2, 4, 5, 6, 7]
        .iter()
        .map(|&ll| flags_word(0, 0, 0, 1, ll, 0, 0))
        .collect::<Vec<_>>();
    perform_sweep("CONFIGS row 11", words, 256, SEED ^ 11);
}

#[test]
fn row12_perform_debug_output() {
    let _g = lock();
    let words = vec![
        flags_word(0, 1, 0, 1, 3, 0, 0),
        flags_word(0, 1, 1, 1, 3, 0, 0),
        flags_word(1, 1, 0, 0, 7, 1, 0),
        flags_word(1, 1, 1, 1, 0, 1, 0),
    ];
    perform_sweep("CONFIGS row 12", words, 64, SEED ^ 12);
}

#[test]
fn row13_perform_all_256_flag_bytes() {
    let _g = lock();
    diff("CONFIGS row 13", |lib| {
        let mut out = Vec::new();
        for byte in 0u32..256 {
            for a in BOUNDS {
                for b in BOUNDS {
                    out.push(call_perform(lib, a, b, byte));
                }
            }
            let mut rng = Rng::new(SEED ^ 13 ^ byte as u64);
            for _ in 0..64 {
                let (a, b) = (rng.next_i32(), rng.next_i32());
                out.push(call_perform(lib, a, b, byte));
            }
        }
        out
    });
}

#[test]
fn row14_perform_garbage_padding_bits() {
    let _g = lock();
    diff("CONFIGS row 14", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 14);
        for pad in GARBAGE_PADDING {
            for byte in 0u32..256 {
                let fw = pad | byte;
                for a in [i32::MIN, -7, 0, 7, i32::MAX] {
                    for b in [i32::MIN, -7, 0, 7, i32::MAX] {
                        out.push(call_perform(lib, a, b, fw));
                    }
                }
                out.push(call_perform(lib, rng.next_i32(), rng.next_i32(), fw));
            }
        }
        out
    });
}

// ===========================================================================
// Rows 15–19: apply_bit_operations (low-level entry point, driven directly)
// ===========================================================================

fn apply_sweep(row: &str, flag_words: Vec<u32>, random_values: usize, seed: u64) {
    diff(row, move |lib| {
        let mut out = Vec::new();
        for &fw in &flag_words {
            for v in BOUNDS {
                out.push(call_apply(lib, v, fw));
            }
            // Values that make `<< 1` overflow in every interesting way.
            for v in [
                0x4000_0000,
                0x4000_0001,
                0x3FFF_FFFF,
                -0x4000_0000i32,
                -0x4000_0001i32,
                0x7FFF_FFF0,
                0x0000_000F,
                0x0000_0010,
                -16,
                -15,
            ] {
                out.push(call_apply(lib, v, fw));
            }
            let mut rng = Rng::new(seed ^ fw as u64);
            for _ in 0..random_values {
                out.push(call_apply(lib, rng.next_i32(), fw));
                out.push(call_apply(lib, rng.next_u32() as i32, fw));
            }
        }
        out
    });
}

#[test]
fn row15_apply_verbose_off_cache_off() {
    let _g = lock();
    apply_sweep(
        "CONFIGS row 15",
        vec![flags_word(0, 0, 0, 0, 0, 0, 0)],
        512,
        SEED ^ 15,
    );
}

#[test]
fn row16_apply_verbose_off_cache_on() {
    let _g = lock();
    apply_sweep(
        "CONFIGS row 16",
        vec![flags_word(0, 0, 0, 1, 3, 0, 0)],
        512,
        SEED ^ 16,
    );
}

#[test]
fn row17_apply_verbose_on_cache_off() {
    let _g = lock();
    apply_sweep(
        "CONFIGS row 17",
        vec![flags_word(1, 0, 0, 0, 3, 0, 0)],
        512,
        SEED ^ 17,
    );
}

#[test]
fn row18_apply_verbose_on_cache_on() {
    let _g = lock();
    apply_sweep(
        "CONFIGS row 18",
        vec![flags_word(1, 0, 0, 1, 3, 0, 0)],
        512,
        SEED ^ 18,
    );
}

#[test]
fn row19_apply_all_256_flag_bytes_and_padding() {
    let _g = lock();
    diff("CONFIGS row 19", |lib| {
        let mut out = Vec::new();
        for pad in GARBAGE_PADDING {
            for byte in 0u32..256 {
                let fw = pad | byte;
                for v in BOUNDS {
                    out.push(call_apply(lib, v, fw));
                }
                let mut rng = Rng::new(SEED ^ 19 ^ fw as u64);
                for _ in 0..8 {
                    out.push(call_apply(lib, rng.next_i32(), fw));
                }
            }
        }
        out
    });
}

// ===========================================================================
// Rows 20–29: envy, end to end
// ===========================================================================

fn envy_sweep(row: &str, env: Vec<(&'static str, &'static str)>, randoms: usize, seed: u64) {
    diff(row, move |lib| {
        let mut out = Vec::new();
        env_clear_all();
        for (k, v) in &env {
            env_set(k, v);
        }
        // Boundary 4-tuples: sweep each parameter over BOUNDS while the others
        // take a few representative values.
        for p in BOUNDS {
            for q in [-1i32, 0, 1, 7] {
                out.push(call_envy(lib, p, q, q, q));
                out.push(call_envy(lib, q, p, q, q));
                out.push(call_envy(lib, q, q, p, q));
                out.push(call_envy(lib, q, q, q, p));
            }
        }
        let mut rng = Rng::new(seed);
        for _ in 0..randoms {
            let (a, b, c, d) = (
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            );
            out.push(call_envy(lib, a, b, c, d));
        }
        env_clear_all();
        out
    });
}

#[test]
fn row20_envy_all_env_absent() {
    let _g = lock();
    envy_sweep("CONFIGS row 20", vec![], 512, SEED ^ 20);
}

#[test]
fn row21_envy_optimize_only() {
    let _g = lock();
    envy_sweep(
        "CONFIGS row 21",
        vec![("PROG_OPTIMIZE", "")],
        512,
        SEED ^ 21,
    );
}

#[test]
fn row22_envy_verbose_only() {
    let _g = lock();
    envy_sweep(
        "CONFIGS row 22",
        vec![("PROG_VERBOSE", "1")],
        256,
        SEED ^ 22,
    );
}

#[test]
fn row23_envy_debug_only() {
    let _g = lock();
    envy_sweep("CONFIGS row 23", vec![("PROG_DEBUG", "1")], 256, SEED ^ 23);
}

#[test]
fn row24_envy_all_eight_flag_combinations() {
    let _g = lock();
    diff("CONFIGS row 24", |lib| {
        let mut out = Vec::new();
        for v in [false, true] {
            for d in [false, true] {
                for o in [false, true] {
                    apply_env(
                        if v { Some("1") } else { Some("0") },
                        if d { Some("1") } else { Some("0") },
                        if o { Some("") } else { None },
                    );
                    for p in BOUNDS {
                        out.push(call_envy(lib, p, p, p, p));
                        out.push(call_envy(lib, p, 1, 0, 0));
                        out.push(call_envy(lib, 1, p, 3, 5));
                    }
                    let mut rng = Rng::new(SEED ^ 24 ^ ((v as u64) << 2 | (d as u64) << 1 | o as u64));
                    for _ in 0..128 {
                        out.push(call_envy(
                            lib,
                            rng.next_i32(),
                            rng.next_i32(),
                            rng.next_i32(),
                            rng.next_i32(),
                        ));
                    }
                }
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row25_envy_numeric_env_interaction() {
    let _g = lock();
    diff("CONFIGS row 25", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 25);
        let offsets: Vec<String> = [
            "0",
            "1",
            "-1",
            "64",
            "-64",
            "2147483647",
            "-2147483648",
            "100",
            "-1000000",
        ]
        .iter()
        .map(|s| s.to_string())
        .chain((0..6).map(|_| rng.next_i32().to_string()))
        .collect();
        let mults: Vec<String> = ["0", "1", "-1", "10", "-10", "2147483647", "-2147483648"]
            .iter()
            .map(|s| s.to_string())
            .chain((0..5).map(|_| rng.next_i32().to_string()))
            .collect();

        for off in &offsets {
            for m in &mults {
                env_clear_all();
                env_set("PROG_BASE_OFFSET", off);
                env_set("PROG_MULTIPLIER", m);
                for p in [i32::MIN, -1000, -1, 0, 1, 1000, i32::MAX] {
                    out.push(call_envy(lib, p, p, 1, 1));
                    out.push(call_envy(lib, p, 2, p, 4));
                }
                let mut r2 = Rng::new(SEED ^ 25 ^ off.len() as u64 ^ ((m.len() as u64) << 8));
                for _ in 0..24 {
                    out.push(call_envy(
                        lib,
                        r2.next_i32(),
                        r2.next_i32(),
                        r2.next_i32(),
                        r2.next_i32(),
                    ));
                }
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row26_envy_rejected_numeric_env_with_all_flag_combos() {
    let _g = lock();
    diff("CONFIGS row 26", |lib| {
        let mut out = Vec::new();
        for (off, mult) in [
            (",", ";"),
            (";", ","),
            ("1,2", "3;4"),
            (",", ","),
            (";", ";"),
            ("5", ","),
            (",", "5"),
        ] {
            for v in [false, true] {
                for d in [false, true] {
                    for o in [false, true] {
                        env_clear_all();
                        if v {
                            env_set("PROG_VERBOSE", "1");
                        }
                        if d {
                            env_set("PROG_DEBUG", "1");
                        }
                        if o {
                            env_set("PROG_OPTIMIZE", "");
                        }
                        env_set("PROG_BASE_OFFSET", off);
                        env_set("PROG_MULTIPLIER", mult);
                        for p in [i32::MIN, -5, 0, 5, i32::MAX] {
                            out.push(call_envy(lib, p, 3, 2, 9));
                        }
                    }
                }
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row27_envy_input_shape_grid() {
    let _g = lock();
    diff("CONFIGS row 27", |lib| {
        let mut out = Vec::new();
        env_clear_all();
        let p3s: [i32; 2] = [0, 13];
        let p4s: [i32; 3] = [0, 17, -17];
        for p3 in p3s {
            for p4 in p4s {
                for s1 in [-1i32, 1] {
                    for s2 in [-1i32, 1] {
                        let mut rng = Rng::new(
                            SEED ^ 27
                                ^ (p3 as u32 as u64)
                                ^ ((p4 as u32 as u64) << 16)
                                ^ ((s1 as u32 as u64) << 32)
                                ^ ((s2 as u32 as u64) << 48),
                        );
                        for _ in 0..64 {
                            let m1 = (rng.next_u32() % 100_000) as i32;
                            let m2 = (rng.next_u32() % 100_000) as i32;
                            out.push(call_envy(lib, s1 * m1, s2 * m2, p3, p4));
                        }
                        out.push(call_envy(lib, s1 * i32::MAX, s2 * i32::MAX, p3, p4));
                        out.push(call_envy(lib, i32::MIN, i32::MIN, p3, p4));
                    }
                }
            }
        }
        out
    });
}

#[test]
fn row28_envy_negative_result_restores_backup() {
    let _g = lock();
    diff("CONFIGS row 28", |lib| {
        let mut out = Vec::new();
        for v in [false, true] {
            for d in [false, true] {
                for o in [false, true] {
                    for off in ["-2147483648", "-1000000", "-64", "-1"] {
                        env_clear_all();
                        if v {
                            env_set("PROG_VERBOSE", "1");
                        }
                        if d {
                            env_set("PROG_DEBUG", "1");
                        }
                        if o {
                            env_set("PROG_OPTIMIZE", "1");
                        }
                        env_set("PROG_BASE_OFFSET", off);
                        env_set("PROG_MULTIPLIER", "-1000");
                        for p1 in [i32::MIN, -100, -1, 0, 1, 100, i32::MAX] {
                            for p2 in [i32::MIN, -100, 0, 100, i32::MAX] {
                                out.push(call_envy(lib, p1, p2, 1, 1));
                                out.push(call_envy(lib, p1, p2, 0, 0));
                            }
                        }
                    }
                }
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row29_envy_param4_arithmetic_shift() {
    let _g = lock();
    diff("CONFIGS row 29", |lib| {
        let mut out = Vec::new();
        let p4s: [i32; 14] = [
            i32::MIN,
            i32::MIN + 1,
            -8,
            -5,
            -4,
            -3,
            -2,
            -1,
            1,
            2,
            3,
            4,
            5,
            i32::MAX,
        ];
        for o in [false, true] {
            env_clear_all();
            if o {
                env_set("PROG_OPTIMIZE", "1");
            }
            for p4 in p4s {
                for p1 in [0i32, 1, -1, 1000, -1000] {
                    for p2 in [0i32, 2, -2, 1000, -1000] {
                        out.push(call_envy(lib, p1, p2, 0, p4));
                        out.push(call_envy(lib, p1, p2, 3, p4));
                    }
                }
            }
        }
        env_clear_all();
        out
    });
}

// ===========================================================================
// Row 30: the low-level functions composed into the full pipeline by the test,
// rather than going through the `envy` convenience wrapper.
// ===========================================================================

#[test]
fn row30_composed_pipeline_low_level() {
    let _g = lock();
    diff("CONFIGS row 30", |lib| {
        let mut out = Vec::new();
        for v in TRISTATE {
            for d in TRISTATE {
                for o in OPT_STATE {
                    apply_env(v, d, o);
                    let mut rng = Rng::new(
                        SEED ^ 30
                            ^ (v.map_or(9, |s| s.len() as u64))
                            ^ ((d.map_or(9, |s| s.len() as u64)) << 8)
                            ^ ((o.map_or(9, |s| s.len() as u64)) << 16),
                    );
                    for _ in 0..256 {
                        let (a, b) = (rng.next_i32(), rng.next_i32());

                        // init_config_from_env -> perform_operation ->
                        // apply_bit_operations, exactly as `envy` chains them,
                        // but assembled here so the composition is under test.
                        let mut storage: u32 = rng.next_u32();
                        unsafe { (lib.init_config_from_env)(&mut storage) };
                        out.push(storage as i64);

                        let base_offset = {
                            let n = std::ffi::CString::new("PROG_BASE_OFFSET").unwrap();
                            unsafe { (lib.parse_env_numeric)(n.as_ptr(), 0o100) }
                        };
                        let multiplier = {
                            let n = std::ffi::CString::new("PROG_MULTIPLIER").unwrap();
                            unsafe { (lib.parse_env_numeric)(n.as_ptr(), 0o12) }
                        };
                        out.push(base_offset as i64);
                        out.push(multiplier as i64);

                        let mut result =
                            unsafe { (lib.perform_operation)(a, b, &mut storage) };
                        out.push(result as i64);

                        let p3 = rng.next_i32();
                        if p3 != 0 {
                            result = result.wrapping_add(p3.wrapping_mul(multiplier));
                        }
                        let p4 = rng.next_i32();
                        if p4 != 0 {
                            result = result.wrapping_add(p4 >> 2);
                        }

                        result = unsafe { (lib.apply_bit_operations)(result, &mut storage) };
                        out.push(result as i64);

                        result = result.wrapping_add(base_offset);
                        out.push(result as i64);
                        out.push(storage as i64);
                    }
                }
            }
        }
        env_clear_all();
        out
    });
}
