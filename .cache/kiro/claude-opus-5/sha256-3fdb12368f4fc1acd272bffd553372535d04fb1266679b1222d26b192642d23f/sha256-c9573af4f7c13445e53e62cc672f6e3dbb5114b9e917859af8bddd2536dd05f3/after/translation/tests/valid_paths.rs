//! Phase B — valid-path differential tests.
//!
//! One test per group of rows in `CONFIGS.md`. Every call goes through the
//! `.so` exports of BOTH implementations (loaded with `libloading`), never
//! through Rust functions directly, and every heap-sensitive row is run under
//! both allocator orderings (see `common::seed_heap`).

mod common;

use common::{Heap, Impl, Pair, Rng, HEAP_STATES};
use std::ffi::{c_char, c_int};

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// Rows 1-5: apply_bitmask
// ---------------------------------------------------------------------------

#[test]
fn valid_apply_bitmask_all_operations() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED);
    for operation in [0, 1, 2, 3] {
        for _ in 0..512 {
            let value = rng.interesting_i32();
            let c = unsafe { (p.c.apply_bitmask)(value, operation) };
            let r = unsafe { (p.rust.apply_bitmask)(value, operation) };
            assert_eq!(c, r, "apply_bitmask({value}, {operation}): C={c} Rust={r}");
        }
        // Exhaustive low-byte sweep: every mask interaction with every low byte.
        for value in 0..=255i32 {
            for hi in [0i32, -256, 0x7fff_ff00u32 as i32, i32::MIN] {
                let v = hi | value;
                let c = unsafe { (p.c.apply_bitmask)(v, operation) };
                let r = unsafe { (p.rust.apply_bitmask)(v, operation) };
                assert_eq!(c, r, "apply_bitmask({v}, {operation})");
            }
        }
    }
}

#[test]
fn valid_apply_bitmask_random_pairs() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xAA);
    for _ in 0..20_000 {
        let value = rng.next_i32();
        let operation = match rng.next_u64() % 2 {
            0 => rng.range_i32(-8, 8),
            _ => rng.next_i32(),
        };
        let c = unsafe { (p.c.apply_bitmask)(value, operation) };
        let r = unsafe { (p.rust.apply_bitmask)(value, operation) };
        assert_eq!(c, r, "apply_bitmask({value}, {operation})");
    }
}

// ---------------------------------------------------------------------------
// Rows 6-8: process_string
// ---------------------------------------------------------------------------

#[test]
fn valid_process_string_single_byte_all_values() {
    let p = common::load_pair();
    for b in 1u16..=255 {
        let buf: [c_char; 2] = [b as u8 as c_char, 0];
        let c = unsafe { (p.c.process_string)(buf.as_ptr()) };
        let r = unsafe { (p.rust.process_string)(buf.as_ptr()) };
        assert_eq!(c, r, "process_string(byte {b:#04x}): C={c} Rust={r}");
        assert_eq!(c, 1, "sanity: single non-NUL byte has strlen 1");
    }
}

#[test]
fn valid_process_string_random_lengths() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xBB);
    let mut buf = vec![0u8; 300];
    for _ in 0..3000 {
        let len = (rng.next_u64() % 255 + 1) as usize;
        for i in 0..len {
            // Any non-NUL byte, including >= 0x80 to exercise char signedness.
            buf[i] = ((rng.next_u64() % 255) + 1) as u8;
        }
        buf[len] = 0;
        let ptr = buf.as_ptr() as *const c_char;
        let c = unsafe { (p.c.process_string)(ptr) };
        let r = unsafe { (p.rust.process_string)(ptr) };
        assert_eq!(c, r, "process_string(len {len})");
        assert_eq!(c, len as c_int);
    }
}

