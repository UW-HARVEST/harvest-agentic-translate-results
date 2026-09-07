//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundary rows G1-G5.
//! Every test constructs the exact invalid input/condition, calls BOTH the C
//! `.so` and the Rust `.so` through `libloading`, and asserts they return the
//! SAME sentinel (`-1`, `-2`, `0`, ...) — not merely "both failed".

mod common;
use common::{Pair, Rng};

fn pair() -> Pair {
    Pair::load()
}

// ===========================================================================
// Row 1 — find_entry returns NULL (no id matches) -> dataentry case 1 == -2
// ===========================================================================
#[test]
fn err01_find_entry_null_no_match() {
    let p = pair();
    let mut rng = Rng::new(0x1001);
    for count in 1..=12i32 {
        // every param2 below 0 and at/above count misses -> -2
        for b in -(count + 4)..0 {
            let v = p.assert_same(1, count, b, 0);
            assert_eq!(v, -2, "count={count} param2={b} must miss -> -2");
        }
        for b in count..(count + 8) {
            let v = p.assert_same(1, count, b, 0);
            assert_eq!(v, -2, "count={count} param2={b} must miss -> -2");
        }
    }
    for _ in 0..20_000 {
        let count = rng.range_i32(1, 16);
        let b = rng.next_i32();
        let v = p.assert_same(1, count, b, 0);
        if !(0..count).contains(&b) {
            assert_eq!(v, -2, "out-of-range param2={b} (count={count}) -> -2");
        }
    }
}

// ===========================================================================
// Row 2 — find_entry with count <= 0: `end <= ptr`, loop never runs -> NULL.
// Unreachable through `dataentry` (count = param1 > 0 ? param1 : 5|3), so the
// row is verified by proving the ternary really clamps: any param1 <= 0 must
// behave EXACTLY like param1 == 5 (case 1) / param1 == 3 (case 2) in both libs.
// ===========================================================================
#[test]
fn err02_find_entry_nonpositive_count_is_unreachable() {
    let p = pair();
    let mut rng = Rng::new(0x1002);
    for _ in 0..5_000 {
        let a = rng.range_i32(i32::MIN, 0);
        let b = rng.spicy_i32();
        let d = rng.spicy_i32();
        let clamped1 = p.assert_same(1, 5, b, d);
        let got1 = p.assert_same(1, a, b, d);
        assert_eq!(got1, clamped1, "case 1: param1={a} must act as count 5");
        let clamped2 = p.assert_same(2, 3, b, d);
        let got2 = p.assert_same(2, a, b, d);
        assert_eq!(got2, clamped2, "case 2: param1={a} must act as count 3");
    }
}

// ===========================================================================
// Rows 3, 4, 5, 26, 27 — process_name's rejection paths.
// `dataentry`'s default arm always passes a non-NULL dest holding "Default",
// so `dest == NULL` / `*dest == '\0'` can never fire and the -1 can never be
// observed; the result is always strlen("TestName") * param1 == 8 * param1.
// Both libraries must agree on that, including the wrap.
// ===========================================================================
#[test]
fn err03_04_05_26_27_process_name_rejections_unobservable() {
    let p = pair();
    let mut rng = Rng::new(0x1003);
    for _ in 0..20_000 {
        let m = rng.next_i32();
        if (1..=3).contains(&m) {
            continue;
        }
        let a = rng.spicy_i32();
        let v = p.assert_same(m, a, rng.spicy_i32(), rng.spicy_i32());
        assert_eq!(
            v,
            a.wrapping_mul(8),
            "default arm: expected 8*param1 (never -1); mode={m} param1={a}"
        );
        assert_ne!(
            (v, a),
            (-1, a),
            "process_name's -1 must never surface unless 8*param1 == -1"
        );
    }
    // Explicit extremes.
    for a in [0, 1, -1, i32::MAX, i32::MIN, 0x2000_0000, -0x2000_0000] {
        let v = p.assert_same(0, a, 0, 0);
        assert_eq!(v, a.wrapping_mul(8));
    }
}

// ===========================================================================
// Rows 6, 25 — calculate_lookup returning 0 (a zero table cell) is impossible:
// every lookup_table entry is non-zero, so case 3 in range ALWAYS adds param3.
// ===========================================================================
#[test]
fn err06_25_calculate_lookup_zero_cell_unreachable() {
    let p = pair();
    let table = [
        [10, 20, 30],
        [40, 50, 60],
        [70, 80, 90],
        [100, 110, 120],
    ];
    let mut rng = Rng::new(0x1006);
    for row in 0..4i32 {
        for col in 0..3i32 {
            for _ in 0..500 {
                let d = rng.spicy_i32();
                let v = p.assert_same(3, row, col, d);
                let expect = (table[row as usize][col as usize] * 2i32).wrapping_add(d);
                assert_eq!(
                    v, expect,
                    "case 3 must always take the `!= 0` branch: [{row}][{col}] d={d}"
                );
            }
        }
    }
}

