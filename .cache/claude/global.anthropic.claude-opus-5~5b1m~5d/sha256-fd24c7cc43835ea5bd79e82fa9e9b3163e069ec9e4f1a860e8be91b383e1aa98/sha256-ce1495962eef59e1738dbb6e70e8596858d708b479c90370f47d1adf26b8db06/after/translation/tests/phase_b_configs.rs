//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1 .. C37). Every test drives BOTH the C
//! `.so` and the Rust `.so` through `libloading` and compares the return value,
//! the bytes printed to `stdout`, and any heap buffer handed back through an
//! out-param — byte for byte.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

const INT_MAX: i32 = i32::MAX;
const INT_MIN: i32 = i32::MIN;

const READ_PERM: i32 = 0o400;
const WRITE_PERM: i32 = 0o200;
const EXEC_PERM: i32 = 0o100;

// ---------------------------------------------------------------------------
// per-function differential drivers
// ---------------------------------------------------------------------------

/// `create_result_string`: observable = (buffer contents or NULL, stdout).
fn drive_crs(lib: &Lib, op: Option<&[u8]>, val: i32) -> (Option<Vec<u8>>, Vec<u8>) {
    let owned = op.map(cstring);
    let p: *const c_char = owned
        .as_ref()
        .map(|c| c.as_ptr())
        .unwrap_or(std::ptr::null());
    capture_stdout(|| unsafe {
        let out = (lib.create_result_string)(p, val);
        let s = read_cstr(out);
        cfree(out);
        s
    })
}

/// `multiply_with_log`: observable = ((return value, message or NULL), stdout).
fn drive_mwl(lib: &Lib, a: i32, b: i32) -> ((c_int, Option<Vec<u8>>), Vec<u8>) {
    capture_stdout(|| unsafe {
        // Poison the out-param so we can tell "not written" from "written NULL".
        let mut msg: *mut c_char = 1usize as *mut c_char;
        let ret = (lib.multiply_with_log)(a, b, &mut msg);
        assert_ne!(msg as usize, 1, "{}: out-param was never written", lib.name);
        let s = read_cstr(msg);
        cfree(msg);
        (ret, s)
    })
}

/// `copy_and_sum`: observable = ((return value, src buffer after the call), stdout).
/// The C `memcpy`s into a scratch buffer, so `src` must come back untouched.
fn drive_cas(lib: &Lib, src: &[i32], count: i32) -> ((c_int, Vec<i32>), Vec<u8>) {
    let mut buf = src.to_vec();
    capture_stdout(|| unsafe {
        let ret = (lib.copy_and_sum)(buf.as_mut_ptr(), count);
        (ret, buf.clone())
    })
}

fn drive_cmp(lib: &Lib, a: &[u8], b: &[u8]) -> (c_int, Vec<u8>) {
    let (ca, cb) = (cstring(a), cstring(b));
    capture_stdout(|| unsafe { (lib.compare_operations)(ca.as_ptr(), cb.as_ptr()) })
}

fn drive_add(lib: &Lib, a: i32, b: i32, perms: i32) -> (c_int, Vec<u8>) {
    capture_stdout(|| unsafe { (lib.safe_add)(a, b, perms) })
}

fn drive_mode(lib: &Lib, m: i32, v1: i32, v2: i32, v3: i32) -> (c_int, Vec<u8>) {
    capture_stdout(|| unsafe { (lib.complexmode)(m, v1, v2, v3) })
}

// ===========================================================================
// C1 .. C5 — check_permissions
// ===========================================================================

/// C1: `required == 0` is the always-accept boundary.
#[test]
fn c1_check_permissions_required_zero() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC1);
    for i in 0..20_000 {
        let perms = if i < 4 {
            [0, -1, INT_MIN, INT_MAX][i as usize]
        } else {
            rng.i32_any()
        };
        let cv = unsafe { (c.check_permissions)(perms, 0) };
        let rv = unsafe { (r.check_permissions)(perms, 0) };
        assert_eq!(cv, rv, "check_permissions({perms}, 0)");
    }
}

/// C2: single-bit `required` (each of the three permission macros).
#[test]
fn c2_check_permissions_single_bit() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC2);
    for &req in &[READ_PERM, WRITE_PERM, EXEC_PERM, 1, 1 << 30, INT_MIN] {
        for _ in 0..20_000 {
            let perms = rng.i32_any();
            let cv = unsafe { (c.check_permissions)(perms, req) };
            let rv = unsafe { (r.check_permissions)(perms, req) };
            assert_eq!(cv, rv, "check_permissions({perms}, {req})");
        }
    }
}