#[test]
fn valid_process_string_boundary_lengths() {
    let p = common::load_pair();
    for len in [1usize, 2, 5, 63, 64, 255, 4096] {
        for fill in [b'A', 0x01, 0x7F, 0x80, 0xFF] {
            let mut buf = vec![fill; len + 1];
            buf[len] = 0;
            let ptr = buf.as_ptr() as *const c_char;
            let c = unsafe { (p.c.process_string)(ptr) };
            let r = unsafe { (p.rust.process_string)(ptr) };
            assert_eq!(c, r, "process_string(len {len}, fill {fill:#04x})");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 9-11: shift_array
// ---------------------------------------------------------------------------

/// Run `shift_array` on identical copies through both `.so`s and compare the
/// resulting buffers byte for byte.
#[track_caller]
fn shift_both(p: &Pair, data: &[c_int], size: c_int, positions: c_int) {
    let mut a = data.to_vec();
    let mut b = data.to_vec();
    unsafe { (p.c.shift_array)(a.as_mut_ptr(), size, positions) };
    unsafe { (p.rust.shift_array)(b.as_mut_ptr(), size, positions) };
    assert_eq!(
        a, b,
        "shift_array(size={size}, positions={positions}) on {data:?}: C={a:?} Rust={b:?}"
    );
}

#[test]
fn valid_shift_array_arity4_config() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xCC);
    for _ in 0..5000 {
        let data = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        shift_both(&p, &data, 4, 1);
    }
}

#[test]
fn valid_shift_array_all_sizes_and_positions() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xDD);
    for size in [2usize, 3, 4, 5, 8, 64, 1024] {
        for positions in 1..size {
            for _ in 0..8 {
                let data: Vec<c_int> = (0..size).map(|_| rng.interesting_i32()).collect();
                shift_both(&p, &data, size as c_int, positions as c_int);
            }
        }
    }
}

#[test]
fn valid_shift_array_random_with_guards() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xEE);
    const GUARD: c_int = 0x5A5A_5A5A;
    for _ in 0..2000 {
        let size = (rng.next_u64() % 255 + 2) as usize;
        let positions = (rng.next_u64() % (size as u64 - 1) + 1) as c_int;
        // 8 guard ints on each side of the working window.
        let mut a = vec![GUARD; size + 16];
        for i in 0..size {
            a[8 + i] = rng.interesting_i32();
        }
        let mut b = a.clone();
        unsafe { (p.c.shift_array)(a[8..].as_mut_ptr(), size as c_int, positions) };
        unsafe { (p.rust.shift_array)(b[8..].as_mut_ptr(), size as c_int, positions) };
        assert_eq!(a, b, "shift_array(size={size}, positions={positions})");
        for i in 0..8 {
            assert_eq!(a[i], GUARD, "C wrote before the buffer");
            assert_eq!(a[8 + size + i], GUARD, "C wrote past the buffer");
            assert_eq!(b[i], GUARD, "Rust wrote before the buffer");
            assert_eq!(b[8 + size + i], GUARD, "Rust wrote past the buffer");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 12-13: init_matrix
// ---------------------------------------------------------------------------

#[test]
fn valid_init_matrix_exact_buffer() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..2000 {
        let mut a = [0i32; 12];
        for v in a.iter_mut() {
            *v = rng.next_i32();
        }
        let mut b = a;
        unsafe { (p.c.init_matrix)(a.as_mut_ptr()) };
        unsafe { (p.rust.init_matrix)(b.as_mut_ptr()) };
        assert_eq!(a, b, "init_matrix: C={a:?} Rust={b:?}");
        assert_eq!(a, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }
}

#[test]
fn valid_init_matrix_guarded_window() {
    let p = common::load_pair();
    const GUARD: c_int = -0x3C3C_3C3C;
    let mut a = [GUARD; 12 + 16];
    let mut b = a;
    unsafe { (p.c.init_matrix)(a[8..].as_mut_ptr()) };
    unsafe { (p.rust.init_matrix)(b[8..].as_mut_ptr()) };
    assert_eq!(a, b, "init_matrix in a window: C={a:?} Rust={b:?}");
    for i in 0..8 {
        assert_eq!(a[i], GUARD);
        assert_eq!(a[20 + i], GUARD);
    }
    assert_eq!(&a[8..20], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
}

// ---------------------------------------------------------------------------
// Rows 14-18: compare_allocations
// ---------------------------------------------------------------------------

#[test]
fn valid_compare_allocations_matrix() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x22);
    let val1_classes: [&[c_int]; 3] = [
        &[1, 2, 7, 100, i32::MAX, i32::MAX - 1],       // val1 > 0  -> +10
        &[0],                                          // val1 == 0 -> no bonus
        &[-1, -2, -100, i32::MIN, i32::MIN + 1],       // val1 < 0  -> no bonus
    ];
    for class in val1_classes {
        for &val1 in class {
            for _ in 0..64 {
                let val2 = rng.interesting_i32();
                for order in HEAP_STATES {
                    let (c, r) = common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                        (imp.compare_allocations)(val1, val2)
                    });
                    assert_eq!(
                        c, r,
                        "compare_allocations({val1}, {val2}) [heap={order:?}]: C={c} Rust={r}"
                    );
                    // Cross-check against the ordering we forced.
                    let expected_base = match order {
                        Heap::Ascending => 1,
                        Heap::Descending => 2,
                    };
                    let bonus = if val1 > 0 { 10 } else { 0 };
                    assert_eq!(
                        c,
                        expected_base + bonus,
                        "forced heap ordering not observed for val1={val1}"
                    );
                }
            }
        }
    }
}

