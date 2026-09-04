//! Differential tests: C `libdriver.so` vs Rust `libdriver.so`, both loaded
//! with `libloading` and driven through their exported `driver` symbol.
//!
//! Row numbers refer to `CONFIGS.md` (valid paths) and `ERRORS.md` (rejections
//! and boundaries).

mod common;

use common::*;
use core::ffi::c_char;

// ---------------------------------------------------------------------------
// Phase A sanity: both objects really export `driver`.
// ---------------------------------------------------------------------------

#[test]
fn symbol_parity_driver_is_loadable_from_both() {
    let p = libs();
    // `libs()` panics if either `dlsym("driver")` fails.
    assert_eq!(p.c.name, "C libdriver.so");
    assert_eq!(p.rs.name, "Rust libdriver.so");
    // And it is actually callable through the FFI boundary on both sides.
    let a = capture_to_file(|| p.c.call(b'a' as c_char));
    let b = capture_to_file(|| p.rs.call(b'a' as c_char));
    assert!(!a.is_empty(), "C produced no output");
    assert_same("smoke driver('a')", &a, &b);
}

// ---------------------------------------------------------------------------
// CONFIGS row 1 — exhaustive over every possible `char`.
// Subsumes ERRORS rows 2..=13 (every boundary value is in this sweep).
// ---------------------------------------------------------------------------