/// C3: multi-bit `required` against superset / equal / partial / disjoint `perms`.
#[test]
fn c3_check_permissions_multi_bit() {
    let (c, r) = libs();
    let reqs = [
        READ_PERM | WRITE_PERM,
        0o700,
        0o644,
        0o777,
        READ_PERM | EXEC_PERM,
    ];
    let mut rng = Rng::new(0xC3);
    for &req in &reqs {
        let mut cases: Vec<i32> = vec![
            req,          // exact equal
            req | 0o7000, // strict superset
            req & !0o100, // partial overlap (one bit cleared)
            !req,         // disjoint
            0,
            -1,
        ];
        for _ in 0..5_000 {
            cases.push(rng.i32_any());
            cases.push(req & rng.i32_any()); // random subset
            cases.push(req | rng.i32_any()); // random superset
        }
        for &perms in &cases {
            let cv = unsafe { (c.check_permissions)(perms, req) };
            let rv = unsafe { (r.check_permissions)(perms, req) };
            assert_eq!(cv, rv, "check_permissions({perms}, {req})");
        }
    }
}

/// C4: negative / sign-bit `perms` and `required`.
#[test]
fn c4_check_permissions_negative() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC4);
    let edges = [0, -1, 1, INT_MIN, INT_MAX, INT_MIN + 1, INT_MAX - 1];
    for &p in &edges {
        for &q in &edges {
            for (perms, req) in [(p, q), (q, p)] {
                let cv = unsafe { (c.check_permissions)(perms, req) };
                let rv = unsafe { (r.check_permissions)(perms, req) };
                assert_eq!(cv, rv, "check_permissions({perms}, {req})");
            }
        }
    }
    for _ in 0..50_000 {
        let (perms, req) = (rng.i32_any() | INT_MIN, rng.i32_any() | INT_MIN);
        let cv = unsafe { (c.check_permissions)(perms, req) };
        let rv = unsafe { (r.check_permissions)(perms, req) };
        assert_eq!(cv, rv, "check_permissions({perms}, {req})");
    }
}

/// C5: exhaustive sweep of the whole 9-bit permission space (512 x 512).
#[test]
fn c5_check_permissions_exhaustive_9bit() {
    let (c, r) = libs();
    for perms in 0..512i32 {
        for req in 0..512i32 {
            let cv = unsafe { (c.check_permissions)(perms, req) };
            let rv = unsafe { (r.check_permissions)(perms, req) };
            assert_eq!(cv, rv, "check_permissions({perms:#o}, {req:#o})");
        }
    }
}

// ===========================================================================
// C6 .. C9 — safe_add
// ===========================================================================

/// C6: permission granted, small operands.
#[test]
fn c6_safe_add_granted_small() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC6);
    for _ in 0..3_000 {
        let (a, b) = (rng.i32_small(), rng.i32_small());
        // Any perms superset of 0600 must take the accept path.
        let perms = (READ_PERM | WRITE_PERM) | (rng.i32_any() & !0o600);
        assert_same(
            &format!("safe_add({a}, {b}, {perms:#o})"),
            drive_add(c, a, b, perms),
            drive_add(r, a, b, perms),
        );
    }
}

/// C7: permission granted, `a + b` overflows `int`.
#[test]
fn c7_safe_add_overflow() {
    let (c, r) = libs();
    let perms = 0o644;
    let mut cases: Vec<(i32, i32)> = vec![
        (INT_MAX, 1),
        (1, INT_MAX),
        (INT_MAX, INT_MAX),
        (INT_MIN, -1),
        (-1, INT_MIN),
        (INT_MIN, INT_MIN),
        (INT_MAX, INT_MIN),
        (INT_MAX / 2 + 1, INT_MAX / 2 + 1),
        (0, 0),
        (INT_MIN, INT_MAX),
    ];
    let mut rng = Rng::new(0xC7);
    for _ in 0..2_000 {
        cases.push((rng.i32_any(), rng.i32_any()));
    }
    for (a, b) in cases {
        assert_same(
            &format!("safe_add({a}, {b}, {perms:#o})"),
            drive_add(c, a, b, perms),
            drive_add(r, a, b, perms),
        );
    }
}