#[test]
fn valid_compare_allocations_random() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x33);
    for _ in 0..4000 {
        let val1 = rng.interesting_i32();
        let val2 = rng.interesting_i32();
        common::assert_both_heaps(&p, "compare_allocations random", |imp| unsafe {
            (imp.compare_allocations)(val1, val2)
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 19-34: arity4
// ---------------------------------------------------------------------------

/// Build a `param1` with the requested `param1 % 4` value (C truncating
/// remainder, so negative results come from negative `param1`).
fn param1_with_mod(rng: &mut Rng, m: c_int) -> c_int {
    let k = rng.range_i32(0, 500) as i64;
    let v = match m {
        0 => {
            if rng.next_u64() % 2 == 0 {
                4 * k
            } else {
                -4 * k
            }
        }
        1..=3 => 4 * k + m as i64,
        -3..=-1 => -4 * k + m as i64,
        _ => unreachable!(),
    };
    v as c_int
}

const MODS: [c_int; 7] = [0, 1, 2, 3, -1, -2, -3];

#[test]
fn valid_arity4_mod_and_switch_matrix() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x44);
    // param3 / param4 classes: off, on-positive, on-negative.
    let p3_classes: [Option<bool>; 3] = [None, Some(true), Some(false)];
    let p4_classes: [Option<bool>; 3] = [None, Some(true), Some(false)];
    for m in MODS {
        for p3c in p3_classes {
            for p4c in p4_classes {
                for _ in 0..40 {
                    let param1 = param1_with_mod(&mut rng, m);
                    let param2 = rng.interesting_i32();
                    let param3 = match p3c {
                        None => 0,
                        Some(true) => rng.range_i32(1, 10_000),
                        Some(false) => rng.range_i32(-10_000, -1),
                    };
                    let param4 = match p4c {
                        None => 0,
                        Some(true) => rng.range_i32(1, 1_000_000),
                        Some(false) => rng.range_i32(-1_000_000, -1),
                    };
                    common::assert_both_heaps(
                        &p,
                        "arity4 matrix",
                        |imp| unsafe { (imp.arity4)(param1, param2, param3, param4) },
                    );
                }
            }
        }
    }
    // param1 == 0 exactly: mod 0 but no compare_allocations bonus (row 26).
    for _ in 0..200 {
        let param2 = rng.interesting_i32();
        let param3 = rng.range_i32(-1000, 1000);
        let param4 = rng.range_i32(-1000, 1000);
        common::assert_both_heaps(&p, "arity4 param1==0", |imp| unsafe {
            (imp.arity4)(0, param2, param3, param4)
        });
    }
}

#[test]
fn valid_arity4_random_small() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x55);
    for _ in 0..3000 {
        let a = rng.small_i32();
        let b = rng.small_i32();
        let c = rng.small_i32();
        let d = rng.small_i32();
        common::assert_both_heaps(&p, "arity4 small", |imp| unsafe {
            (imp.arity4)(a, b, c, d)
        });
    }
}