#[test]
fn configs_row_01_exhaustive_all_char_values() {
    for v in ALL_CHARS {
        diff_char(v as c_char);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 2 — negative half only, visited in randomized order.
// (ERRORS row 6: the whole high-bit-set byte range.)
// ---------------------------------------------------------------------------

#[test]
fn configs_row_02_negative_chars_randomized() {
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    for _ in 0..256 {
        let v = -128 + (rng.next_u64() % 128) as i16;
        diff_char(v as c_char);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 3..=12 — one row per distinct classification shape.
// ---------------------------------------------------------------------------

fn diff_set(label: &str, vals: &[i16]) {
    assert!(!vals.is_empty(), "{label}: empty value set");
    for &v in vals {
        diff_char(v as c_char);
    }
}

#[test]
fn configs_row_03_nul_char() {
    // ERRORS row 2 as well: the output contains an embedded NUL byte.
    let p = libs();
    let a = capture_to_file(|| p.c.call(0));
    let b = capture_to_file(|| p.rs.call(0));
    assert_same("driver(0)", &a, &b);
    assert!(a.contains(&0u8), "expected an embedded NUL byte in the output");
}

#[test]
fn configs_row_04_c0_controls_not_space() {
    let mut vals: Vec<i16> = (1..=8).collect();
    vals.extend(14..=31);
    diff_set("C0 controls (non-space)", &vals);
}

#[test]
fn configs_row_05_tab_cntrl_space_blank() {
    diff_set("tab", &[9]);
}

#[test]
fn configs_row_06_whitespace_cntrl_not_blank() {
    diff_set("\\n \\v \\f \\r", &[10, 11, 12, 13]);
}

#[test]
fn configs_row_07_space_print_space_blank_not_graph() {
    diff_set("space", &[32]);
}

#[test]
fn configs_row_08_digits() {
    let vals: Vec<i16> = ((b'0' as i16)..=(b'9' as i16)).collect();
    diff_set("digits", &vals);
}

#[test]
fn configs_row_09_hex_letters() {
    let mut vals: Vec<i16> = ((b'A' as i16)..=(b'F' as i16)).collect();
    vals.extend((b'a' as i16)..=(b'f' as i16));
    diff_set("hex letters", &vals);
}

#[test]
fn configs_row_10_non_hex_letters() {
    let mut vals: Vec<i16> = ((b'G' as i16)..=(b'Z' as i16)).collect();
    vals.extend((b'g' as i16)..=(b'z' as i16));
    diff_set("non-hex letters", &vals);
}

#[test]
fn configs_row_11_all_punctuation() {
    let punct = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
    assert_eq!(punct.len(), 32, "the C locale has exactly 32 punct chars");
    let vals: Vec<i16> = punct.iter().map(|&b| b as i16).collect();
    diff_set("punctuation", &vals);
}

#[test]
fn configs_row_12_del() {
    diff_set("DEL", &[127]);
}

// ---------------------------------------------------------------------------
// CONFIGS rows 13..=16 — locale axis.
// ---------------------------------------------------------------------------

fn diff_with_locale(locale: &str, draws: usize, seed: u64) {
    let p = libs();
    let mut rng = Rng::new(seed);
    for _ in 0..draws {
        let c = rng.next_char();
        // Each library sees exactly the same pre-call locale state.
        set_locale(locale);
        let a = capture_to_file(|| p.c.call(c));
        let after_c = query_locale();
        set_locale(locale);
        let b = capture_to_file(|| p.rs.call(c));
        let after_rs = query_locale();
        assert_same(&format!("locale={locale} driver({c})"), &a, &b);
        assert_eq!(
            after_c, after_rs,
            "post-call locale differs for locale={locale}, c={c}"
        );
    }
    set_locale("C");
}

#[test]
fn configs_row_13_non_c_locale() {
    // Try a handful; whichever ones exist on this box get exercised, and the
    // ones that do not fold into row 14 (setlocale failure).
    for loc in ["en_US.UTF-8", "en_US.utf8", "de_DE.UTF-8"] {
        let available = set_locale(loc);
        set_locale("C");
        if available {
            diff_with_locale(loc, 64, 0x1111_2222_3333_4444);
        }
    }
    // Always run at least one non-default locale case that is guaranteed to
    // exist: the POSIX alias of the C locale.
    if {
        let ok = set_locale("POSIX");
        set_locale("C");
        ok
    } {
        diff_with_locale("POSIX", 64, 0x5555_6666_7777_8888);
    }
}

#[test]
fn configs_row_14_unavailable_locale_setlocale_failed() {
    // ERRORS row 1: `setlocale` may fail; `driver` discards the return value.
    let bogus = "no_SUCH.locale-42";
    assert!(!set_locale(bogus), "expected `{bogus}` to be unavailable");
    diff_with_locale(bogus, 128, 0x9999_AAAA_BBBB_CCCC);
}

#[test]
fn configs_row_15_c_utf8_locale() {
    let available = {
        let ok = set_locale("C.UTF-8");
        set_locale("C");
        ok
    };
    if available {
        diff_with_locale("C.UTF-8", 128, 0xDDDD_EEEE_FFFF_0001);
    } else {
        // Fall back to the always-present "C" so the row still runs.
        diff_with_locale("C", 128, 0xDDDD_EEEE_FFFF_0001);
    }
}

#[test]
fn configs_row_16_locale_left_as_c_after_call() {
    // ERRORS row 15: the global side effect must match, not just stdout.
    let p = libs();
    for start in ["C", "POSIX", "en_US.UTF-8", "no_SUCH.locale-42"] {
        set_locale(start);
        let before = query_locale();
        let _ = capture_to_file(|| p.c.call(b'Q' as c_char));
        let after_c = query_locale();

        set_locale(start);
        assert_eq!(query_locale(), before, "locale setup is not reproducible");
        let _ = capture_to_file(|| p.rs.call(b'Q' as c_char));
        let after_rs = query_locale();

        assert_eq!(
            after_c, after_rs,
            "post-call locale mismatch (started from {start})"
        );
        assert_eq!(after_c, "C", "driver() must leave the locale as \"C\"");
    }
    set_locale("C");
}

// ---------------------------------------------------------------------------
// CONFIGS row 17 / ERRORS row 7 — values wider than `char` across the FFI
// boundary, including out-of-range "enum-like" ints.
// ---------------------------------------------------------------------------

#[test]
fn configs_row_17_out_of_char_range_arguments() {
    let vals: [i32; 26] = [
        128,
        129,
        200,
        254,
        255,
        256,
        257,
        383,
        384,
        511,
        0x1FF,
        0x100,
        1000,
        65535,
        65536,
        -129,
        -130,
        -200,
        -255,
        -256,
        -257,
        -1000,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    for v in vals {
        diff_wide(v);
    }
}

#[test]
fn configs_row_17b_randomized_wide_arguments() {
    let mut rng = Rng::new(0x0BAD_F00D_DEAD_BEEF);
    for _ in 0..512 {
        diff_wide(rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 18..=19 — stdout destination / buffering mode.
// ---------------------------------------------------------------------------

#[test]
fn configs_row_18_stdout_is_a_regular_file() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for _ in 0..256 {
        let c = rng.next_char();
        let p = libs();
        let a = capture_to_file(|| p.c.call(c));
        let b = capture_to_file(|| p.rs.call(c));
        assert_same(&format!("file-buffered driver({c})"), &a, &b);
    }
}

#[test]
fn configs_row_19_stdout_is_a_pipe() {
    let mut rng = Rng::new(0x0FED_CBA9_8765_4321);
    let p = libs();
    for _ in 0..256 {
        let c = rng.next_char();
        let a = capture_to_pipe(|| p.c.call(c));
        let b = capture_to_pipe(|| p.rs.call(c));
        assert_same(&format!("pipe-buffered driver({c})"), &a, &b);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 20..=22 — call multiplicity and interleaving.
// ---------------------------------------------------------------------------

#[test]
fn configs_row_20_repeated_calls_same_value() {
    let p = libs();
    for &c in &[0i16, 9, 32, 65, 97, 127, -1, -128] {
        let c = c as c_char;
        let a = capture_to_file(|| {
            for _ in 0..32 {
                p.c.call(c)
            }
        });
        let b = capture_to_file(|| {
            for _ in 0..32 {
                p.rs.call(c)
            }
        });
        assert_same(&format!("32x driver({c})"), &a, &b);
        // No hidden state: the block is the single-call output repeated.
        let one = capture_to_file(|| p.rs.call(c));
        let mut expect = Vec::new();
        for _ in 0..32 {
            expect.extend_from_slice(&one);
        }
        assert_same(&format!("idempotence driver({c})"), &expect, &b);
    }
}

#[test]
fn configs_row_21_long_randomized_sequence_in_one_capture() {
    let p = libs();
    let mut r1 = Rng::new(0xABCD_1234_ABCD_1234);
    let mut r2 = Rng::new(0xABCD_1234_ABCD_1234);
    let a = capture_to_file(|| {
        for _ in 0..256 {
            p.c.call(r1.next_char())
        }
    });
    let b = capture_to_file(|| {
        for _ in 0..256 {
            p.rs.call(r2.next_char())
        }
    });
    assert_same("256-call sequence", &a, &b);
}

#[test]
fn configs_row_22_interleaved_with_caller_printf() {
    unsafe extern "C" {
        fn printf(fmt: *const c_char, ...) -> core::ffi::c_int;
    }
    let p = libs();
    let pre = b"before %d\n\0";
    let post = b"after %d\n\0";
    let mut rng = Rng::new(0x2468_ACE0_1357_9BDF);
    for i in 0..64 {
        let c = rng.next_char();
        let run = |lib: &common::Lib| {
            capture_to_file(|| unsafe {
                printf(pre.as_ptr() as *const c_char, i);
                lib.call(c);
                printf(post.as_ptr() as *const c_char, i);
                lib.call(c);
            })
        };
        let a = run(&p.c);
        let b = run(&p.rs);
        assert_same(&format!("interleaved i={i} c={c}"), &a, &b);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 23..=24 — property sweeps.
// ---------------------------------------------------------------------------

#[test]
fn configs_row_23_randomized_property_sweep() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_BABE);
    for _ in 0..4096 {
        diff_char(rng.next_char());
    }
}

#[test]
fn configs_row_24_randomized_with_locale_perturbation() {
    let p = libs();
    let locales = ["C", "POSIX", "C.UTF-8", "en_US.UTF-8", "no_SUCH.locale-42"];
    let mut rng = Rng::new(0x0102_0304_0506_0708);
    for _ in 0..512 {
        let c = rng.next_char();
        let loc = locales[(rng.next_u64() % locales.len() as u64) as usize];
        set_locale(loc);
        let a = capture_to_file(|| p.c.call(c));
        set_locale(loc);
        let b = capture_to_file(|| p.rs.call(c));
        assert_same(&format!("loc={loc} c={c}"), &a, &b);
    }
    set_locale("C");
}