/// C8: `perms` missing exactly one required bit -> reject path + message.
#[test]
fn c8_safe_add_missing_one_bit() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC8);
    for &perms in &[
        READ_PERM,             // 0400 only
        WRITE_PERM,            // 0200 only
        0,                     // nothing
        EXEC_PERM,             // wrong bit
        READ_PERM | EXEC_PERM, // missing WRITE
        WRITE_PERM | EXEC_PERM,
        0o477 & !WRITE_PERM,
    ] {
        for _ in 0..300 {
            let (a, b) = (rng.i32_any(), rng.i32_any());
            assert_same(
                &format!("safe_add({a}, {b}, {perms:#o})"),
                drive_add(c, a, b, perms),
                drive_add(r, a, b, perms),
            );
        }
    }
}

/// C9: fully randomized `perms` — accept/reject paths interleaved.
#[test]
fn c9_safe_add_random_perms() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC9);
    for _ in 0..3_000 {
        let (a, b, perms) = (rng.i32_any(), rng.i32_any(), rng.i32_any() & 0o7777);
        assert_same(
            &format!("safe_add({a}, {b}, {perms:#o})"),
            drive_add(c, a, b, perms),
            drive_add(r, a, b, perms),
        );
    }
}

// ===========================================================================
// C10 .. C13 — create_result_string
// ===========================================================================

/// C10: empty `op`, small `val` — shortest output.
#[test]
fn c10_create_result_string_empty_op() {
    let (c, r) = libs();
    for val in [0, 1, -1, 9, -9, 10, -10] {
        assert_same(
            &format!("create_result_string(\"\", {val})"),
            drive_crs(c, Some(b""), val),
            drive_crs(r, Some(b""), val),
        );
    }
}

/// C11: short `op`, `val` over the full `i32` range.
#[test]
fn c11_create_result_string_val_range() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC11);
    let mut vals: Vec<i32> = vec![0, 1, -1, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1];
    for _ in 0..2_000 {
        vals.push(rng.i32_any());
    }
    for op in [b"multiply".as_ref(), b"add".as_ref(), b"x".as_ref()] {
        for &val in &vals {
            assert_same(
                &format!("create_result_string({:?}, {val})", show(op)),
                drive_crs(c, Some(op), val),
                drive_crs(r, Some(op), val),
            );
        }
    }
}

/// C12: `op` length swept across the 64-byte `snprintf` truncation boundary.
#[test]
fn c12_create_result_string_truncation_sweep() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC12);
    for len in 0..=80usize {
        for val in [0, 7, -7, INT_MAX, INT_MIN] {
            let op: Vec<u8> = vec![b'A'; len];
            assert_same(
                &format!("create_result_string(len={len}, {val})"),
                drive_crs(c, Some(&op), val),
                drive_crs(r, Some(&op), val),
            );
        }
        // Random content of the same length, so truncation is not just 'A's.
        for _ in 0..20 {
            let op = rng.cbytes(len);
            let val = rng.i32_any();
            assert_same(
                &format!("create_result_string(random len={len}, {val})"),
                drive_crs(c, Some(&op), val),
                drive_crs(r, Some(&op), val),
            );
        }
    }
}

/// C13: `op` with high-bit bytes, punctuation, and `%` (data, not format).
#[test]
fn c13_create_result_string_hostile_bytes() {
    let (c, r) = libs();
    let cases: Vec<Vec<u8>> = vec![
        b"%s".to_vec(),
        b"%d %d %d %d %d".to_vec(),
        b"%n".to_vec(),
        b"100%%".to_vec(),
        b"a\tb\rc".to_vec(),
        vec![0x80, 0xFF, 0x7F, 0x01],
        vec![0xFFu8; 40],
        "operación".as_bytes().to_vec(),
        b", Value: 0".to_vec(),
    ];
    for op in &cases {
        for val in [0, -12345, INT_MIN] {
            assert_same(
                &format!("create_result_string({}, {val})", show(op)),
                drive_crs(c, Some(op), val),
                drive_crs(r, Some(op), val),
            );
        }
    }
}

// ===========================================================================
// C14 .. C16 — multiply_with_log
// ===========================================================================

/// C14: valid out-param, small operands.
#[test]
fn c14_multiply_with_log_small() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC14);
    for _ in 0..3_000 {
        let (a, b) = (rng.i32_small(), rng.i32_small());
        assert_same(
            &format!("multiply_with_log({a}, {b})"),
            drive_mwl(c, a, b),
            drive_mwl(r, a, b),
        );
    }
}