// ===========================================================================
// Row 7 — calculate_lookup with an out-of-bounds row/col is guarded by the
// caller. Prove the guard: a mode-3 call with any out-of-range param1/param2
// returns 0 in BOTH libs and never touches the table.
// (Same conditions as rows 21-24, asserted here from the callee's viewpoint.)
// ===========================================================================
#[test]
fn err07_calculate_lookup_oob_is_guarded() {
    let p = pair();
    let mut rng = Rng::new(0x1007);
    for _ in 0..50_000 {
        let a = rng.spicy_i32();
        let b = rng.spicy_i32();
        let v = p.assert_same(3, a, b, rng.spicy_i32());
        if !(0..4).contains(&a) || !(0..3).contains(&b) {
            assert_eq!(v, 0, "mode 3 out-of-range ({a},{b}) must return 0");
        }
    }
}

// ===========================================================================
// Rows 8, 13, 18 + G5 — malloc failure.
//
// `create_entries` does `malloc(count * sizeof(DataEntry))` (40 bytes/entry)
// BEFORE checking anything. A huge `param1` makes malloc return NULL, which
// `dataentry` turns into -1 for case 1 and case 2 alike.
//
// This host has 371 GiB of RAM and heuristic overcommit, so an 80 GiB malloc
// would SUCCEED and the fill loop would then touch all 80 GiB. The test
// therefore re-executes itself inside `sh -c 'ulimit -v ...'` so that malloc
// genuinely fails for both libraries under identical conditions.
// ===========================================================================
const CHILD_ENV: &str = "HARVEST_MALLOC_FAIL_CHILD";

#[test]
fn err08_13_18_g5_malloc_failure_returns_minus_one() {
    if std::env::var_os(CHILD_ENV).is_some() {
        malloc_failure_body();
        return;
    }

    // Make sure both .so files exist *before* we clamp the address space,
    // because building them inside the limited child would fail.
    let p = pair();
    let c_so = p.c_path.clone();
    let rust_so = p.rust_path.clone();
    assert!(c_so.is_file() && rust_so.is_file());
    drop(p);

    let exe = std::env::current_exe().expect("current_exe");
    // 512 MiB of address space: plenty for the two libraries + the Rust
    // runtime, far too little for the multi-GiB allocations below.
    let script = format!(
        "ulimit -v 524288; exec {} --exact {} --nocapture --test-threads=1",
        shell_quote(&exe.to_string_lossy()),
        "err08_13_18_g5_malloc_failure_returns_minus_one"
    );
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .env(CHILD_ENV, "1")
        .output()
        .expect("spawn address-space-limited child");

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.success(),
        "address-space-limited child failed ({:?})\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        out.status
    );
    assert!(
        stdout.contains("MALLOC-FAILURE-CHECKS-DONE"),
        "child did not run the malloc-failure checks\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn malloc_failure_body() {
    let p = pair();
    // Every one of these needs >= 512 MiB (count * 40 bytes).
    let huge = [
        i32::MAX,
        i32::MAX - 1,
        2_000_000_000,
        1_000_000_000,
        1 << 30,
        1 << 28,
        1 << 26,
        100_000_000,
        50_000_000,
        20_000_000,
    ];
    let mut checked = 0usize;
    for &a in &huge {
        // Row 13 / row 8: case 1 -> create_entries(count, 100) == NULL -> -1
        let v1 = p.assert_same(1, a, 0, 0);
        // Row 18 / row 8: case 2 -> create_entries(count, 200) == NULL -> -1
        let v2 = p.assert_same(2, a, 1, 7);
        if v1 == -1 {
            checked += 1;
            assert_eq!(v2, -1, "case 2 must also report -1 for param1={a}");
        }
        // Whatever the outcome, both libraries must have agreed (assert_same).
        let _ = v2;
    }
    assert!(
        checked > 0,
        "no allocation actually failed under `ulimit -v`; the malloc-failure \
         path was not exercised"
    );
    // Also with random huge counts and random param2/param3.
    let mut rng = Rng::new(0x1008);
    for _ in 0..200 {
        let a = rng.range_i32(20_000_000, i32::MAX);
        p.assert_same(1, a, rng.spicy_i32(), rng.spicy_i32());
        p.assert_same(2, a, rng.spicy_i32(), rng.spicy_i32());
    }
    println!("MALLOC-FAILURE-CHECKS-DONE ({checked} sizes failed to allocate)");
}