#[test]
fn valid_arity4_random_full_range() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x66);
    for _ in 0..5000 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let c = rng.next_i32();
        let d = rng.next_i32();
        common::assert_both_heaps(&p, "arity4 full range", |imp| unsafe {
            (imp.arity4)(a, b, c, d)
        });
    }
}

#[test]
fn valid_arity4_boundary_pool() {
    let p = common::load_pair();
    const POOL: [c_int; 15] = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        4,
        -4,
        99,
        -99,
        100,
        -100,
        i32::MAX,
        i32::MIN,
    ];
    // Full cross product over the first two params, pool over the last two.
    for &a in POOL.iter() {
        for &b in POOL.iter() {
            for &c in POOL.iter() {
                for d in [0, 1, -1, i32::MAX, i32::MIN] {
                    common::assert_both_heaps(&p, "arity4 boundary pool", |imp| unsafe {
                        (imp.arity4)(a, b, c, d)
                    });
                }
            }
        }
    }
}

#[test]
fn valid_arity4_param3_zero_vs_nonzero() {
    // ERRORS.md row 29: `param3 == 0` skips the `(result * param3) / 100`
    // statement entirely, which is the only thing preventing the `* 0` collapse.
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xC1);
    for _ in 0..1500 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let d = rng.interesting_i32();
        common::assert_both_heaps(&p, "arity4 param3==0", |imp| unsafe {
            (imp.arity4)(a, b, 0, d)
        });
        // And the adjacent non-zero values on both sides of the branch.
        for p3 in [1, -1, 2, -2, 100, -100, i32::MAX, i32::MIN] {
            common::assert_both_heaps(&p, "arity4 param3!=0", |imp| unsafe {
                (imp.arity4)(a, b, p3, d)
            });
        }
    }
}

#[test]
fn valid_arity4_param4_zero_vs_nonzero() {
    // ERRORS.md row 30: `param4 == 0` skips `result += param4`.
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xC2);
    for _ in 0..1500 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let c = rng.interesting_i32();
        common::assert_both_heaps(&p, "arity4 param4==0", |imp| unsafe {
            (imp.arity4)(a, b, c, 0)
        });
        for p4 in [1, -1, 2, -2, 100, -100, i32::MAX, i32::MIN] {
            common::assert_both_heaps(&p, "arity4 param4!=0", |imp| unsafe {
                (imp.arity4)(a, b, c, p4)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 35-36: arity3 / arity2
// ---------------------------------------------------------------------------

#[test]
fn valid_arity3_matrix() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x77);
    for m in MODS {
        for p3c in [None, Some(true), Some(false)] {
            for _ in 0..60 {
                let param1 = param1_with_mod(&mut rng, m);
                let param2 = rng.interesting_i32();
                let param3 = match p3c {
                    None => 0,
                    Some(true) => rng.range_i32(1, 10_000),
                    Some(false) => rng.range_i32(-10_000, -1),
                };
                common::assert_both_heaps(&p, "arity3 matrix", |imp| unsafe {
                    (imp.arity3)(param1, param2, param3)
                });
            }
        }
    }
    for _ in 0..2000 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let c = rng.next_i32();
        common::assert_both_heaps(&p, "arity3 random", |imp| unsafe {
            (imp.arity3)(a, b, c)
        });
    }
    // arity3(a,b,c) must equal arity4(a,b,c,0) on both sides.
    for _ in 0..500 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let c = rng.interesting_i32();
        for order in HEAP_STATES {
            let (c3, r3) =
                common::run_seeded(&p, order, &|imp: &Impl| unsafe { (imp.arity3)(a, b, c) });
            let (c4, r4) = common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                (imp.arity4)(a, b, c, 0)
            });
            assert_eq!(c3, c4, "C: arity3({a},{b},{c}) != arity4({a},{b},{c},0)");
            assert_eq!(r3, r4, "Rust: arity3 != arity4(..,0)");
            assert_eq!(c3, r3);
        }
    }
}