/// C15: product overflows `int` (computed twice in the C — both must wrap alike).
#[test]
fn c15_multiply_with_log_overflow() {
    let (c, r) = libs();
    let mut cases: Vec<(i32, i32)> = vec![
        (INT_MIN, -1),
        (-1, INT_MIN),
        (INT_MAX, 2),
        (2, INT_MAX),
        (INT_MAX, INT_MAX),
        (INT_MIN, INT_MIN),
        (INT_MAX, INT_MIN),
        (65536, 65536),
        (46341, 46341),
        (-46341, 46341),
        (1 << 30, 4),
    ];
    let mut rng = Rng::new(0xC15);
    for _ in 0..2_000 {
        cases.push((rng.i32_any(), rng.i32_any()));
    }
    for (a, b) in cases {
        assert_same(
            &format!("multiply_with_log({a}, {b})"),
            drive_mwl(c, a, b),
            drive_mwl(r, a, b),
        );
    }
}

/// C16: zero and negative products.
#[test]
fn c16_multiply_with_log_zero_and_negative() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC16);
    let mut cases: Vec<(i32, i32)> = vec![
        (0, 0),
        (0, 12345),
        (12345, 0),
        (0, INT_MIN),
        (INT_MIN, 0),
        (-1, 1),
        (1, -1),
        (-3, 7),
        (7, -3),
        (-3, -7),
    ];
    for _ in 0..1_000 {
        let a = rng.i32_small();
        cases.push((a, 0));
        cases.push((0, a));
        cases.push((-a.abs().max(1), rng.i32_small().abs().max(1)));
    }
    for (a, b) in cases {
        assert_same(
            &format!("multiply_with_log({a}, {b})"),
            drive_mwl(c, a, b),
            drive_mwl(r, a, b),
        );
    }
}

// ===========================================================================
// C17 .. C21 — copy_and_sum
// ===========================================================================

/// C17: `count == 0` with non-NULL src (`malloc(0)`, empty loop).
#[test]
fn c17_copy_and_sum_zero_count() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC17);
    for _ in 0..500 {
        let buf: Vec<i32> = (0..4).map(|_| rng.i32_any()).collect();
        assert_same(
            "copy_and_sum(buf, 0)",
            drive_cas(c, &buf, 0),
            drive_cas(r, &buf, 0),
        );
    }
}

/// C18: `count == 1`.
#[test]
fn c18_copy_and_sum_one() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC18);
    let mut vals: Vec<i32> = vec![0, 1, -1, INT_MAX, INT_MIN];
    for _ in 0..2_000 {
        vals.push(rng.i32_any());
    }
    for v in vals {
        let buf = vec![v];
        assert_same(
            &format!("copy_and_sum([{v}], 1)"),
            drive_cas(c, &buf, 1),
            drive_cas(r, &buf, 1),
        );
    }
}

/// C19: `count == 3` — the shape `complexmode` mode 3 hard-codes.
#[test]
fn c19_copy_and_sum_three() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC19);
    for i in 0..3_000 {
        let buf: Vec<i32> = if i < 3 {
            match i {
                0 => vec![0, 0, 0],
                1 => vec![INT_MAX, INT_MAX, INT_MAX],
                _ => vec![INT_MIN, INT_MIN, INT_MIN],
            }
        } else if i % 3 == 0 {
            (0..3).map(|_| rng.i32_any()).collect()
        } else {
            (0..3).map(|_| rng.i32_small()).collect()
        };
        assert_same(
            &format!("copy_and_sum({buf:?}, 3)"),
            drive_cas(c, &buf, 3),
            drive_cas(r, &buf, 3),
        );
    }
}

/// C20: many elements.
#[test]
fn c20_copy_and_sum_many() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC20);
    for &n in &[2usize, 4, 5, 8, 17, 64, 255, 1000, 65536, 1 << 20] {
        let reps = if n > 4096 { 3 } else { 40 };
        for _ in 0..reps {
            let buf: Vec<i32> = (0..n).map(|_| rng.i32_small()).collect();
            assert_same(
                &format!("copy_and_sum(len={n}, {n})"),
                drive_cas(c, &buf, n as i32),
                drive_cas(r, &buf, n as i32),
            );
        }
        // count smaller than the buffer: only the prefix is summed.
        let buf: Vec<i32> = (0..n).map(|_| rng.i32_any()).collect();
        for k in [0usize, 1, n / 2, n] {
            assert_same(
                &format!("copy_and_sum(len={n}, {k})"),
                drive_cas(c, &buf, k as i32),
                drive_cas(r, &buf, k as i32),
            );
        }
    }
}