// ===========================================================================
// Rows 9, 14 — `create_entries(count <= 0)` and `dataentry`'s `count == 0`
// guard. Both are unreachable because of the `param1 > 0 ? param1 : 5|3`
// clamp; the observable consequence is that `dataentry` NEVER returns the -1
// that those branches would produce for a small non-huge param1.
// ===========================================================================
#[test]
fn err09_14_zero_and_negative_count_never_yield_minus_one() {
    let p = pair();
    let mut rng = Rng::new(0x1009);
    for a in [0, -1, -2, -5, -10, -1_000, i32::MIN, i32::MIN + 1, -0x4000_0000] {
        for _ in 0..300 {
            let b = rng.spicy_i32();
            let d = rng.spicy_i32();
            let v1 = p.assert_same(1, a, b, d);
            assert_ne!(v1, -1, "case 1 param1={a}: count==0/NULL path must not fire");
            // case 2 with param1 <= 0 always allocates 3 entries successfully
            let _ = p.assert_same(2, a, b, d);
        }
    }
}

// ===========================================================================
// Rows 10, 11, 12 — modify_entries' rejection paths.
// `entries == NULL` (-1) is pre-empted by `dataentry`'s own NULL check;
// `count <= 0` is pre-empted by the ternary clamp; `value == 0` cannot happen
// because value = (200 + i) * 10 != 0 for i >= 0. Consequence: with a non-zero
// multiplier, EVERY entry is multiplied and summed.
// ===========================================================================
#[test]
fn err10_11_12_modify_entries_rejections_unreachable() {
    let p = pair();
    let mut rng = Rng::new(0x100A);
    for _ in 0..10_000 {
        let count = rng.range_i32(1, 24);
        let mult = {
            let m = rng.spicy_i32();
            if m == 0 { 1 } else { m }
        };
        let d = rng.spicy_i32();
        // Emulate: total = sum_i ((200+i)*10 * mult), all wrapping i32.
        let mut total: i32 = 0;
        for i in 0..count {
            let value = (200i32.wrapping_add(i)).wrapping_mul(10);
            assert_ne!(value, 0, "value must never be 0 -> no entry is skipped");
            total = total.wrapping_add(value.wrapping_mul(mult));
        }
        let expect = if total != 0 { total.wrapping_add(d) } else { 0 };
        let v = p.assert_same(2, count, mult, d);
        assert_eq!(v, expect, "count={count} mult={mult} d={d}");
    }
}

// ===========================================================================
// Rows 15, 17 — find_entry misses. Row 17 makes `100 + param2` overflow.
// ===========================================================================
#[test]
fn err15_17_find_entry_miss_and_overflowing_target() {
    let p = pair();
    // Row 17: 100 + INT_MAX wraps to a negative id that cannot exist.
    for b in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 50,
        i32::MAX - 99,
        i32::MAX - 100,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 99,
        i32::MIN + 100,
    ] {
        for count in 1..=10i32 {
            let v = p.assert_same(1, count, b, 0);
            assert_eq!(v, -2, "overflowing target 100+{b} (count={count}) -> -2");
        }
    }
    // Row 15: plain out-of-range misses, randomized.
    let mut rng = Rng::new(0x100F);
    for _ in 0..20_000 {
        let count = rng.range_i32(1, 20);
        let b = if rng.next_u64() % 2 == 0 {
            rng.range_i32(count, count + 10_000)
        } else {
            rng.range_i32(-10_000, -1)
        };
        let v = p.assert_same(1, count, b, rng.spicy_i32());
        assert_eq!(v, -2);
    }
}

// ===========================================================================
// Row 16 — `found->id == 0` is unreachable: ids are 100..100+count-1.
// Consequence: whenever find_entry hits, the result is the entry's value, and
// -2 is returned ONLY on a miss.
// ===========================================================================
#[test]
fn err16_found_id_zero_unreachable() {
    let p = pair();
    let mut rng = Rng::new(0x1010);
    for _ in 0..20_000 {
        let count = rng.range_i32(1, 32);
        let b = rng.range_i32(0, count - 1);
        let v = p.assert_same(1, count, b, rng.spicy_i32());
        assert_eq!(
            v,
            (100i32.wrapping_add(b)).wrapping_mul(10),
            "a hit must return value, never -2 (count={count} b={b})"
        );
    }
}

