//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! `c_src/` contains no `assert`, no error enum, no sentinel return and no
//! pointer/length parameter, so the rejection surface is small and unusual: the
//! only *reported* error in the whole project is `main`'s `argc < 3`, and the
//! only *silent* rejection is `DISPATCH_REP`'s `default: break;`. Both are
//! covered here, together with the generic C boundaries (overflow, values one
//! step past a documented range, `atoi` garbage) and an explicit assertion of
//! the facts that make the remaining generic boundaries inapplicable.

mod common;

use std::ffi::c_int;

use common::*;

/* ================= rows 1-2: main's argc < 3 ================= */

/// Row 1 — no operands at all.
#[test]
fn e01_argc_zero_operands() {
    diff_driver("./driver", &[]);

    // And the exact C contract, independently of the Rust side.
    let out = std::process::Command::new(c_driver_path()).output().unwrap();
    assert_eq!(out.status.code(), Some(2), "C exit status for argc<3");
    assert!(out.stdout.is_empty(), "C wrote to stdout on the usage path");
    assert!(
        String::from_utf8_lossy(&out.stderr).ends_with(" A B\n"),
        "C usage line: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Row 2 — exactly one operand.
#[test]
fn e02_argc_one_operand() {
    for a in ["7", "-7", "abc", "", "0"] {
        diff_driver("./driver", &[a]);
    }
    // argv[0] is interpolated into the message, so vary it.
    for arg0 in ["./driver", "driver", "/usr/local/bin/driver", "", "a b c", "üñî"] {
        diff_driver(arg0, &[]);
        diff_driver(arg0, &["7"]);
    }
}

/* ========== rows 3-5: DISPATCH_REP's `default: break;` ========== */

/// Row 3 — `n == 7`, the first value past the last `case`.
#[test]
fn e03_use_generated_seven() {
    let (c, r) = (c_impl(), rust_impl());
    let (cf, rf) = (c.unop("use_generated"), r.unop("use_generated"));
    let (cv, rv) = unsafe { (cf(7), rf(7)) };
    assert_eq!(cv, rv, "[{}] use_generated(7): C={cv} Rust={rv}", config_label());
    // `default: break;` leaves `acc` at `INIT_<OP>`.
    assert_eq!(cv, INIT, "[{}] C use_generated(7) should be INIT_{OP}={INIT}", config_label());

    // `case 6` and `n == 7` must be different unless REP6 is a no-op
    // (which only happens when the six steps cancel -- they do not for
    // add/sub/mul), so this also proves the default branch was taken.
    let six = unsafe { cf(6) };
    assert_ne!(six, cv, "[{}] case 6 and default indistinguishable", config_label());
}

/// Row 4 — negative `n`.
#[test]
fn e04_use_generated_negative() {
    let (c, r) = (c_impl(), rust_impl());
    let (cf, rf) = (c.unop("use_generated"), r.unop("use_generated"));
    for n in [-1, -2, -6, -7, -8, -100, c_int::MIN, c_int::MIN + 1] {
        let (cv, rv) = unsafe { (cf(n), rf(n)) };
        assert_eq!(cv, rv, "[{}] use_generated({n}): C={cv} Rust={rv}", config_label());
        assert_eq!(cv, INIT, "[{}] C use_generated({n}) should be INIT_{OP}", config_label());
    }
}

/// Row 5 — `n` far above the range.
#[test]
fn e05_use_generated_oversized() {
    let (c, r) = (c_impl(), rust_impl());
    let (cf, rf) = (c.unop("use_generated"), r.unop("use_generated"));
    for n in [8, 9, 10, 100, 255, 256, 1 << 16, 1 << 20, c_int::MAX - 1, c_int::MAX] {
        let (cv, rv) = unsafe { (cf(n), rf(n)) };
        assert_eq!(cv, rv, "[{}] use_generated({n}): C={cv} Rust={rv}", config_label());
        assert_eq!(cv, INIT, "[{}] C use_generated({n}) should be INIT_{OP}", config_label());
    }
}

/* ============ rows 6-8: signed-int overflow in the leaf ops ============ */

fn overflow_row(sym: &str, cases: &[(c_int, c_int)], model: fn(c_int, c_int) -> c_int) {
    let (c, r) = (c_impl(), rust_impl());
    let (cf, rf) = (c.binop(sym), r.binop(sym));
    for &(a, b) in cases {
        let (cv, rv) = unsafe { (cf(a, b), rf(a, b)) };
        assert_eq!(cv, rv, "[{}] {sym}({a}, {b}): C={cv} Rust={rv}", config_label());
        assert_eq!(cv, model(a, b), "[{}] C {sym}({a}, {b}) did not wrap", config_label());
    }
}

/// Row 6 — `op_add` overflow.
#[test]
fn e06_op_add_overflow() {
    overflow_row(
        "op_add",
        &[
            (c_int::MAX, 1),
            (1, c_int::MAX),
            (c_int::MIN, -1),
            (-1, c_int::MIN),
            (c_int::MAX, c_int::MAX),
            (c_int::MIN, c_int::MIN),
            (c_int::MAX, 2),
            (c_int::MIN, -2),
            (c_int::MAX / 2 + 1, c_int::MAX / 2 + 1),
        ],
        |a, b| a.wrapping_add(b),
    );
}

/// Row 7 — `op_sub` overflow.
#[test]
fn e07_op_sub_overflow() {
    overflow_row(
        "op_sub",
        &[
            (c_int::MIN, 1),
            (c_int::MAX, -1),
            (c_int::MIN, c_int::MAX),
            (c_int::MAX, c_int::MIN),
            (0, c_int::MIN),
            (-1, c_int::MAX),
            (c_int::MIN, 2),
        ],
        |a, b| a.wrapping_sub(b),
    );
}

/// Row 8 — `op_mul` overflow.
#[test]
fn e08_op_mul_overflow() {
    overflow_row(
        "op_mul",
        &[
            (c_int::MAX, 2),
            (2, c_int::MAX),
            (c_int::MIN, -1),
            (-1, c_int::MIN),
            (65536, 65536),
            (-65536, 65536),
            (46341, 46341),
            (c_int::MAX, c_int::MAX),
            (c_int::MIN, c_int::MIN),
            (c_int::MIN, 2),
            (3, c_int::MAX / 2),
        ],
        |a, b| a.wrapping_mul(b),
    );
}

/// Row 9 — overflow inside the unrolled accumulator and in `r + acc`.
#[test]
fn e09_accumulator_overflow() {
    let (c, r) = (c_impl(), rust_impl());
    let (c_hc, r_hc) = (c.binop("helper_call"), r.binop("helper_call"));

    // Operand pairs whose op result sits at the very top/bottom of the range, so
    // adding the accumulator wraps.
    let mut cases: Vec<(c_int, c_int)> = BOUNDARY_PAIRS.to_vec();
    for k in 0..8 {
        cases.push((c_int::MAX - k, 0));
        cases.push((c_int::MIN + k, 0));
        cases.push((0, c_int::MAX - k));
        cases.push((0, c_int::MIN + k));
    }
    for (a, b) in cases {
        let (cv, rv) = unsafe { (c_hc(a, b), r_hc(a, b)) };
        assert_eq!(cv, rv, "[{}] helper_call({a}, {b}) wrap: C={cv} Rust={rv}", config_label());
    }

    // The accumulator itself, recovered from both libraries.
    let op_sym = format!("op_{OP}");
    let (c_op, r_op) = (c.binop(&op_sym), r.binop(&op_sym));
    let c_acc = unsafe { c_hc(0, 0).wrapping_sub(c_op(0, 0)) };
    let r_acc = unsafe { r_hc(0, 0).wrapping_sub(r_op(0, 0)) };
    assert_eq!(c_acc, r_acc, "[{}] accumulator differs", config_label());

    // And `use_generated` over the whole `int` range never diverges.
    diff_unop("use_generated", random_ints(SEED ^ 0xBB, 512));
}

/* ===== rows 10-11: the generic boundaries, and why some don't exist ===== */

/// Row 10 — there is no `enum` in the C API; the closest analogue is an `int`
/// argument with no matching `switch case`. Assert that every exported function
/// accepts arbitrary 32-bit patterns (no argument validation anywhere) and that
/// both libraries agree on all of them, including the exact bit patterns a
/// bogus enum tag would produce.
#[test]
fn e10_no_enum_surface() {
    let bogus: Vec<c_int> = vec![
        -1,
        0,
        1,
        7,
        8,
        0x7F,
        0x80,
        0xFF,
        0x100,
        0xDEAD_BEEFu32 as c_int,
        0xFFFF_FFFFu32 as c_int,
        0x8000_0000u32 as c_int,
        0x7FFF_FFFF,
        i32::MIN,
        i32::MAX,
    ];

    // Unary export.
    diff_unop("use_generated", bogus.iter().copied());

    // Binary exports, over the cross-product of the bogus values.
    let mut pairs = Vec::new();
    for &a in &bogus {
        for &b in &bogus {
            pairs.push((a, b));
        }
    }
    for sym in ["op_add", "op_sub", "op_mul", "helper_ptr", "helper_call"] {
        diff_binop(sym, pairs.iter().copied());
    }
}

/// Row 11 — assert the ABI facts that make the null-pointer / zero-length /
/// oversized-length boundaries inapplicable: the exported surface is exactly six
/// `int`-only functions plus two 8-byte data objects, identical in both `.so`s.
/// (Nothing is stubbed to satisfy this; it is checked against `nm -D`.)
#[test]
fn e11_no_pointer_parameters() {
    let expected_fns = ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr", "use_generated"];
    let expected_objs = ["G_OP", "G_OP_NAME"];

    let c_syms = defined_dynamic_symbols(&c_so_path());
    let r_syms = defined_dynamic_symbols(&rust_so_path());

    for name in expected_fns.iter().chain(expected_objs.iter()) {
        assert!(c_syms.contains(*name), "C .so is missing {name}");
        assert!(r_syms.contains(*name), "Rust .so is missing {name}");
    }
    // The C export set contains nothing beyond the eight known symbols, so there
    // is no pointer-taking entry point that could need a null check.
    let mut extra: Vec<&String> = c_syms
        .iter()
        .filter(|s| !expected_fns.contains(&s.as_str()) && !expected_objs.contains(&s.as_str()))
        .collect();
    extra.sort();
    assert!(extra.is_empty(), "unexpected C exports (update ERRORS.md row 11): {extra:?}");

    // Both `dlsym` lookups succeed and the two data objects are non-NULL.
    let (c, r) = (c_impl(), rust_impl());
    for i in [c, r] {
        assert!(!i.g_op().is_null());
        assert!(!i.g_op_name().is_null());
        assert!(!unsafe { *i.g_op_name() }.is_null(), "{}: G_OP_NAME is NULL", i.name);
    }
}

/// `nm -D --defined-only` symbol names.
fn defined_dynamic_symbols(path: &std::path::Path) -> std::collections::BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/* ================= rows 12-13: atoi in main ================= */

/// Row 12 — operands that `atoi` cannot fully parse.
#[test]
fn e12_atoi_garbage() {
    // NOTE: a string containing an interior NUL cannot be passed as an `argv`
    // entry at all (`execve` takes NUL-terminated strings), so there is no such
    // input to compare.
    let garbage: &[&str] = &[
        "", " ", "\t", "\n", "abc", "x1", "-", "+", "--5", "++5", "+-5", "- 5", ".5", ".", "1.9",
        "1e3", "0x10", "0X10", "0b1", "12abc", "12 34", "12,34", "  \t\n 42 junk", "٣",
        "\u{feff}7", "2 ",
    ];
    for a in garbage {
        for b in ["3", "-3", "0", "abc"] {
            diff_driver("./driver", &[a, b]);
        }
    }
}

/// Row 13 — operands outside the `int` range (`atoi` == `(int)strtol`).
#[test]
fn e13_atoi_out_of_range() {
    let oor: &[&str] = &[
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551616",
        "99999999999999999999",
        "-99999999999999999999",
        "000000000000000000000000012",
        "+2147483648",
    ];
    for a in oor {
        for b in ["1", "-1", "0"] {
            diff_driver("./driver", &[a, b]);
        }
    }
    for b in oor {
        diff_driver("./driver", &["1", b]);
    }
}
