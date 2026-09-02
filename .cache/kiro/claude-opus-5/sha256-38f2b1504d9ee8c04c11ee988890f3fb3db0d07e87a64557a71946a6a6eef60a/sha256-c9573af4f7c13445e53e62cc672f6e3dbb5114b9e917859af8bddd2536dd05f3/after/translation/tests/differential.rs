//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through `dlsym` on the two
//! `.so` files (see `tests/support/mod.rs`); the crate under test is never
//! called directly.
//!
//! All randomized rows use a fixed seed so failures are reproducible.

mod support;

use std::ffi::{c_char, c_int};
use support::*;

/// Panic message prefix that names the row and the inputs.
macro_rules! diff {
    ($row:literal, $inputs:expr, $c:expr, $rs:expr) => {
        assert_eq!(
            $c, $rs,
            "CONFIGS.md row {} diverged for inputs {:?}\n  C   = {:?}\n  Rust= {:?}",
            $row, $inputs, $c, $rs
        )
    };
}

// ===========================================================================
// Rows 1-4 — create_buffer / destroy_buffer over the initial_capacity axis.
// ===========================================================================

/// Creates a buffer in both libraries, snapshots it, destroys it, returns the
/// two snapshots. `read_first_byte` controls whether `data[0]` is inspected
/// (it is written even when `capacity == 0`, which is the C's OOB write).
unsafe fn create_destroy(cap: c_int) -> (BufSnapshot, BufSnapshot) {
    let p = pair();
    let cb = (p.c.create_buffer)(cap);
    let cs = snapshot(cb);
    (p.c.destroy_buffer)(cb);

    let rb = (p.rs.create_buffer)(cap);
    let rs = snapshot(rb);
    (p.rs.destroy_buffer)(rb);

    (cs, rs)
}

#[test]
fn row01_create_buffer_capacity_one() {
    unsafe {
        let (c, r) = create_destroy(1);
        diff!(1, 1, c, r);
        assert!(!c.is_null, "capacity 1 should allocate");
        assert_eq!(c.capacity, 1);
        assert_eq!(c.length, 0);
        assert_eq!(c.bytes.as_deref(), Some(&[0u8][..]));
    }
}

#[test]
fn row02_create_buffer_capacity_zero() {
    unsafe {
        // malloc(0) returns a non-NULL minimal chunk; the C then writes
        // data[0] = '\0' one byte out of bounds. Both libraries must agree.
        let (c, r) = create_destroy(0);
        diff!(2, 0, c, r);
        assert_eq!(c.capacity, 0);
        assert_eq!(c.length, 0);
    }
}

#[test]
fn row03_create_buffer_randomized_small_capacities() {
    let mut rng = Rng::new(0x0303_0303);
    unsafe {
        for _ in 0..2000 {
            let cap = rng.range(1, 4096) as c_int;
            let (c, r) = create_destroy(cap);
            diff!(3, cap, c, r);
            assert_eq!(c.capacity, cap);
        }
    }
}

#[test]
fn row04_create_buffer_large_valid_capacities() {
    unsafe {
        for cap in [1 << 12, 1 << 16, 1 << 20, 1 << 24] {
            let (c, r) = create_destroy(cap);
            diff!(4, cap, c, r);
        }
    }
}

// ===========================================================================
// Rows 5-14 — append_to_buffer.
// ===========================================================================

/// Drives one append scenario against both libraries and returns the
/// `(return value, snapshot)` pair for each.
unsafe fn append_once(
    cap: c_int,
    s: &[u8],
    tweak: impl Fn(*mut StringBuffer),
) -> ((c_int, BufSnapshot), (c_int, BufSnapshot)) {
    let p = pair();

    let cb = (p.c.create_buffer)(cap);
    assert!(!cb.is_null());
    tweak(cb);
    let crc = (p.c.append_to_buffer)(cb, s.as_ptr() as *const c_char);
    let cs = snapshot(cb);
    (p.c.destroy_buffer)(cb);

    let rb = (p.rs.create_buffer)(cap);
    assert!(!rb.is_null());
    tweak(rb);
    let rrc = (p.rs.append_to_buffer)(rb, s.as_ptr() as *const c_char);
    let rsn = snapshot(rb);
    (p.rs.destroy_buffer)(rb);

    ((crc, cs), (rrc, rsn))
}

#[test]
fn row05_append_empty_string() {
    unsafe {
        for cap in [0, 1, 2, 8, 32] {
            let (c, r) = append_once(cap, b"\0", |_| {});
            diff!(5, cap, c, r);
            assert_eq!(c.0, 0);
            assert_eq!(c.1.length, 0);
        }
    }
}