/// C21: the running `int` accumulator overflows repeatedly mid-loop.
#[test]
fn c21_copy_and_sum_overflow() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC21);
    let patterns: Vec<Vec<i32>> = vec![
        vec![INT_MAX, 1],
        vec![INT_MAX, INT_MAX, INT_MAX, INT_MAX],
        vec![INT_MIN, -1],
        vec![INT_MIN; 7],
        vec![INT_MAX, INT_MIN, INT_MAX, INT_MIN],
        vec![1 << 30; 9],
        vec![-(1 << 30); 9],
    ];
    for buf in &patterns {
        let n = buf.len() as i32;
        assert_same(
            &format!("copy_and_sum({buf:?}, {n})"),
            drive_cas(c, buf, n),
            drive_cas(r, buf, n),
        );
    }
    for _ in 0..600 {
        let n = 1 + rng.below(40) as usize;
        // Large magnitudes guarantee wraparound part-way through the loop.
        let buf: Vec<i32> = (0..n).map(|_| rng.i32_any() | (1 << 30)).collect();
        assert_same(
            &format!("copy_and_sum(overflowing len={n})"),
            drive_cas(c, &buf, n as i32),
            drive_cas(r, &buf, n as i32),
        );
    }
}

// ===========================================================================
// C22 .. C27 — compare_operations
// ===========================================================================

/// C22: equal strings (including empty vs empty).
#[test]
fn c22_compare_operations_equal() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC22);
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"none".to_vec(),
        b"multiplication".to_vec(),
        vec![0xFFu8; 64],
    ];
    for _ in 0..500 {
        cases.push(rng.cbytes_upto(33));
    }
    for s in &cases {
        assert_same(
            &format!("compare_operations({0}, {0})", show(s)),
            drive_cmp(c, s, s),
            drive_cmp(r, s, s),
        );
    }
}

/// C23: differ at the first byte, both orders.
#[test]
fn c23_compare_operations_first_byte() {
    let (c, r) = libs();
    for a in 1u16..=255 {
        for b in [1u16, 2, 64, 65, 127, 128, 129, 200, 255] {
            let x = vec![a as u8, b'z'];
            let y = vec![b as u8, b'z'];
            assert_same(
                &format!("compare_operations([{a},z], [{b},z])"),
                drive_cmp(c, &x, &y),
                drive_cmp(r, &x, &y),
            );
        }
    }
}

/// C24: proper prefix, both orders.
#[test]
fn c24_compare_operations_prefix() {
    let (c, r) = libs();
    let base = b"multiplication_operation_string";
    for i in 0..base.len() {
        for j in 0..base.len() {
            let (x, y) = (&base[..i], &base[..j]);
            assert_same(
                &format!("compare_operations(prefix {i}, prefix {j})"),
                drive_cmp(c, x, y),
                drive_cmp(r, x, y),
            );
        }
    }
}

/// C25: long strings differing only at a late byte (vectorized libc path).
#[test]
fn c25_compare_operations_long() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC25);
    for len in [31usize, 32, 33, 63, 64, 65, 127, 128, 255, 256] {
        let base = rng.cbytes(len);
        for pos in [0usize, 1, len / 2, len - 1] {
            let mut alt = base.clone();
            alt[pos] = alt[pos].wrapping_add(1);
            if alt[pos] == 0 {
                alt[pos] = 1;
            }
            assert_same(
                &format!("compare_operations(len={len}, differ@{pos})"),
                drive_cmp(c, &base, &alt),
                drive_cmp(r, &base, &alt),
            );
            assert_same(
                &format!("compare_operations(len={len}, differ@{pos}, swapped)"),
                drive_cmp(c, &alt, &base),
                drive_cmp(r, &alt, &base),
            );
        }
    }
}

/// C26: high-bit vs low-bit bytes — `strcmp` compares as *unsigned* char.
#[test]
fn c26_compare_operations_high_bit() {
    let (c, r) = libs();
    for &hi in &[0x80u8, 0x81, 0xC0, 0xFE, 0xFF] {
        for &lo in &[0x01u8, 0x20, 0x41, 0x7E, 0x7F] {
            for prefix in [0usize, 1, 8, 17] {
                let mut x = vec![b'p'; prefix];
                let mut y = x.clone();
                x.push(hi);
                y.push(lo);
                assert_same(
                    &format!("compare_operations(prefix={prefix}, {hi:#x} vs {lo:#x})"),
                    drive_cmp(c, &x, &y),
                    drive_cmp(r, &x, &y),
                );
                assert_same(
                    &format!("compare_operations(prefix={prefix}, {lo:#x} vs {hi:#x})"),
                    drive_cmp(c, &y, &x),
                    drive_cmp(r, &y, &x),
                );
            }
        }
    }
}