// ===========================================================================
// Rows 19, 20 — modify_entries total == 0 suppresses `result += param3`.
// Row 19: multiplier 0. Row 20: a total that WRAPS to exactly 0.
// ===========================================================================
#[test]
fn err19_zero_total_via_zero_multiplier() {
    let p = pair();
    let mut rng = Rng::new(0x1013);
    for count in 1..=40i32 {
        for _ in 0..50 {
            let d = rng.spicy_i32();
            let v = p.assert_same(2, count, 0, d);
            assert_eq!(v, 0, "multiplier 0 -> total 0 -> param3 ({d}) NOT added");
        }
    }
}

#[test]
fn err20_zero_total_via_wraparound() {
    // Find (count, multiplier) pairs whose wrapping total is exactly 0 while
    // every individual pre-multiply value is non-zero.
    let mut found = Vec::new();
    'outer: for count in 1..=64i32 {
        let mut s: i32 = 0;
        for i in 0..count {
            s = s.wrapping_add((200i32.wrapping_add(i)).wrapping_mul(10));
        }
        // s * mult == 0 (mod 2^32)  <=>  mult is a multiple of 2^(32 - tz(s))
        let tz = (s as u32).trailing_zeros();
        if tz >= 32 {
            continue;
        }
        let shift = 32 - tz;
        if shift >= 32 {
            continue;
        }
        let mult = (1u32 << shift) as i32;
        if mult == 0 {
            continue;
        }
        let mut total: i32 = 0;
        for i in 0..count {
            total = total
                .wrapping_add(((200i32.wrapping_add(i)).wrapping_mul(10)).wrapping_mul(mult));
        }
        if total == 0 {
            found.push((count, mult));
            if found.len() >= 12 {
                break 'outer;
            }
        }
    }
    assert!(
        !found.is_empty(),
        "could not construct a wrapping-to-zero total"
    );

    let p = pair();
    let mut rng = Rng::new(0x1014);
    for (count, mult) in found {
        for d in [0, 1, -1, 42, i32::MAX, i32::MIN, rng.next_i32()] {
            let v = p.assert_same(2, count, mult, d);
            assert_eq!(
                v, 0,
                "wrapping total 0 (count={count} mult={mult}) must suppress param3={d}"
            );
        }
    }
}

// ===========================================================================
// Rows 21, 22, 23, 24 — mode 3's four explicit range rejections.
// ===========================================================================
#[test]
fn err21_mode3_param1_negative() {
    let p = pair();
    let mut rng = Rng::new(0x1015);
    for a in [-1, -2, -4, -100, i32::MIN, i32::MIN + 1, -0x4000_0000] {
        for col in -2..5i32 {
            let v = p.assert_same(3, a, col, rng.spicy_i32());
            assert_eq!(v, 0, "mode 3 param1={a} < 0 -> 0");
        }
    }
    for _ in 0..10_000 {
        let a = rng.range_i32(i32::MIN, -1);
        let v = p.assert_same(3, a, rng.range_i32(0, 2), rng.spicy_i32());
        assert_eq!(v, 0);
    }
}

#[test]
fn err22_mode3_param1_at_or_past_row_limit() {
    let p = pair();
    let mut rng = Rng::new(0x1016);
    for a in [4, 5, 6, 100, i32::MAX, i32::MAX - 1, 0x4000_0000i64 as i32] {
        for col in 0..3i32 {
            let v = p.assert_same(3, a, col, rng.spicy_i32());
            assert_eq!(v, 0, "mode 3 param1={a} >= 4 -> 0");
        }
    }
    for _ in 0..10_000 {
        let a = rng.range_i32(4, i32::MAX);
        let v = p.assert_same(3, a, rng.range_i32(0, 2), rng.spicy_i32());
        assert_eq!(v, 0);
    }
}

#[test]
fn err23_mode3_param2_negative() {
    let p = pair();
    let mut rng = Rng::new(0x1017);
    for b in [-1, -2, -3, -100, i32::MIN, i32::MIN + 1] {
        for row in 0..4i32 {
            let v = p.assert_same(3, row, b, rng.spicy_i32());
            assert_eq!(v, 0, "mode 3 param2={b} < 0 -> 0");
        }
    }
    for _ in 0..10_000 {
        let b = rng.range_i32(i32::MIN, -1);
        let v = p.assert_same(3, rng.range_i32(0, 3), b, rng.spicy_i32());
        assert_eq!(v, 0);
    }
}