#[test]
fn row06_append_fits_without_growth() {
    let mut rng = Rng::new(0x0606_0606);
    unsafe {
        for _ in 0..1500 {
            let cap = rng.range(4, 256) as c_int;
            // strlen + 1 < cap  =>  strlen <= cap - 2
            let len = rng.range(0, (cap - 2) as u32) as usize;
            let s = rng.cstring(len);
            let (c, r) = append_once(cap, &s, |_| {});
            diff!(6, (cap, len), c, r);
            assert_eq!(c.0, 0);
            assert_eq!(c.1.capacity, cap, "no realloc expected");
            assert_eq!(c.1.length, len as c_int);
            assert_eq!(&c.1.bytes.as_ref().unwrap()[..len], &s[..len]);
        }
    }
}

#[test]
fn row07_append_exact_fit_no_growth() {
    let mut rng = Rng::new(0x0707_0707);
    unsafe {
        for _ in 0..800 {
            let cap = rng.range(1, 256) as c_int;
            // strlen + 1 == cap  =>  required_capacity == capacity, not >
            let s = rng.cstring((cap - 1) as usize);
            let (c, r) = append_once(cap, &s, |_| {});
            diff!(7, cap, c, r);
            assert_eq!(c.1.capacity, cap, "exact fit must not realloc");
        }
    }
}

#[test]
fn row08_append_one_byte_over() {
    let mut rng = Rng::new(0x0808_0808);
    unsafe {
        for _ in 0..800 {
            let cap = rng.range(1, 256) as c_int;
            // strlen + 1 == cap + 1  =>  grows to (cap+1)*2
            let s = rng.cstring(cap as usize);
            let (c, r) = append_once(cap, &s, |_| {});
            diff!(8, cap, c, r);
            assert_eq!(c.1.capacity, (cap + 1) * 2);
        }
    }
}

#[test]
fn row09_append_much_longer_than_capacity() {
    let mut rng = Rng::new(0x0909_0909);
    unsafe {
        for _ in 0..600 {
            let cap = rng.range(1, 32) as c_int;
            let len = rng.range(64, 4096) as usize;
            let s = rng.cstring(len);
            let (c, r) = append_once(cap, &s, |_| {});
            diff!(9, (cap, len), c, r);
            assert_eq!(c.1.capacity, (len as c_int + 1) * 2);
            assert_eq!(c.1.length, len as c_int);
        }
    }
}