/// C27: fully randomized byte strings and lengths.
#[test]
fn c27_compare_operations_random() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC27);
    for _ in 0..4_000 {
        let a = rng.cbytes_upto(33);
        let b = rng.cbytes_upto(33);
        assert_same(
            &format!("compare_operations({}, {})", show(&a), show(&b)),
            drive_cmp(c, &a, &b),
            drive_cmp(r, &a, &b),
        );
    }
}

// ===========================================================================
// C28 .. C35 — complexmode
// ===========================================================================

/// C28: mode 1, small values (`value3` present but unused).
#[test]
fn c28_complexmode_mode1_small() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC28);
    for _ in 0..2_000 {
        let (v1, v2, v3) = (rng.i32_small(), rng.i32_small(), rng.i32_any());
        assert_same(
            &format!("complexmode(1, {v1}, {v2}, {v3})"),
            drive_mode(c, 1, v1, v2, v3),
            drive_mode(r, 1, v1, v2, v3),
        );
    }
}

/// C29: mode 1 with `value1 + value2` overflow.
#[test]
fn c29_complexmode_mode1_overflow() {
    let (c, r) = libs();
    let mut cases: Vec<(i32, i32)> = vec![
        (INT_MAX, 1),
        (INT_MAX, INT_MAX),
        (INT_MIN, -1),
        (INT_MIN, INT_MIN),
        (INT_MAX, INT_MIN),
        (0, 0),
    ];
    let mut rng = Rng::new(0xC29);
    for _ in 0..1_500 {
        cases.push((rng.i32_any(), rng.i32_any()));
    }
    for (v1, v2) in cases {
        let v3 = rng.i32_any();
        assert_same(
            &format!("complexmode(1, {v1}, {v2}, {v3})"),
            drive_mode(c, 1, v1, v2, v3),
            drive_mode(r, 1, v1, v2, v3),
        );
    }
}

/// C30: mode 2, small values — full three-line output.
#[test]
fn c30_complexmode_mode2_small() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC30);
    for _ in 0..2_000 {
        let (v1, v2, v3) = (rng.i32_small(), rng.i32_small(), rng.i32_any());
        assert_same(
            &format!("complexmode(2, {v1}, {v2}, {v3})"),
            drive_mode(c, 2, v1, v2, v3),
            drive_mode(r, 2, v1, v2, v3),
        );
    }
}

/// C31: mode 2 with product overflow, and product == 0 (the
/// `strcmp(log, "") == 0` gate must still be false).
#[test]
fn c31_complexmode_mode2_overflow_and_zero() {
    let (c, r) = libs();
    let mut cases: Vec<(i32, i32)> = vec![
        (0, 0),
        (0, 99),
        (99, 0),
        (0, INT_MIN),
        (INT_MIN, -1),
        (INT_MAX, 2),
        (INT_MAX, INT_MAX),
        (INT_MIN, INT_MIN),
        (65536, 65536),
        (-1, -1),
    ];
    let mut rng = Rng::new(0xC31);
    for _ in 0..1_500 {
        cases.push((rng.i32_any(), rng.i32_any()));
    }
    for (v1, v2) in cases {
        let v3 = rng.i32_any();
        assert_same(
            &format!("complexmode(2, {v1}, {v2}, {v3})"),
            drive_mode(c, 2, v1, v2, v3),
            drive_mode(r, 2, v1, v2, v3),
        );
    }
}

/// C32: mode 3, randomized values plus sum-overflow corners.
#[test]
fn c32_complexmode_mode3() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC32);
    let mut cases: Vec<(i32, i32, i32)> = vec![
        (0, 0, 0),
        (INT_MAX, INT_MAX, INT_MAX),
        (INT_MIN, INT_MIN, INT_MIN),
        (INT_MAX, 1, 0),
        (INT_MIN, -1, 0),
        (INT_MAX, INT_MIN, INT_MAX),
        (-1, -1, -1),
    ];
    for _ in 0..2_000 {
        cases.push((rng.i32_any(), rng.i32_any(), rng.i32_any()));
        cases.push((rng.i32_small(), rng.i32_small(), rng.i32_small()));
    }
    for (v1, v2, v3) in cases {
        assert_same(
            &format!("complexmode(3, {v1}, {v2}, {v3})"),
            drive_mode(c, 3, v1, v2, v3),
            drive_mode(r, 3, v1, v2, v3),
        );
    }
}

