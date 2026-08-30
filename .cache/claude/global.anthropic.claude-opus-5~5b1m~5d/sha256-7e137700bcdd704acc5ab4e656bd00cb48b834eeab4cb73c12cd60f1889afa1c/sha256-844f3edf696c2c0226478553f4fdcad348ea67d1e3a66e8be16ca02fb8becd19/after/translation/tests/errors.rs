//! Phase C — error-path differential tests, gated on `ERRORS.md`.
//!
//! One test (or one clearly-labelled block) per row of `ERRORS.md` rows 1–14.
//! Rows 15–17 (the writable `.data` globals) live in `globals.rs`, which needs a
//! process of its own because it mutates process-global state. Rows 18–21 are
//! process-level and live in `driver_cli.rs`; rows 22–25 are build-time and are
//! discharged by `check_all_features.sh`.
//!
//! Both libraries are reached only through `dlopen`/`dlsym`, so the assertions
//! cover the `#[no_mangle]` wrappers and the `.data` placement of the globals.

mod common;

use std::ffi::c_int;

use common::{load_pair, same, Rng, INIT_FOR, OP_NAME, SEED};

/// `ERRORS.md` rows 1–5: the exact boundary values around the `DISPATCH_REP`
/// `switch`. `n` is an enum-like selector crossing the FFI boundary as a plain
/// `int`, so every value with no matching `case` is a real input the C handles
/// via `default: break;` — it must return `INIT_FOR(OP)`, not garbage and not a
/// panic.
#[test]
fn row_1_to_5_use_generated_out_of_range_selectors() {
    let (c, r) = load_pair();

    // (row, n, description)
    let cases: &[(u32, c_int, &str)] = &[
        (1, 7, "one past the last `case 6` (REP7 exists but is not in the switch)"),
        (2, -1, "one below the first `case 0`"),
        (3, i32::MIN, "extreme negative"),
        (4, i32::MAX, "extreme positive"),
        (5, 8, "first value beyond any defined REPn"),
    ];

    for &(row, n, why) in cases {
        // SAFETY: `int use_generated(int)`; `int` accepts every bit pattern.
        let (cv, rv) = unsafe { ((c.use_generated)(n), (r.use_generated)(n)) };
        same("use_generated", &format!("{n}"), cv, rv);
        assert_eq!(
            cv, INIT_FOR,
            "ERRORS.md row {row}: use_generated({n}) [{why}] must take `default:` \
             and return INIT_FOR({OP_NAME}) == {INIT_FOR}, got C={cv}"
        );
        assert_eq!(
            rv, INIT_FOR,
            "ERRORS.md row {row}: Rust use_generated({n}) [{why}] must return \
             INIT_FOR({OP_NAME}) == {INIT_FOR}, got {rv}"
        );
    }
}

/// `ERRORS.md` row 6: the whole negative half of `int` must take `default:`.
/// Randomised (fixed seed) rather than a single hand-picked negative, plus the
/// dense window just below zero where an off-by-one would hide.
#[test]
fn row_6_use_generated_all_negative_selectors_rejected() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 0x6666);
    let mut checked = 0usize;

    for _ in 0..2000 {
        // Map any u32 into [i32::MIN, -1].
        let n = (rng.next_i32() as i64).abs().wrapping_neg().wrapping_sub(1) as i32;
        let n = if n >= 0 { i32::MIN } else { n };
        assert!(n < 0);
        check_default_arm(&c, &r, n, 6);
        checked += 1;
    }
    for n in i32::MIN..i32::MIN + 32 {
        check_default_arm(&c, &r, n, 6);
        checked += 1;
    }
    for n in -64..0 {
        check_default_arm(&c, &r, n, 6);
        checked += 1;
    }
    assert!(checked >= 2000);
}

/// `ERRORS.md` row 7: the whole out-of-range positive tail `[7, INT_MAX]` must
/// take `default:`.
#[test]
fn row_7_use_generated_all_high_selectors_rejected() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 0x7777);

    for _ in 0..2000 {
        // Map any u32 into [7, i32::MAX].
        let raw = (rng.next_i32() as u32) % (i32::MAX as u32 - 6);
        let n = 7i32.wrapping_add(raw as i32);
        assert!(n >= 7, "n={n}");
        check_default_arm(&c, &r, n, 7);
    }
    for n in 7..128 {
        check_default_arm(&c, &r, n, 7);
    }
    for n in i32::MAX - 32..=i32::MAX {
        check_default_arm(&c, &r, n, 7);
    }
}

#[track_caller]
fn check_default_arm(c: &common::Api, r: &common::Api, n: c_int, row: u32) {
    // SAFETY: `int use_generated(int)`.
    let (cv, rv) = unsafe { ((c.use_generated)(n), (r.use_generated)(n)) };
    same("use_generated", &format!("{n}"), cv, rv);
    assert_eq!(
        cv, INIT_FOR,
        "ERRORS.md row {row}: C use_generated({n}) should hit `default:`"
    );
    assert_eq!(
        rv, INIT_FOR,
        "ERRORS.md row {row}: Rust use_generated({n}) should hit `default:`"
    );
}