#[test]
fn valid_arity2_matrix() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x88);
    for m in MODS {
        for _ in 0..120 {
            let param1 = param1_with_mod(&mut rng, m);
            let param2 = rng.interesting_i32();
            common::assert_both_heaps(&p, "arity2 matrix", |imp| unsafe {
                (imp.arity2)(param1, param2)
            });
        }
    }
    for _ in 0..2000 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        common::assert_both_heaps(&p, "arity2 random", |imp| unsafe { (imp.arity2)(a, b) });
    }
    for _ in 0..500 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        for order in HEAP_STATES {
            let (c2, r2) = common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                (imp.arity2)(a, b)
            });
            let (c4, r4) = common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                (imp.arity4)(a, b, 0, 0)
            });
            assert_eq!(c2, c4, "C: arity2({a},{b}) != arity4({a},{b},0,0)");
            assert_eq!(r2, r4, "Rust: arity2 != arity4(..,0,0)");
            assert_eq!(c2, r2);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 37-42: arity dispatcher
// ---------------------------------------------------------------------------

/// `arity` takes `int *params` (non-const in the header), so use a mutable
/// buffer. `GARBAGE` fills the slots the given `len` must not read.
const GARBAGE: c_int = 0x1BAD_C0DEu32 as c_int;

#[track_caller]
fn arity_both(p: &Pair, len: c_int, params: &[c_int; 4]) {
    for order in HEAP_STATES {
        // Fixed-size arrays, not Vecs: a 4-element `Vec<i32>` is a 32-byte chunk,
        // i.e. the same tcache bin the library uses, so avoiding it keeps the
        // harness out of the measurement entirely.
        let mut a = *params;
        let mut b = *params;
        common::seed_heap(order);
        let rc = unsafe { (p.c.arity)(len, a.as_mut_ptr()) };
        common::seed_heap(order);
        let rr = unsafe { (p.rust.arity)(len, b.as_mut_ptr()) };
        assert_eq!(
            rc, rr,
            "arity({len}, {params:?}) [heap={order:?}]: C={rc} Rust={rr}"
        );
        assert_eq!(a, b, "arity({len}) mutated the params buffer differently");
        assert_eq!(&a, params, "arity({len}) must not mutate params at all");
    }
}

#[test]
fn valid_arity_dispatch_len2() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x99);
    for _ in 0..1500 {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            GARBAGE,
            GARBAGE,
        ];
        arity_both(&p, 2, &params);
        // Must equal arity2 of the first two elements.
        for order in HEAP_STATES {
            let mut buf = params;
            common::seed_heap(order);
            let via_arity = unsafe { (p.c.arity)(2, buf.as_mut_ptr()) };
            common::seed_heap(order);
            let via_arity2 = unsafe { (p.c.arity2)(params[0], params[1]) };
            assert_eq!(via_arity, via_arity2, "C: arity(2, ..) != arity2(..)");
            common::seed_heap(order);
            let r_arity = unsafe { (p.rust.arity)(2, buf.as_mut_ptr()) };
            assert_eq!(r_arity, via_arity, "Rust arity(2, ..) diverged");
        }
    }
}

#[test]
fn valid_arity_dispatch_len3() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xA1);
    for _ in 0..1500 {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            GARBAGE,
        ];
        arity_both(&p, 3, &params);
    }
}

#[test]
fn valid_arity_dispatch_len4() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xA2);
    for _ in 0..1500 {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        arity_both(&p, 4, &params);
    }
}

#[test]
fn valid_arity_dispatch_len_above_four() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xA3);
    for len in [5i32, 6, 7, 8, 100, 127, 128, 200, 254, 255] {
        for _ in 0..100 {
            let params = [
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ];
            arity_both(&p, len, &params);
        }
    }
}

#[test]
fn valid_arity_full_len_sweep() {
    let p = common::load_pair();
    let params: [c_int; 4] = [7, -13, 5, 21];
    for len in 0..=255i32 {
        arity_both(&p, len, &params);
    }
}

#[test]
fn valid_arity_random_end_to_end() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xA4);
    for _ in 0..4000 {
        let len = match rng.next_u64() % 3 {
            0 => rng.range_i32(0, 6),
            1 => rng.range_i32(0, 255),
            _ => rng.next_i32(),
        };
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        arity_both(&p, len, &params);
    }
}