/// C33: mode 4 — `0644 & 0100 == 0`, so the multiply branch is dead and the
/// result must be `v1 + v2 + v3`.
#[test]
fn c33_complexmode_mode4() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC33);
    let mut cases: Vec<(i32, i32, i32)> = vec![
        (0, 0, 0),
        (2, 3, 4), // 2*3+4 = 10 vs 2+3+4 = 9 -> distinguishes the branches
        (10, 10, 1),
        (INT_MAX, INT_MAX, INT_MAX),
        (INT_MIN, INT_MIN, INT_MIN),
        (INT_MAX, 1, 0),
        (INT_MIN, -1, 0),
    ];
    for _ in 0..2_000 {
        cases.push((rng.i32_any(), rng.i32_any(), rng.i32_any()));
        cases.push((rng.i32_small(), rng.i32_small(), rng.i32_small()));
    }
    for (v1, v2, v3) in cases {
        let ctx = format!("complexmode(4, {v1}, {v2}, {v3})");
        let cr = drive_mode(c, 4, v1, v2, v3);
        // The C takes the else-branch, so the sum (not the product) is expected.
        let expect = v1.wrapping_add(v2).wrapping_add(v3);
        assert_eq!(cr.0, expect, "C reference for {ctx} took the multiply branch?");
        assert_same(&ctx, cr, drive_mode(r, 4, v1, v2, v3));
    }
}

/// C34: all four valid modes x the distinguished value shapes.
#[test]
fn c34_complexmode_modes_x_shapes() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC34);
    let mut shapes: Vec<(i32, i32, i32)> = vec![
        (0, 0, 0),
        (INT_MAX, INT_MAX, INT_MAX),
        (INT_MIN, INT_MIN, INT_MIN),
        (INT_MAX, INT_MIN, 0),
        (INT_MIN, INT_MAX, -1),
        (1, -1, 1),
        (-1, 1, -1),
        (1, 0, INT_MIN),
    ];
    for _ in 0..400 {
        shapes.push((rng.i32_any(), rng.i32_any(), rng.i32_any()));
        shapes.push((rng.i32_small(), rng.i32_small(), rng.i32_small()));
    }
    for mode in 1..=4 {
        for &(v1, v2, v3) in &shapes {
            assert_same(
                &format!("complexmode({mode}, {v1}, {v2}, {v3})"),
                drive_mode(c, mode, v1, v2, v3),
                drive_mode(r, mode, v1, v2, v3),
            );
        }
    }
}

/// C35: randomized `mode` over the whole `i32` range x randomized values —
/// valid arms and the `default` arm interleaved, checking that the
/// `Operation performed:` trailer appears only for modes 1..4.
#[test]
fn c35_complexmode_random_mode() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC35);
    for i in 0..4_000 {
        // Bias a quarter of the draws into the valid range so both sides of the
        // switch are hit densely.
        let mode = if i % 4 == 0 {
            1 + (rng.below(6) as i32) - 1
        } else {
            rng.i32_any()
        };
        let (v1, v2, v3) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        let ctx = format!("complexmode({mode}, {v1}, {v2}, {v3})");
        let cr = drive_mode(c, mode, v1, v2, v3);
        let has_trailer = cr.1.windows(20).any(|w| w == b"Operation performed:");
        assert_eq!(
            has_trailer,
            (1..=4).contains(&mode),
            "C trailer presence unexpected for {ctx}: {}",
            show(&cr.1)
        );
        assert_same(&ctx, cr, drive_mode(r, mode, v1, v2, v3));
    }
}

// ===========================================================================
// C36 — composed pipeline over the low-level entry points
// ===========================================================================