/// `ERRORS.md` row 8: a 64-bit value whose low 32 bits *are* in range must be
/// truncated by the ABI **before** the `switch`, so it selects a valid `case`
/// rather than being rejected. This is the mirror-image trap of rows 1–7: over-
/// eager range checking in Rust would wrongly reject it.
#[test]
fn row_8_use_generated_selector_is_truncated_to_int_by_the_abi() {
    let (c, r) = load_pair();

    // Call through a signature that passes a 64-bit argument in the same
    // register the callee reads as a 32-bit `int`, so the high half is ignored
    // exactly as the C ABI specifies.
    for low in 0..=6i32 {
        let wide: i64 = 0x1_0000_0000i64 | (low as i64);
        // SAFETY: transmuting the resolved `int(*)(int)` to `int(*)(long)` is
        // deliberate: on x86-64 SysV both pass the argument in `edi`/`rdi`, and
        // the callee, being a real C `int` function, reads only `edi`. This is
        // precisely how a mismatched caller would reach the library.
        let cf: unsafe extern "C" fn(i64) -> c_int =
            unsafe { std::mem::transmute(c.use_generated) };
        let rf: unsafe extern "C" fn(i64) -> c_int =
            unsafe { std::mem::transmute(r.use_generated) };
        // SAFETY: see above.
        let (cv, rv) = unsafe { (cf(wide), rf(wide)) };
        same("use_generated", &format!("0x{wide:x} (truncates to {low})"), cv, rv);

        // And it must equal the properly-typed in-range call, i.e. NOT rejected.
        // SAFETY: `int use_generated(int)`.
        let (cn, rn) = unsafe { ((c.use_generated)(low), (r.use_generated)(low)) };
        assert_eq!(cv, cn, "ERRORS.md row 8: C must truncate 0x{wide:x} to {low}");
        assert_eq!(rv, rn, "ERRORS.md row 8: Rust must truncate 0x{wide:x} to {low}");
    }
}

/// `ERRORS.md` rows 9–11: signed-overflow inputs to the three leaf ops.
///
/// Signed overflow is UB in C, but the emitted instruction wraps two's
/// complement; the Rust translation must reproduce the C `.so`'s bits exactly
/// rather than panicking (note the tests run in the dev profile, where Rust
/// overflow checks are ON — a non-`wrapping_*` translation would abort here).
#[test]
fn rows_9_to_11_leaf_op_overflow() {
    let (c, r) = load_pair();

    let add_cases: &[(c_int, c_int)] = &[
        (i32::MAX, 1),
        (1, i32::MAX),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
    ];
    let sub_cases: &[(c_int, c_int)] = &[
        (i32::MIN, 1),
        (i32::MAX, -1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (0, i32::MIN),
        (-1, i32::MAX),
    ];
    let mul_cases: &[(c_int, c_int)] = &[
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MIN, i32::MIN),
        (46341, 46341),
        (-46341, 46341),
        (65536, 65536),
        (i32::MAX, 2),
        (i32::MIN, 2),
    ];

    for &(a, b) in add_cases {
        // SAFETY: `int op_add(int, int)`.
        let (cv, rv) = unsafe { ((c.op_add)(a, b), (r.op_add)(a, b)) };
        same("op_add (row 9, overflow)", &format!("{a}, {b}"), cv, rv);
    }
    for &(a, b) in sub_cases {
        // SAFETY: `int op_sub(int, int)`.
        let (cv, rv) = unsafe { ((c.op_sub)(a, b), (r.op_sub)(a, b)) };
        same("op_sub (row 10, overflow)", &format!("{a}, {b}"), cv, rv);
    }
    for &(a, b) in mul_cases {
        // SAFETY: `int op_mul(int, int)`.
        let (cv, rv) = unsafe { ((c.op_mul)(a, b), (r.op_mul)(a, b)) };
        same("op_mul (row 11, overflow)", &format!("{a}, {b}"), cv, rv);
    }

    // Randomised full-range sweep: with uniform 32-bit inputs the multiply
    // overflows almost always, so this is the broad version of the above.
    let mut rng = Rng::new(SEED ^ 0x9999);
    for _ in 0..2000 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        let args = format!("{a}, {b}");
        // SAFETY: all three are `int f(int, int)`.
        unsafe {
            same("op_add (row 9)", &args, (c.op_add)(a, b), (r.op_add)(a, b));
            same("op_sub (row 10)", &args, (c.op_sub)(a, b), (r.op_sub)(a, b));
            same("op_mul (row 11)", &args, (c.op_mul)(a, b), (r.op_mul)(a, b));
        }
    }
}