// ---------------------------------------------------------------------------
// Rows 43-44: composed pipeline, driven from the low-level entry points
// ---------------------------------------------------------------------------

/// Re-implement `arity4`'s body using ONLY the exported low-level symbols of one
/// implementation, in the C's exact order, and check it reproduces that same
/// implementation's `arity4`. This exercises the composed pipeline rather than
/// each wrapper in isolation.
fn manual_arity4(imp: &Impl, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut values: [c_int; 4] = [p1, p2, p3, p4];

    let test_str: [c_char; 6] = [
        b'H' as c_char,
        b'e' as c_char,
        b'l' as c_char,
        b'l' as c_char,
        b'o' as c_char,
        0,
    ];
    let empty_str: [c_char; 1] = [0];
    let len1 = unsafe { (imp.process_string)(test_str.as_ptr()) };
    let len2 = unsafe { (imp.process_string)(empty_str.as_ptr()) };
    result = result.wrapping_add(len1.wrapping_add(len2));

    unsafe { (imp.shift_array)(values.as_mut_ptr(), 4, 1) };
    for v in values {
        result = result.wrapping_add(v);
    }

    result = unsafe { (imp.apply_bitmask)(result, p1 % 4) };

    let mut matrix = [0i32; 12];
    unsafe { (imp.init_matrix)(matrix.as_mut_ptr()) };
    result = result.wrapping_add(matrix[0].wrapping_add(matrix[11]));

    let alloc = unsafe { (imp.compare_allocations)(p1, p2) };
    result = result.wrapping_add(alloc);

    if p3 != 0 {
        result = result.wrapping_mul(p3).wrapping_div(100);
    }
    if p4 != 0 {
        result = result.wrapping_add(p4);
    }
    result
}

#[test]
fn valid_manual_pipeline_matches_arity4() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xB1);
    for _ in 0..2000 {
        let (a, b, c, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        for order in HEAP_STATES {
            // manual_arity4 performs exactly one allocating call
            // (compare_allocations), so a single seeding is enough.
            common::seed_heap(order);
            let man_c = manual_arity4(&p.c, a, b, c, d);
            common::seed_heap(order);
            let real_c = unsafe { (p.c.arity4)(a, b, c, d) };
            common::seed_heap(order);
            let man_r = manual_arity4(&p.rust, a, b, c, d);
            common::seed_heap(order);
            let real_r = unsafe { (p.rust.arity4)(a, b, c, d) };
            assert_eq!(
                man_c, real_c,
                "C: hand-composed pipeline != arity4({a},{b},{c},{d}) [heap={order:?}]"
            );
            assert_eq!(man_r, real_r, "Rust: hand-composed pipeline != arity4");
            assert_eq!(man_c, man_r, "C vs Rust hand-composed pipeline diverged");
            assert_eq!(real_c, real_r, "C vs Rust arity4 diverged");
        }
    }
}

#[test]
fn valid_cross_impl_pipeline() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0xB2);
    // Mix helpers from the two implementations inside one pipeline: any
    // behavioral difference in an intermediate step shows up in the result.
    for _ in 0..1000 {
        let vals: [c_int; 8] = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        for positions in 1..8i32 {
            let mut a = vals;
            let mut b = vals;
            // Shift with C, mask with Rust.
            unsafe { (p.c.shift_array)(a.as_mut_ptr(), 8, positions) };
            unsafe { (p.rust.shift_array)(b.as_mut_ptr(), 8, positions) };
            assert_eq!(a, b);
            let mut acc_c: c_int = 0;
            let mut acc_r: c_int = 0;
            for i in 0..8 {
                acc_c = unsafe { (p.rust.apply_bitmask)(acc_c.wrapping_add(a[i]), i as c_int % 4) };
                acc_r = unsafe { (p.c.apply_bitmask)(acc_r.wrapping_add(b[i]), i as c_int % 4) };
            }
            assert_eq!(acc_c, acc_r, "cross-impl pipeline diverged");
        }
    }
}
