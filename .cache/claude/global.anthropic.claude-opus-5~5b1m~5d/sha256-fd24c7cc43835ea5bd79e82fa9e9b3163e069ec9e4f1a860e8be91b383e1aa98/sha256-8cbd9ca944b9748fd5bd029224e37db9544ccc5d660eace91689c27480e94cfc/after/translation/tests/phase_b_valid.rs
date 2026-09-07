//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through `dlopen`/`dlsym`
//! on both the C `.so` and the Rust `.so`; return values *and* the bytes each
//! library writes to `stdout` are compared byte-for-byte.

mod common;

use common::*;
use std::ffi::CString;

/// Randomized cases per row. Override with `CTORUST_N=20000 cargo test`.
fn n() -> usize {
    std::env::var("CTORUST_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(400)
}

// ---------------------------------------------------------------------------
// Row 1 — check_permissions with required == 0 (vacuously true)
// ---------------------------------------------------------------------------
#[test]
fn row01_check_permissions_required_zero() {
    let mut rng = Rng::new();
    let mut perms: Vec<i32> = vec![0, 1, -1, 0o400, 0o644, i32::MIN, i32::MAX];
    for _ in 0..n() {
        perms.push(rng.any_i32());
    }
    for p in perms {
        diff(&format!("check_permissions({p}, 0)"), |i| {
            check_permissions(i, p, 0)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 2 — single-bit `required` (READ/WRITE/EXEC) crossed with perms
// ---------------------------------------------------------------------------
#[test]
fn row02_check_permissions_single_bit() {
    let bits = [0o400, 0o200, 0o100];
    let mut rng = Rng::new();
    for req in bits {
        // deterministic: bit set / bit clear / only that bit
        for p in [0, req, !req, -1, 0o644, 0o600, 0o777, req | 0o1] {
            diff(&format!("check_permissions({p}, {req})"), |i| {
                check_permissions(i, p, req)
            });
        }
        for _ in 0..n() {
            let p = rng.any_i32();
            diff(&format!("check_permissions({p}, {req})"), |i| {
                check_permissions(i, p, req)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — multi-bit `required`
// ---------------------------------------------------------------------------
#[test]
fn row03_check_permissions_multi_bit() {
    let reqs = [0o600, 0o644, 0o777, 0o700, 0o066];
    let mut rng = Rng::new();
    for req in reqs {
        for p in [0, req, req & !0o200, req | 0o111, -1, 0o644, 0o444, 0o222] {
            diff(&format!("check_permissions({p}, {req})"), |i| {
                check_permissions(i, p, req)
            });
        }
        for _ in 0..n() {
            // Bias towards perms that share bits with `req` so both outcomes occur.
            let p = (rng.any_i32() & req) | (rng.any_i32() & !req & 0o7777);
            diff(&format!("check_permissions({p}, {req})"), |i| {
                check_permissions(i, p, req)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — full random 32-bit perms x required, incl. negatives
// ---------------------------------------------------------------------------
#[test]
fn row04_check_permissions_random_full_range() {
    let mut rng = Rng::with_seed(SEED ^ 4);
    for a in INTERESTING_INTS {
        for b in INTERESTING_INTS {
            diff(&format!("check_permissions({a}, {b})"), |i| {
                check_permissions(i, a, b)
            });
        }
    }
    for _ in 0..(n() * 4) {
        let p = rng.any_i32();
        let r = rng.any_i32();
        diff(&format!("check_permissions({p}, {r})"), |i| {
            check_permissions(i, p, r)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 5 — safe_add, permissions granting 0600
// ---------------------------------------------------------------------------
#[test]
fn row05_safe_add_permitted() {
    let mut rng = Rng::with_seed(SEED ^ 5);
    let granting = [0o600, 0o644, 0o777, -1, 0o600 | 0o1, i32::MIN | 0o600];
    for perms in granting {
        for _ in 0..n() {
            let a = rng.small();
            let b = rng.small();
            diff(&format!("safe_add({a}, {b}, {perms})"), |i| {
                safe_add(i, a, b, perms)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — safe_add overflow with permission granted
// ---------------------------------------------------------------------------
#[test]
fn row06_safe_add_overflow() {
    let mut rng = Rng::with_seed(SEED ^ 6);
    let pairs = [
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (i32::MAX, -1),
        (i32::MIN, 1),
        (1 << 30, 1 << 30),
        (-(1 << 30), -(1 << 30)),
    ];
    for (a, b) in pairs {
        diff(&format!("safe_add({a}, {b}, 0644)"), |i| {
            safe_add(i, a, b, 0o644)
        });
    }
    for _ in 0..(n() * 2) {
        let a = rng.any_i32();
        let b = rng.any_i32();
        diff(&format!("safe_add({a}, {b}, 0644)"), |i| {
            safe_add(i, a, b, 0o644)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 7 — safe_add rejection paths (also ERRORS row 2)
// ---------------------------------------------------------------------------
#[test]
fn row07_safe_add_rejected() {
    let mut rng = Rng::with_seed(SEED ^ 7);
    // 0600 == READ|WRITE. Anything missing either bit must be rejected.
    let denying = [
        0,
        0o400,          // read only
        0o200,          // write only
        0o100,          // exec only
        0o444,          // read for all, no write
        0o222,          // write for all, no read
        0o177,          // no owner rw bits
        !0o600,         // every bit except the two required
        0o044,
        0o022,
    ];
    for perms in denying {
        assert_eq!(
            perms & 0o600 == 0o600,
            false,
            "test bug: {perms:o} actually grants 0600"
        );
        for _ in 0..40 {
            let a = rng.any_i32();
            let b = rng.any_i32();
            diff(&format!("safe_add({a}, {b}, {perms:o}) [denied]"), |i| {
                safe_add(i, a, b, perms)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — create_result_string, short ASCII op x random val
// ---------------------------------------------------------------------------
#[test]
fn row08_create_result_string_short() {
    let mut rng = Rng::with_seed(SEED ^ 8);
    let ops = ["multiply", "add", "x", "op-1", "ABC"];
    for op in ops {
        let cop = CString::new(op).unwrap();
        for v in INTERESTING_INTS {
            let c = cop.clone();
            diff(&format!("create_result_string({op:?}, {v})"), move |i| {
                create_result_string(i, &c, v)
            });
        }
        for _ in 0..n() {
            let v = rng.any_i32();
            let c = cop.clone();
            diff(&format!("create_result_string({op:?}, {v})"), move |i| {
                create_result_string(i, &c, v)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — create_result_string with an empty op
// ---------------------------------------------------------------------------
#[test]
fn row09_create_result_string_empty_op() {
    let mut rng = Rng::with_seed(SEED ^ 9);
    let empty = CString::new("").unwrap();
    for v in INTERESTING_INTS {
        let c = empty.clone();
        diff(&format!("create_result_string(\"\", {v})"), move |i| {
            create_result_string(i, &c, v)
        });
    }
    for _ in 0..n() {
        let v = rng.any_i32();
        let c = empty.clone();
        diff(&format!("create_result_string(\"\", {v})"), move |i| {
            create_result_string(i, &c, v)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 10 — create_result_string truncation at the 64-byte snprintf limit
// ---------------------------------------------------------------------------
#[test]
fn row10_create_result_string_truncation() {
    let mut rng = Rng::with_seed(SEED ^ 10);
    // "Operation: " (11) + op + ", Value: " (9) + digits, capped at 63 + NUL.
    for len in 0usize..=80 {
        let op = CString::new("a".repeat(len)).unwrap();
        for v in [0, 7, -7, 12345, i32::MIN, i32::MAX] {
            let c = op.clone();
            diff(&format!("create_result_string(len={len}, {v})"), move |i| {
                create_result_string(i, &c, v)
            });
        }
    }
    for _ in 0..n() {
        let len = rng.range(0, 120) as usize;
        let op = rand_cstring(&mut rng, len);
        let v = rng.any_i32();
        let c = op.clone();
        diff(&format!("create_result_string(rand len={len}, {v})"), move |i| {
            create_result_string(i, &c, v)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 11 — op containing format specifiers and high bytes
// ---------------------------------------------------------------------------
#[test]
fn row11_create_result_string_format_chars_and_high_bytes() {
    let ops: Vec<CString> = vec![
        CString::new("%d").unwrap(),
        CString::new("%s").unwrap(),
        CString::new("%%").unwrap(),
        CString::new("%n").unwrap(),
        CString::new("100%% done %d %s").unwrap(),
        CString::new(vec![0x80u8, 0xFF, 0xC3, 0x28, b'x']).unwrap(),
        CString::new(vec![0xFFu8; 40]).unwrap(),
        CString::new("tab\there\nnewline").unwrap(),
    ];
    let mut rng = Rng::with_seed(SEED ^ 11);
    for op in ops {
        for v in [0, -1, i32::MIN, i32::MAX, rng.any_i32()] {
            let c = op.clone();
            diff(&format!("create_result_string({op:?}, {v})"), move |i| {
                create_result_string(i, &c, v)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — multiply_with_log: return value and out-parameter string
// ---------------------------------------------------------------------------
#[test]
fn row12_multiply_with_log_random() {
    let mut rng = Rng::with_seed(SEED ^ 12);
    for _ in 0..(n() * 2) {
        let a = rng.small();
        let b = rng.small();
        diff(&format!("multiply_with_log({a}, {b})"), |i| {
            multiply_with_log(i, a, b)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 13 — multiply_with_log overflow / zero / negative products
// ---------------------------------------------------------------------------
#[test]
fn row13_multiply_with_log_overflow() {
    let pairs = [
        (i32::MAX, 2),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (65536, 65536),
        (0, i32::MIN),
        (-1, i32::MAX),
        (46341, 46341),
        (-46341, 46341),
        (1 << 16, 1 << 15),
    ];
    for (a, b) in pairs {
        diff(&format!("multiply_with_log({a}, {b})"), |i| {
            multiply_with_log(i, a, b)
        });
    }
    let mut rng = Rng::with_seed(SEED ^ 13);
    for _ in 0..(n() * 2) {
        let a = rng.any_i32();
        let b = rng.any_i32();
        diff(&format!("multiply_with_log({a}, {b})"), |i| {
            multiply_with_log(i, a, b)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 14 — copy_and_sum with count == 0
// ---------------------------------------------------------------------------
#[test]
fn row14_copy_and_sum_count_zero() {
    let mut v: Vec<i32> = vec![1, 2, 3, 4];
    let p = v.as_mut_ptr();
    diff("copy_and_sum(valid, 0)", |i| copy_and_sum(i, p, 0));
    // Also with a one-element and a large backing buffer.
    let mut w: Vec<i32> = vec![i32::MIN; 1];
    let q = w.as_mut_ptr();
    diff("copy_and_sum(1-elem buf, 0)", |i| copy_and_sum(i, q, 0));
}

// ---------------------------------------------------------------------------
// Row 15 — copy_and_sum with count == 1
// ---------------------------------------------------------------------------
#[test]
fn row15_copy_and_sum_count_one() {
    let mut rng = Rng::with_seed(SEED ^ 15);
    for v0 in INTERESTING_INTS {
        let mut v = vec![v0];
        let p = v.as_mut_ptr();
        diff(&format!("copy_and_sum([{v0}], 1)"), |i| copy_and_sum(i, p, 1));
    }
    for _ in 0..n() {
        let v0 = rng.any_i32();
        let mut v = vec![v0];
        let p = v.as_mut_ptr();
        diff(&format!("copy_and_sum([{v0}], 1)"), |i| copy_and_sum(i, p, 1));
    }
}

// ---------------------------------------------------------------------------
// Row 16 — copy_and_sum with count == 3 (complexmode mode 3's shape)
// ---------------------------------------------------------------------------
#[test]
fn row16_copy_and_sum_count_three() {
    let mut rng = Rng::with_seed(SEED ^ 16);
    for _ in 0..(n() * 2) {
        let mut v = vec![rng.any_i32(), rng.any_i32(), rng.any_i32()];
        let ctx = format!("copy_and_sum({v:?}, 3)");
        let p = v.as_mut_ptr();
        diff(&ctx, |i| copy_and_sum(i, p, 3));
    }
}

// ---------------------------------------------------------------------------
// Row 17 — copy_and_sum with large counts, incl. accumulator overflow
// ---------------------------------------------------------------------------
#[test]
fn row17_copy_and_sum_large_counts() {
    let mut rng = Rng::with_seed(SEED ^ 17);
    for count in [2i32, 4, 8, 16, 64, 255, 256, 1024] {
        for _ in 0..8 {
            let mut v: Vec<i32> = (0..count).map(|_| rng.any_i32()).collect();
            let ctx = format!("copy_and_sum(rand, {count})");
            let p = v.as_mut_ptr();
            diff(&ctx, |i| copy_and_sum(i, p, count));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — copy_and_sum with guaranteed accumulator wraparound
// ---------------------------------------------------------------------------
#[test]
fn row18_copy_and_sum_saturated_buffers() {
    for (label, fill) in [("INT_MAX", i32::MAX), ("INT_MIN", i32::MIN), ("-1", -1)] {
        for count in [1i32, 2, 3, 7, 64, 1000] {
            let mut v: Vec<i32> = vec![fill; count as usize];
            let ctx = format!("copy_and_sum([{label}; {count}], {count})");
            let p = v.as_mut_ptr();
            diff(&ctx, |i| copy_and_sum(i, p, count));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — compare_operations on equal strings
// ---------------------------------------------------------------------------
#[test]
fn row19_compare_operations_equal() {
    let mut rng = Rng::with_seed(SEED ^ 19);
    let mut cases: Vec<CString> = vec![
        CString::new("").unwrap(),
        CString::new("none").unwrap(),
        CString::new("addition").unwrap(),
        CString::new("multiplication").unwrap(),
        CString::new("array_sum").unwrap(),
        CString::new("complex").unwrap(),
    ];
    for _ in 0..n() {
        let len = rng.range(0, 40) as usize;
        cases.push(rand_cstring(&mut rng, len));
    }
    for s in cases {
        let a = s.clone();
        let b = s.clone();
        diff(&format!("compare_operations(eq {:?})", s.as_bytes()), move |i| {
            compare_operations(i, a.as_ptr(), b.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// Row 20 — compare_operations on unequal strings, both orderings
// ---------------------------------------------------------------------------
#[test]
fn row20_compare_operations_unequal() {
    let words = [
        "", "a", "b", "A", "aa", "ab", "abc", "abcd", "none", "nonf", "none ", "addition",
        "array_sum", "complex", "multiplication", "zzz",
    ];
    for x in words {
        for y in words {
            let a = CString::new(x).unwrap();
            let b = CString::new(y).unwrap();
            diff(&format!("compare_operations({x:?}, {y:?})"), move |i| {
                compare_operations(i, a.as_ptr(), b.as_ptr())
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — compare_operations with bytes >= 0x80 (unsigned char comparison)
// ---------------------------------------------------------------------------
#[test]
fn row21_compare_operations_high_bytes() {
    let variants: Vec<Vec<u8>> = vec![
        vec![b'x', 0x7f],
        vec![b'x', 0x80],
        vec![b'x', 0xff],
        vec![0x80],
        vec![0xff],
        vec![0x01],
        vec![0xfe, 0xff, 0x01],
        vec![0xfe, 0xff],
    ];
    for x in &variants {
        for y in &variants {
            let a = CString::new(x.clone()).unwrap();
            let b = CString::new(y.clone()).unwrap();
            diff(&format!("compare_operations({x:02x?}, {y:02x?})"), move |i| {
                compare_operations(i, a.as_ptr(), b.as_ptr())
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22 — compare_operations on fully random byte strings
// ---------------------------------------------------------------------------
#[test]
fn row22_compare_operations_random() {
    let mut rng = Rng::with_seed(SEED ^ 22);
    for _ in 0..(n() * 2) {
        let la = rng.range(0, 24) as usize;
        let lb = rng.range(0, 24) as usize;
        let a = rand_cstring(&mut rng, la);
        let b = rand_cstring(&mut rng, lb);
        // Occasionally make them share a prefix to hit the interesting cases.
        let (a, b) = if rng.next_u64() % 3 == 0 {
            let mut merged = a.as_bytes().to_vec();
            merged.extend_from_slice(b.as_bytes());
            (a.clone(), CString::new(merged).unwrap())
        } else {
            (a, b)
        };
        let (x, y) = (a.clone(), b.clone());
        diff("compare_operations(random)", move |i| {
            compare_operations(i, x.as_ptr(), y.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// Row 23 — complexmode mode 1
// ---------------------------------------------------------------------------
#[test]
fn row23_complexmode_mode1() {
    let mut rng = Rng::with_seed(SEED ^ 23);
    for _ in 0..(n() * 2) {
        let (a, b, c) = (rng.small(), rng.small(), rng.any_i32());
        diff(&format!("complexmode(1, {a}, {b}, {c})"), |i| {
            complexmode(i, 1, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 24 — complexmode mode 1 with overflow
// ---------------------------------------------------------------------------
#[test]
fn row24_complexmode_mode1_overflow() {
    let pairs = [
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (1 << 30, 1 << 30),
    ];
    for (a, b) in pairs {
        diff(&format!("complexmode(1, {a}, {b}, 0)"), |i| {
            complexmode(i, 1, a, b, 0)
        });
    }
    let mut rng = Rng::with_seed(SEED ^ 24);
    for _ in 0..n() {
        let (a, b) = (rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode(1, {a}, {b}, 0)"), |i| {
            complexmode(i, 1, a, b, 0)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 25 — complexmode mode 2
// ---------------------------------------------------------------------------
#[test]
fn row25_complexmode_mode2() {
    let mut rng = Rng::with_seed(SEED ^ 25);
    for _ in 0..(n() * 2) {
        let (a, b, c) = (rng.small(), rng.small(), rng.any_i32());
        diff(&format!("complexmode(2, {a}, {b}, {c})"), |i| {
            complexmode(i, 2, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 26 — complexmode mode 2 with overflow / zero / negative product
// ---------------------------------------------------------------------------
#[test]
fn row26_complexmode_mode2_overflow() {
    let pairs = [
        (i32::MAX, 2),
        (i32::MIN, -1),
        (65536, 65536),
        (0, 5),
        (-3, 7),
        (i32::MIN, i32::MIN),
        (46341, 46341),
    ];
    for (a, b) in pairs {
        diff(&format!("complexmode(2, {a}, {b}, 0)"), |i| {
            complexmode(i, 2, a, b, 0)
        });
    }
    let mut rng = Rng::with_seed(SEED ^ 26);
    for _ in 0..n() {
        let (a, b) = (rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode(2, {a}, {b}, 0)"), |i| {
            complexmode(i, 2, a, b, 0)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 27 — complexmode mode 3
// ---------------------------------------------------------------------------
#[test]
fn row27_complexmode_mode3() {
    let mut rng = Rng::with_seed(SEED ^ 27);
    for _ in 0..(n() * 2) {
        let (a, b, c) = (rng.small(), rng.small(), rng.small());
        diff(&format!("complexmode(3, {a}, {b}, {c})"), |i| {
            complexmode(i, 3, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 28 — complexmode mode 3 with overflow
// ---------------------------------------------------------------------------
#[test]
fn row28_complexmode_mode3_overflow() {
    let triples = [
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, 1, 0),
        (i32::MIN, -1, 0),
        (1 << 30, 1 << 30, 1 << 30),
    ];
    for (a, b, c) in triples {
        diff(&format!("complexmode(3, {a}, {b}, {c})"), |i| {
            complexmode(i, 3, a, b, c)
        });
    }
    let mut rng = Rng::with_seed(SEED ^ 28);
    for _ in 0..n() {
        let (a, b, c) = (rng.any_i32(), rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode(3, {a}, {b}, {c})"), |i| {
            complexmode(i, 3, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 29 — complexmode mode 4 (else branch: v1 + v2 + v3)
// ---------------------------------------------------------------------------
#[test]
fn row29_complexmode_mode4() {
    let mut rng = Rng::with_seed(SEED ^ 29);
    for _ in 0..(n() * 2) {
        let (a, b, c) = (rng.small(), rng.small(), rng.small());
        diff(&format!("complexmode(4, {a}, {b}, {c})"), |i| {
            complexmode(i, 4, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 30 — complexmode mode 4 with overflow
// ---------------------------------------------------------------------------
#[test]
fn row30_complexmode_mode4_overflow() {
    let triples = [
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, 1, 0),
        (i32::MIN, -1, 0),
        (46341, 46341, 1),
    ];
    for (a, b, c) in triples {
        diff(&format!("complexmode(4, {a}, {b}, {c})"), |i| {
            complexmode(i, 4, a, b, c)
        });
    }
    let mut rng = Rng::with_seed(SEED ^ 30);
    for _ in 0..n() {
        let (a, b, c) = (rng.any_i32(), rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode(4, {a}, {b}, {c})"), |i| {
            complexmode(i, 4, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 31 — every mode x extreme operand permutations
// ---------------------------------------------------------------------------
#[test]
fn row31_complexmode_all_modes_extremes() {
    let extremes = [i32::MIN, i32::MAX, 0, -1, 1];
    for mode in [1, 2, 3, 4] {
        for &a in &extremes {
            for &b in &extremes {
                for &c in &extremes {
                    diff(&format!("complexmode({mode}, {a}, {b}, {c})"), |i| {
                        complexmode(i, mode, a, b, c)
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 32 — full random sweep over mode in [-8, 12] x random operand triples
// ---------------------------------------------------------------------------
#[test]
fn row32_complexmode_random_sweep() {
    let mut rng = Rng::with_seed(SEED ^ 32);
    for _ in 0..(n() * 4) {
        let mode = rng.range(-8, 12) as i32;
        let (a, b, c) = (rng.any_i32(), rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode({mode}, {a}, {b}, {c})"), |i| {
            complexmode(i, mode, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// Extra: over-read within an over-allocated buffer (ERRORS.md row 9)
// ---------------------------------------------------------------------------
#[test]
fn extra_copy_and_sum_count_beyond_logical_length() {
    // 4096 ints of known content; ask for far more than the "logical" 3
    // elements, but stay inside the allocation so both libraries read the same
    // bytes.
    let mut v: Vec<i32> = (0..4096).map(|k| (k as i32).wrapping_mul(2654435761u32 as i32)).collect();
    let p = v.as_mut_ptr();
    for count in [3i32, 4, 10, 64, 1000, 4096] {
        diff(&format!("copy_and_sum(over-read, {count})"), |i| {
            copy_and_sum(i, p, count)
        });
    }
}