/// `ERRORS.md` rows 12–13: overflow inside `helper_call`'s `return r + acc`,
/// and the `sub`/`mul` accumulator shapes (negative / factorial).
#[test]
fn rows_12_13_helper_call_sum_overflow() {
    let (c, r) = load_pair();

    // Inputs chosen so that `op(a,b)` lands at or next to INT_MAX / INT_MIN and
    // the subsequent `+ acc` therefore wraps for any non-zero acc.
    let cases: &[(c_int, c_int)] = &[
        (i32::MAX, 0),
        (0, i32::MAX),
        (i32::MIN, 0),
        (0, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX - 21, 0),  // add/REPEAT=7 acc==21 lands exactly on INT_MAX
        (i32::MAX - 20, 0),  // ... and one past it
        (i32::MIN + 21, 0),
        (i32::MIN + 20, 0),
        (1, 1),
        (-1, -1),
    ];
    for &(a, b) in cases {
        // SAFETY: `int helper_call(int, int)`.
        let (cv, rv) = unsafe { ((c.helper_call)(a, b), (r.helper_call)(a, b)) };
        same("helper_call (rows 12-13)", &format!("{a}, {b}"), cv, rv);
    }

    let mut rng = Rng::new(SEED ^ 0xAAAA);
    for _ in 0..2000 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        // SAFETY: as above.
        let (cv, rv) = unsafe { ((c.helper_call)(a, b), (r.helper_call)(a, b)) };
        same("helper_call (rows 12-13)", &format!("{a}, {b}"), cv, rv);
    }
}

/// `ERRORS.md` row 14: `helper_ptr` has no null-function-pointer path, because
/// `fp` is initialised from the compile-time constant `OP_FN(OP)`. Overflowing
/// inputs must still wrap identically.
#[test]
fn row_14_helper_ptr_has_no_null_fp_path_and_wraps() {
    let (c, r) = load_pair();
    let cases: &[(c_int, c_int)] = &[
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (46341, 46341),
        (0, 0),
    ];
    for &(a, b) in cases {
        // SAFETY: `int helper_ptr(int, int)`.
        let (cv, rv) = unsafe { ((c.helper_ptr)(a, b), (r.helper_ptr)(a, b)) };
        same("helper_ptr (row 14)", &format!("{a}, {b}"), cv, rv);
    }
    let mut rng = Rng::new(SEED ^ 0xBBBB);
    for _ in 0..2000 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        // SAFETY: as above.
        let (cv, rv) = unsafe { ((c.helper_ptr)(a, b), (r.helper_ptr)(a, b)) };
        same("helper_ptr (row 14)", &format!("{a}, {b}"), cv, rv);
    }
}

/// Generic FFI boundary sweep required by Phase C even though it is not an
/// `ERRORS.md` row: *every* `int` bit pattern is a legal argument to every
/// exported function (there are no pointer parameters anywhere in the library,
/// so there is no null-pointer or length argument to abuse). This drives all six
/// exported functions over an exhaustive set of structurally interesting bit
/// patterns and asserts total agreement — no function may reject, trap, or
/// diverge on any of them.
#[test]
fn every_int_bit_pattern_is_accepted_identically() {
    let (c, r) = load_pair();

    // Powers of two, their negations and neighbours, plus all-ones patterns:
    // the bit shapes that expose sign-extension and width mistakes.
    let mut vals: Vec<c_int> = Vec::new();
    for bit in 0..32u32 {
        let v = 1i64 << bit;
        for cand in [v - 1, v, v + 1, -v - 1, -v, -v + 1] {
            vals.push(cand as i32);
        }
    }
    vals.extend_from_slice(&[0, -1, i32::MIN, i32::MAX, 0x5555_5555, -0x5555_5556]);
    vals.sort_unstable();
    vals.dedup();

    for &n in &vals {
        // SAFETY: `int use_generated(int)`.
        let (cv, rv) = unsafe { ((c.use_generated)(n), (r.use_generated)(n)) };
        same("use_generated", &format!("{n}"), cv, rv);
    }
    for &a in &vals {
        for &b in [0i32, 1, -1, i32::MIN, i32::MAX, a].iter() {
            let args = format!("{a}, {b}");
            // SAFETY: all are `int f(int, int)`.
            unsafe {
                same("op_add", &args, (c.op_add)(a, b), (r.op_add)(a, b));
                same("op_sub", &args, (c.op_sub)(a, b), (r.op_sub)(a, b));
                same("op_mul", &args, (c.op_mul)(a, b), (r.op_mul)(a, b));
                same("helper_call", &args, (c.helper_call)(a, b), (r.helper_call)(a, b));
                same("helper_ptr", &args, (c.helper_ptr)(a, b), (r.helper_ptr)(a, b));
            }
            same("G_OP", &args, c.call_g_op(a, b), r.call_g_op(a, b));
        }
    }
}