#[test]
fn row10_append_randomized_sequences_state_checked_each_step() {
    let mut rng = Rng::new(0x1010_1010);
    let p = pair();
    unsafe {
        for _ in 0..300 {
            let cap = rng.range(0, 64) as c_int;
            let n = rng.range(1, 32);
            let strings: Vec<Vec<u8>> = (0..n)
                .map(|_| {
                    let l = rng.range(0, 40) as usize;
                    rng.cstring(l)
                })
                .collect();

            let cb = (p.c.create_buffer)(cap);
            let rb = (p.rs.create_buffer)(cap);
            assert!(!cb.is_null() && !rb.is_null());
            for (step, s) in strings.iter().enumerate() {
                let crc = (p.c.append_to_buffer)(cb, s.as_ptr() as *const c_char);
                let rrc = (p.rs.append_to_buffer)(rb, s.as_ptr() as *const c_char);
                let cs = (crc, snapshot(cb));
                let rsn = (rrc, snapshot(rb));
                diff!(10, (cap, step, s.len() - 1), cs, rsn);
            }
            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row11_append_after_length_forced_to_zero() {
    // Exactly what buffapp does: create, zero the length, then append.
    let mut rng = Rng::new(0x1111_1111);
    let p = pair();
    unsafe {
        for _ in 0..300 {
            let cap = rng.range(1, 64) as c_int;
            let first = rng.cstring_len(1, 50);
            let second = rng.cstring_len(1, 50);

            let cb = (p.c.create_buffer)(cap);
            let rb = (p.rs.create_buffer)(cap);
            (p.c.append_to_buffer)(cb, first.as_ptr() as *const c_char);
            (p.rs.append_to_buffer)(rb, first.as_ptr() as *const c_char);
            (*cb).length = 0;
            (*rb).length = 0;
            let cs = (
                (p.c.append_to_buffer)(cb, second.as_ptr() as *const c_char),
                snapshot(cb),
            );
            let rsn = (
                (p.rs.append_to_buffer)(rb, second.as_ptr() as *const c_char),
                snapshot(rb),
            );
            diff!(11, (cap, first.len(), second.len()), cs, rsn);
            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row12_append_after_length_forced_to_interior_offset() {
    let mut rng = Rng::new(0x1212_1212);
    let p = pair();
    unsafe {
        for _ in 0..300 {
            let first = rng.cstring_len(4, 60);
            let flen = (first.len() - 1) as c_int;
            let second = rng.cstring_len(0, 40);
            let off = rng.range(0, flen as u32) as c_int;
            let cap = 256;

            let cb = (p.c.create_buffer)(cap);
            let rb = (p.rs.create_buffer)(cap);
            (p.c.append_to_buffer)(cb, first.as_ptr() as *const c_char);
            (p.rs.append_to_buffer)(rb, first.as_ptr() as *const c_char);
            (*cb).length = off;
            (*rb).length = off;
            let cs = (
                (p.c.append_to_buffer)(cb, second.as_ptr() as *const c_char),
                snapshot(cb),
            );
            let rsn = (
                (p.rs.append_to_buffer)(rb, second.as_ptr() as *const c_char),
                snapshot(rb),
            );
            diff!(12, (off, first.len(), second.len()), cs, rsn);
            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row13_append_with_capacity_field_understated() {
    // Forcing `capacity` below the real allocation makes the C take the realloc
    // branch even though there is room; both libraries must do the same.
    let mut rng = Rng::new(0x1313_1313);
    unsafe {
        for _ in 0..400 {
            let real_cap = rng.range(64, 512) as c_int;
            let understated = rng.range(0, 8) as c_int;
            let s = rng.cstring_len(0, 32);
            let slen = (s.len() - 1) as c_int;
            let (c, r) = append_once(real_cap, &s, |b| (*b).capacity = understated);
            diff!(13, (real_cap, understated, slen), c, r);
            assert_eq!(c.0, 0);
            let required = slen + 1;
            if required > understated {
                assert_eq!(c.1.capacity, required * 2, "realloc branch taken");
            } else {
                assert_eq!(c.1.capacity, understated, "no realloc expected");
            }
        }
    }
}

#[test]
fn row14_append_high_and_control_bytes() {
    let mut rng = Rng::new(0x1414_1414);
    unsafe {
        for _ in 0..600 {
            let len = rng.range(1, 64) as usize;
            // Only 0x80..=0xff and 0x01..=0x1f — strlen/strcpy are byte-exact.
            let mut s: Vec<u8> = (0..len)
                .map(|_| {
                    if rng.next_u32() % 2 == 0 {
                        rng.range(0x80, 0xff) as u8
                    } else {
                        rng.range(0x01, 0x1f) as u8
                    }
                })
                .collect();
            s.push(0);
            let cap = rng.range(0, 32) as c_int;
            let (c, r) = append_once(cap, &s, |_| {});
            diff!(14, (cap, len), c, r);
            assert_eq!(&c.1.bytes.as_ref().unwrap()[..len], &s[..len]);
        }
    }
}

// ===========================================================================
// Rows 15-16 — get_operation_name.
// ===========================================================================

#[test]
fn row15_get_operation_name_valid_codes() {
    let p = pair();
    unsafe {
        for (code, expect) in [
            (0, &b"add"[..]),
            (1, &b"subtract"[..]),
            (2, &b"multiply"[..]),
            (3, &b"divide"[..]),
        ] {
            let c = cstr_bytes((p.c.get_operation_name)(code));
            let r = cstr_bytes((p.rs.get_operation_name)(code));
            diff!(15, code, c, r);
            assert_eq!(c.as_deref(), Some(expect));
        }
    }
}

#[test]
fn row16_get_operation_name_randomized_full_i32() {
    let mut rng = Rng::new(0x1616_1616);
    let p = pair();
    unsafe {
        let mut codes: Vec<i32> = vec![i32::MIN, i32::MAX, -1, -2, -3, -4, 4, 5, 100, 0, 1, 2, 3];
        for _ in 0..5000 {
            codes.push(rng.interesting_i32());
        }
        for code in codes {
            let c = cstr_bytes((p.c.get_operation_name)(code));
            let r = cstr_bytes((p.rs.get_operation_name)(code));
            diff!(16, code, c, r);
        }
    }
}

// ===========================================================================
// Rows 17-24 — perform_operation.
// ===========================================================================

const OPS: [&[u8]; 5] = [
    b"add\0",
    b"subtract\0",
    b"multiply\0",
    b"divide\0",
    b"unknown\0",
];

unsafe fn perform_both(a: c_int, b: c_int, op: &[u8]) -> (c_int, c_int) {
    let p = pair();
    let ptr = op.as_ptr() as *const c_char;
    ((p.c.perform_operation)(a, b, ptr), (p.rs.perform_operation)(a, b, ptr))
}

/// `INT_MIN / -1` is UB in C and traps; it is covered out-of-process by the
/// Phase C tests, so in-process randomized rows must not generate it.
fn is_trapping_div(op: &[u8], a: c_int, b: c_int) -> bool {
    op == b"divide\0" && a == i32::MIN && b == -1
}

fn randomized_op_row(row: u32, seed: u64, op: &'static [u8]) {
    let mut rng = Rng::new(seed);
    unsafe {
        let mut n = 0;
        while n < 5000 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            if is_trapping_div(op, a, b) {
                continue;
            }
            let (c, r) = perform_both(a, b, op);
            assert_eq!(
                c, r,
                "CONFIGS.md row {row} diverged for op={:?} a={a} b={b}: C={c} Rust={r}",
                std::str::from_utf8(&op[..op.len() - 1]).unwrap()
            );
            n += 1;
        }
    }
}

#[test]
fn row17_perform_operation_add() {
    randomized_op_row(17, 0x1717_1717, b"add\0");
}

#[test]
fn row18_perform_operation_subtract() {
    randomized_op_row(18, 0x1818_1818, b"subtract\0");
}

#[test]
fn row19_perform_operation_multiply() {
    randomized_op_row(19, 0x1919_1919, b"multiply\0");
}

#[test]
fn row20_perform_operation_divide_nonzero_divisor() {
    let mut rng = Rng::new(0x2020_2020);
    unsafe {
        let mut n = 0;
        while n < 5000 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            if b == 0 || is_trapping_div(b"divide\0", a, b) {
                continue;
            }
            let (c, r) = perform_both(a, b, b"divide\0");
            diff!(20, (a, b), c, r);
            n += 1;
        }
    }
}

#[test]
fn row21_perform_operation_divide_by_zero() {
    let mut rng = Rng::new(0x2121_2121);
    unsafe {
        for _ in 0..2000 {
            let a = rng.interesting_i32();
            let (c, r) = perform_both(a, 0, b"divide\0");
            diff!(21, a, c, r);
            assert_eq!(c, 0, "divide by zero returns 0");
        }
    }
}

#[test]
fn row22_perform_operation_with_names_from_get_operation_name() {
    // Composition: the operation pointer is whatever get_operation_name hands
    // back, including "unknown" for out-of-range codes.
    let mut rng = Rng::new(0x2222_2222);
    let p = pair();
    unsafe {
        let mut n = 0;
        while n < 6000 {
            let code = rng.interesting_i32();
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let c_op = (p.c.get_operation_name)(code);
            let r_op = (p.rs.get_operation_name)(code);
            // Feed each library its own string, and also cross-feed, to prove
            // the returned bytes are interchangeable.
            if cstr_bytes(c_op).as_deref() == Some(b"divide") && a == i32::MIN && b == -1 {
                continue;
            }
            let c = (p.c.perform_operation)(a, b, c_op);
            let r = (p.rs.perform_operation)(a, b, r_op);
            diff!(22, (code, a, b), c, r);
            let cross_c = (p.c.perform_operation)(a, b, r_op);
            let cross_r = (p.rs.perform_operation)(a, b, c_op);
            diff!(22, (code, a, b, "crossed"), cross_c, cross_r);
            assert_eq!(c, cross_c, "row 22: C must accept the Rust string");
            n += 1;
        }
    }
}

#[test]
fn row23_perform_operation_non_matching_operation_strings() {
    let mut rng = Rng::new(0x2323_2323);
    unsafe {
        // Hand-picked near-misses first.
        for op in [
            &b"\0"[..],
            b"ADD\0",
            b"add \0",
            b" add\0",
            b"addx\0",
            b"ad\0",
            b"Add\0",
            b"subtrac\0",
            b"subtracts\0",
            b"multiplyy\0",
            b"divid\0",
            b"divides\0",
            b"unknown\0",
            b"\x01\x02\x03\0",
            b"\xff\xfe\0",
        ] {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let (c, r) = perform_both(a, b, op);
            diff!(23, (op, a, b), c, r);
            assert_eq!(c, 0, "non-matching operation must yield 0");
        }
        // Then random byte strings.
        for _ in 0..3000 {
            let len = rng.range(0, 16) as usize;
            let s = rng.cstring(len);
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let (c, r) = perform_both(a, b, &s);
            diff!(23, (len, a, b), c, r);
        }
    }
}

#[test]
fn row24_perform_operation_small_exhaustive_grid() {
    unsafe {
        for op in OPS {
            for a in -8i32..=8 {
                for b in -8i32..=8 {
                    let (c, r) = perform_both(a, b, op);
                    diff!(24, (op, a, b), c, r);
                }
            }
        }
        // And the documented overflow corners (ERRORS.md row 16).
        for (op, a, b, want) in [
            (&b"add\0"[..], i32::MAX, 1, i32::MIN),
            (b"subtract\0", i32::MIN, 1, i32::MAX),
            (b"multiply\0", i32::MIN, -1, i32::MIN),
            (b"multiply\0", i32::MAX, i32::MAX, 1),
            (b"add\0", i32::MIN, -1, i32::MAX),
        ] {
            let (c, r) = perform_both(a, b, op);
            diff!(24, (op, a, b), c, r);
            assert_eq!(c, want, "C wrapping result for {a} {b}");
        }
    }
}