#[test]
fn err24_mode3_param2_at_or_past_col_limit() {
    let p = pair();
    let mut rng = Rng::new(0x1018);
    for b in [3, 4, 5, 100, i32::MAX, i32::MAX - 1] {
        for row in 0..4i32 {
            let v = p.assert_same(3, row, b, rng.spicy_i32());
            assert_eq!(v, 0, "mode 3 param2={b} >= 3 -> 0");
        }
    }
    for _ in 0..10_000 {
        let b = rng.range_i32(3, i32::MAX);
        let v = p.assert_same(3, rng.range_i32(0, 3), b, rng.spicy_i32());
        assert_eq!(v, 0);
    }
}

// ===========================================================================
// Row 28 + G4 — `switch (mode)` on an int: every value that is not 1/2/3 must
// take `default:` identically, including "out-of-range enum" values.
// ===========================================================================
#[test]
fn err28_g4_out_of_range_mode_enum_values() {
    let p = pair();
    let mut rng = Rng::new(0x1019);
    let explicit = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -100,
        -4,
        -3,
        -2,
        -1,
        0,
        4,
        5,
        6,
        7,
        8,
        255,
        256,
        65_535,
        65_536,
        1_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &m in &explicit {
        for a in [0, 1, -1, 7, i32::MAX, i32::MIN] {
            let v = p.assert_same(m, a, rng.spicy_i32(), rng.spicy_i32());
            assert_eq!(
                v,
                a.wrapping_mul(8),
                "mode={m} must fall through to default: (8*param1)"
            );
        }
    }
    for _ in 0..20_000 {
        let m = rng.next_i32();
        if (1..=3).contains(&m) {
            continue;
        }
        let a = rng.next_i32();
        let v = p.assert_same(m, a, rng.spicy_i32(), rng.spicy_i32());
        assert_eq!(v, a.wrapping_mul(8));
    }
}

// ===========================================================================
// G1 — all-zero arguments.
// ===========================================================================
#[test]
fn g1_all_zero_arguments() {
    let p = pair();
    let v = p.assert_same(0, 0, 0, 0);
    assert_eq!(v, 0, "8 * 0");
    for m in 0..=4i32 {
        p.assert_same(m, 0, 0, 0);
    }
}

// ===========================================================================
// G2 — INT_MIN / INT_MAX for every parameter, one at a time and combined.
// (`param1` for modes 1/2 is exercised in the address-space-limited test; here
// we sweep it too, but only with values whose allocation is either tiny or
// certain to be rejected/agreed upon by both libraries.)
// ===========================================================================
#[test]
fn g2_extremes_per_parameter() {
    let p = pair();
    let ext = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    // Modes 3 and default: every parameter can safely take any extreme.
    for &m in &[3i32, 0, 4, -1, i32::MIN, i32::MAX] {
        for &a in &ext {
            for &b in &ext {
                for &d in &ext {
                    p.assert_same(m, a, b, d);
                }
            }
        }
    }
    // Modes 1 and 2: param1 clamped to a safe magnitude (huge counts are
    // covered by err08_13_18_g5), param2/param3 fully extreme.
    for &m in &[1i32, 2] {
        for &a in &[i32::MIN, i32::MIN + 1, -1, 0, 1, 2, 7] {
            for &b in &ext {
                for &d in &ext {
                    p.assert_same(m, a, b, d);
                }
            }
        }
    }
}

// ===========================================================================
// G3 — one step past every documented valid range.
// ===========================================================================
#[test]
fn g3_one_past_each_range() {
    let p = pair();
    // mode: 0 (one before case 1) and 4 (one past case 3)
    for m in [0, 4] {
        p.assert_same(m, 1, 0, 0);
    }
    // mode 3: row -1 / 4, col -1 / 3
    for (a, b) in [(-1, 0), (4, 0), (0, -1), (0, 3), (-1, -1), (4, 3), (3, 3), (4, 2)] {
        let v = p.assert_same(3, a, b, 12_345);
        assert_eq!(v, 0, "mode 3 one-past ({a},{b})");
    }
    // mode 3: last valid cell must NOT be rejected
    let v = p.assert_same(3, 3, 2, 0);
    assert_eq!(v, 240);
    // mode 1: param2 == -1 and param2 == count for each count
    for count in 1..=16i32 {
        assert_eq!(p.assert_same(1, count, -1, 0), -2);
        assert_eq!(p.assert_same(1, count, count, 0), -2);
        assert_eq!(
            p.assert_same(1, count, count - 1, 0),
            (100i32 + count - 1) * 10,
            "last valid index must be a hit"
        );
    }
    // mode 1/2: param1 == 0 (one below the `> 0` ternary) and == 1
    for m in [1, 2] {
        p.assert_same(m, 0, 0, 0);
        p.assert_same(m, 1, 0, 0);
        p.assert_same(m, -1, 0, 0);
    }
}