/// C36: drive the low-level exports in composition (the way `complexmode`
/// composes them internally) but with caller-chosen data, asserting every
/// intermediate value matches.
#[test]
fn c36_composed_pipeline() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC36);

    for _ in 0..600 {
        let a = rng.i32_any();
        let b = rng.i32_any();
        let perms = rng.i32_any() & 0o7777;
        let n = 1 + rng.below(9) as usize;
        let buf: Vec<i32> = (0..n).map(|_| rng.i32_any()).collect();
        let op = rng.cbytes_upto(40);

        // step 1: build a message from each library
        let m_c = drive_crs(c, Some(&op), a);
        let m_r = drive_crs(r, Some(&op), a);
        assert_same("pipeline/create_result_string", m_c.clone(), m_r.clone());
        let msg = m_c.0.clone().expect("non-NULL message");

        // step 2: the multiply_with_log message, then compare the two messages
        // with compare_operations - each library compares its OWN products.
        let l_c = drive_mwl(c, a, b);
        let l_r = drive_mwl(r, a, b);
        assert_same("pipeline/multiply_with_log", l_c.clone(), l_r.clone());
        let log = l_c.0 .1.clone().expect("non-NULL log");

        assert_same(
            "pipeline/compare_operations(msg, log)",
            drive_cmp(c, &msg, &log),
            drive_cmp(r, &msg, &log),
        );
        assert_same(
            "pipeline/compare_operations(log, msg)",
            drive_cmp(c, &log, &msg),
            drive_cmp(r, &log, &msg),
        );

        // step 3: permission gate then the accumulate step
        assert_same(
            "pipeline/safe_add",
            drive_add(c, a, b, perms),
            drive_add(r, a, b, perms),
        );
        let cp_c = unsafe { (c.check_permissions)(perms, READ_PERM | WRITE_PERM) };
        let cp_r = unsafe { (r.check_permissions)(perms, READ_PERM | WRITE_PERM) };
        assert_eq!(cp_c, cp_r, "pipeline/check_permissions({perms:#o})");

        assert_same(
            "pipeline/copy_and_sum",
            drive_cas(c, &buf, n as i32),
            drive_cas(r, &buf, n as i32),
        );

        // step 4: the whole one-shot wrapper, for every mode, with the same data
        for mode in [1, 2, 3, 4] {
            let v3 = buf[0];
            assert_same(
                &format!("pipeline/complexmode({mode}, {a}, {b}, {v3})"),
                drive_mode(c, mode, a, b, v3),
                drive_mode(r, mode, a, b, v3),
            );
        }
    }
}

// ===========================================================================
// C37 — cross-library heap / ABI interoperability
// ===========================================================================

/// C37: a real consumer mixes the two objects. Buffers minted by ONE library
/// must be readable by the other and freeable with the shared libc `free`,
/// which only holds if the translation forwards to the same `malloc`/`free`
/// rather than using Rust's own allocator.
#[test]
fn c37_cross_library_interop() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC37);

    for _ in 0..500 {
        let val = rng.i32_any();
        let op = rng.cbytes_upto(30);
        let cop = cstring(&op);

        unsafe {
            // Buffer from C -> read + strcmp'd by Rust, and vice versa.
            let from_c = (c.create_result_string)(cop.as_ptr(), val);
            let from_r = (r.create_result_string)(cop.as_ptr(), val);
            assert!(!from_c.is_null() && !from_r.is_null());

            let bytes_c = read_cstr(from_c).unwrap();
            let bytes_r = read_cstr(from_r).unwrap();
            assert_eq!(bytes_c, bytes_r, "cross-library buffer contents differ");

            // Each library compares the OTHER library's buffer.
            let cmp_c = capture_stdout(|| (c.compare_operations)(from_r, from_c));
            let cmp_r = capture_stdout(|| (r.compare_operations)(from_c, from_r));
            assert_same("C37 compare_operations across libraries", cmp_c, cmp_r);

            // Free C's buffer through the same libc the Rust side uses, and
            // vice versa: identical allocator or this corrupts the heap.
            cfree(from_c);
            cfree(from_r);

            // multiply_with_log out-param from one library, consumed by the other.
            let (a, b) = (rng.i32_any(), rng.i32_any());
            let mut m_c: *mut c_char = std::ptr::null_mut();
            let mut m_r: *mut c_char = std::ptr::null_mut();
            let rc = (c.multiply_with_log)(a, b, &mut m_c);
            let rr = (r.multiply_with_log)(a, b, &mut m_r);
            assert_eq!(rc, rr, "multiply_with_log({a}, {b}) return");
            assert_eq!(read_cstr(m_c), read_cstr(m_r), "log message differs");
            let x = capture_stdout(|| (r.compare_operations)(m_c, m_r));
            let y = capture_stdout(|| (c.compare_operations)(m_r, m_c));
            assert_eq!(x.0, 0, "the two log messages should be equal");
            assert_eq!(y.0, 0, "the two log messages should be equal");
            assert_eq!(x.1, y.1);
            cfree(m_c);
            cfree(m_r);
        }
    }
}
