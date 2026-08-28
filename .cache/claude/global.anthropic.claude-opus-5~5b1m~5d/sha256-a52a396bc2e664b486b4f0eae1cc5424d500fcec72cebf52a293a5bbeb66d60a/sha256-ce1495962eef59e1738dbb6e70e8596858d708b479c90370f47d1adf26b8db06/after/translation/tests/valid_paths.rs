// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Both libraries are driven only through their `.so` exports. Every row uses
// many randomized inputs with a fixed seed. Lowest-level entry points are
// exercised directly, not just through `hatch`.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Hidden-state configurations (axis A of CONFIGS.md).
const STATES: [(c_int, c_int); 6] = [
    (0, 0),                          // fresh
    (12345, 0),                      // counter only
    (0, -98765),                     // accumulator only
    (777, 31337),                    // both set
    (c_int::MAX, c_int::MIN),        // wrapped extremes
    (c_int::MIN, c_int::MAX),        // wrapped extremes, swapped
];

// ---------------------------------------------------------------------------
// C1 / C2 — pure arithmetic leaves
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_add_three_random() {
    let l = libs();
    let mut rng = Rng::new(0xC1_0000_0001);
    for &e0 in EXTREMES.iter() {
        for &e1 in EXTREMES.iter() {
            for &e2 in EXTREMES.iter() {
                let (c, r) = unsafe { ((l.c.add_three)(e0, e1, e2), (l.r.add_three)(e0, e1, e2)) };
                assert_eq!(c, r, "add_three({e0},{e1},{e2})");
            }
        }
    }
    for _ in 0..5000 {
        let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
        let (c, r) = unsafe { ((l.c.add_three)(a, b, c3), (l.r.add_three)(a, b, c3)) };
        assert_eq!(c, r, "add_three({a},{b},{c3})");
    }
}

#[test]
fn cfg_c2_multiply_add_random() {
    let l = libs();
    let mut rng = Rng::new(0xC2_0000_0002);
    for &e0 in EXTREMES.iter() {
        for &e1 in EXTREMES.iter() {
            for &e2 in EXTREMES.iter() {
                let (c, r) =
                    unsafe { ((l.c.multiply_add)(e0, e1, e2), (l.r.multiply_add)(e0, e1, e2)) };
                assert_eq!(c, r, "multiply_add({e0},{e1},{e2})");
            }
        }
    }
    for _ in 0..5000 {
        let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
        let (c, r) = unsafe { ((l.c.multiply_add)(a, b, c3), (l.r.multiply_add)(a, b, c3)) };
        assert_eq!(c, r, "multiply_add({a},{b},{c3})");
    }
}

// ---------------------------------------------------------------------------
// C3 / C4 — the two state mutators, as long randomized sequences
// ---------------------------------------------------------------------------

#[test]
fn cfg_c3_increment_counter_sequence() {
    let l = libs();
    l.set_state(0, 0);
    let mut rng = Rng::new(0xC3_0000_0003);
    for step in 0..2000 {
        let v = rng.interesting();
        let junk = rng.i32_full();
        unsafe {
            (l.c.increment_counter)(v, junk);
            (l.r.increment_counter)(v, junk);
        }
        assert_eq!(
            l.c.read_counter(),
            l.r.read_counter(),
            "global_counter diverged at step {step} (value {v})"
        );
    }
    l.reset();
}

#[test]
fn cfg_c4_update_accumulator_sequence() {
    let l = libs();
    l.set_state(0, 0);
    let mut rng = Rng::new(0xC4_0000_0004);
    for step in 0..2000 {
        let v = rng.interesting();
        let junk = rng.i32_full();
        unsafe {
            (l.c.update_accumulator)(v, junk);
            (l.r.update_accumulator)(v, junk);
        }
        assert_eq!(
            l.c.read_accumulator(),
            l.r.read_accumulator(),
            "global_accumulator diverged at step {step} (value {v})"
        );
    }
    l.reset();
}

// ---------------------------------------------------------------------------
// C5 / C6 / C7 — state readers
// ---------------------------------------------------------------------------

#[test]
fn cfg_c5_complex_calc_vs_state() {
    let l = libs();
    let mut rng = Rng::new(0xC5_0000_0005);
    for &(gc, ga) in STATES.iter() {
        l.set_state(gc, ga);
        for &e0 in EXTREMES.iter() {
            for &e1 in EXTREMES.iter() {
                for &e2 in EXTREMES.iter() {
                    let (c, r) = unsafe {
                        ((l.c.complex_calc)(e0, e1, e2), (l.r.complex_calc)(e0, e1, e2))
                    };
                    assert_eq!(c, r, "complex_calc({e0},{e1},{e2}) state=({gc},{ga})");
                }
            }
        }
        for _ in 0..1000 {
            let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
            let (c, r) =
                unsafe { ((l.c.complex_calc)(a, b, c3), (l.r.complex_calc)(a, b, c3)) };
            assert_eq!(c, r, "complex_calc({a},{b},{c3}) state=({gc},{ga})");
        }
    }
    l.reset();
}

#[test]
fn cfg_c6_process_pointer_vs_state() {
    let l = libs();
    let mut rng = Rng::new(0xC6_0000_0006);
    for &(gc, ga) in STATES.iter() {
        l.set_state(gc, ga);
        for &v in EXTREMES.iter() {
            for &m in EXTREMES.iter() {
                let mut cv = v;
                let mut rv = v;
                let (c, r) = unsafe {
                    (
                        (l.c.process_pointer_data)(&mut cv, m),
                        (l.r.process_pointer_data)(&mut rv, m),
                    )
                };
                assert_eq!(c, r, "process_pointer_data(*{v},{m}) state=({gc},{ga})");
                assert_eq!(cv, rv, "input int must not be modified");
                assert_eq!(cv, v, "input int must not be modified");
            }
        }
        for _ in 0..1000 {
            let v = rng.interesting();
            let m = rng.interesting();
            let mut cv = v;
            let mut rv = v;
            let (c, r) = unsafe {
                (
                    (l.c.process_pointer_data)(&mut cv, m),
                    (l.r.process_pointer_data)(&mut rv, m),
                )
            };
            assert_eq!(c, r, "process_pointer_data(*{v},{m}) state=({gc},{ga})");
            assert_eq!((cv, rv), (v, v));
        }
    }
    l.reset();
}

#[test]
fn cfg_c7_process_pointer_interior() {
    let l = libs();
    l.set_state(4242, -777);
    let mut rng = Rng::new(0xC7_0000_0007);
    for _ in 0..2000 {
        let len = rng.range(1, 64) as usize;
        let mut cbuf = rand_int_buf(&mut rng, len, 0);
        let mut rbuf = cbuf.clone();
        let k = rng.range(0, len as i64 - 1) as usize;
        let m = rng.interesting();
        let (c, r) = unsafe {
            (
                (l.c.process_pointer_data)(cbuf.as_mut_ptr().add(k), m),
                (l.r.process_pointer_data)(rbuf.as_mut_ptr().add(k), m),
            )
        };
        assert_eq!(c, r, "process_pointer_data(&buf[{k}], {m}) len={len}");
        assert_bytes_eq(bytes_of(&cbuf), bytes_of(&rbuf), "buffer untouched");
    }
    l.reset();
}

// ---------------------------------------------------------------------------
// C8 / C9 / C10 / C11 — apply_operation, the library's only "mode" selector
// ---------------------------------------------------------------------------

fn apply_both(l: &Libs, sym: &[u8], a: c_int, b: c_int, c3: c_int) -> (c_int, c_int) {
    let cfp = l.c.addr(sym);
    let rfp = l.r.addr(sym);
    unsafe {
        (
            (l.c.apply_operation)(cfp, a, b, c3),
            (l.r.apply_operation)(rfp, a, b, c3),
        )
    }
}

#[test]
fn cfg_c8_apply_op_add_three() {
    let l = libs();
    let mut rng = Rng::new(0xC8_0000_0008);
    for &e0 in EXTREMES.iter() {
        for &e1 in EXTREMES.iter() {
            let (c, r) = apply_both(&l, b"add_three\0", e0, e1, 3);
            assert_eq!(c, r, "apply_operation(add_three,{e0},{e1},3)");
        }
    }
    for _ in 0..3000 {
        let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
        let (c, r) = apply_both(&l, b"add_three\0", a, b, c3);
        assert_eq!(c, r, "apply_operation(add_three,{a},{b},{c3})");
    }
}

#[test]
fn cfg_c9_apply_op_multiply_add() {
    let l = libs();
    let mut rng = Rng::new(0xC9_0000_0009);
    for &e0 in EXTREMES.iter() {
        for &e1 in EXTREMES.iter() {
            let (c, r) = apply_both(&l, b"multiply_add\0", e0, e1, -7);
            assert_eq!(c, r, "apply_operation(multiply_add,{e0},{e1},-7)");
        }
    }
    for _ in 0..3000 {
        let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
        let (c, r) = apply_both(&l, b"multiply_add\0", a, b, c3);
        assert_eq!(c, r, "apply_operation(multiply_add,{a},{b},{c3})");
    }
}

#[test]
fn cfg_c10_apply_op_complex_calc() {
    let l = libs();
    let mut rng = Rng::new(0xCA_0000_000A);
    for &(gc, ga) in STATES.iter() {
        l.set_state(gc, ga);
        for _ in 0..1500 {
            let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
            let (c, r) = apply_both(&l, b"complex_calc\0", a, b, c3);
            assert_eq!(c, r, "apply_operation(complex_calc,{a},{b},{c3}) state=({gc},{ga})");
        }
    }
    l.reset();
}

#[test]
fn cfg_c11_apply_op_cross_library() {
    let l = libs();
    let mut rng = Rng::new(0xCB_0000_000B);
    // Pure callees only: a cross-.so `complex_calc` would read the *other*
    // library's `global_counter`, which is a different thing by construction.
    for sym in [&b"add_three\0"[..], &b"multiply_add\0"[..]] {
        let c_fp = l.c.addr(sym);
        let r_fp = l.r.addr(sym);
        for _ in 0..2000 {
            let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
            // C's apply_operation calling into the Rust .so, and vice versa.
            let x = unsafe { (l.c.apply_operation)(r_fp, a, b, c3) };
            let y = unsafe { (l.r.apply_operation)(c_fp, a, b, c3) };
            // ...and each library calling its own callee.
            let x0 = unsafe { (l.c.apply_operation)(c_fp, a, b, c3) };
            let y0 = unsafe { (l.r.apply_operation)(r_fp, a, b, c3) };
            assert_eq!(x, y, "cross-library apply_operation({a},{b},{c3})");
            assert_eq!(x, x0, "C apply_operation with foreign vs own callee");
            assert_eq!(y, y0, "Rust apply_operation with foreign vs own callee");
        }
    }
}

// ---------------------------------------------------------------------------
// C12..C16 — shift_array_data, guard TRUE
// ---------------------------------------------------------------------------

/// Runs `shift_array_data` on two identical buffers and compares the return
/// (void) plus the whole buffer image including a trailing red zone.
fn shift_both(l: &Libs, buf: &[c_int], size: c_int, shift_by: c_int, ctx: &str) {
    let mut cbuf = buf.to_vec();
    let mut rbuf = buf.to_vec();
    unsafe {
        (l.c.shift_array_data)(cbuf.as_mut_ptr(), size, shift_by);
        (l.r.shift_array_data)(rbuf.as_mut_ptr(), size, shift_by);
    }
    assert_bytes_eq(bytes_of(&cbuf), bytes_of(&rbuf), ctx);
}

#[test]
fn cfg_c12_shift_hatch_shape() {
    let l = libs();
    let mut rng = Rng::new(0xCC_0000_000C);
    for _ in 0..2000 {
        // size=10, shift_by=3 is exactly what hatch() uses (lib.c:152).
        let buf = rand_int_buf(&mut rng, 10, 4);
        shift_both(&l, &buf, 10, 3, "shift(10,3)");
    }
}

#[test]
fn cfg_c13_shift_min_shape() {
    let l = libs();
    let mut rng = Rng::new(0xCD_0000_000D);
    for _ in 0..2000 {
        let buf = rand_int_buf(&mut rng, 2, 4);
        shift_both(&l, &buf, 2, 1, "shift(2,1)");
    }
}

#[test]
fn cfg_c14_shift_max_shift() {
    let l = libs();
    let mut rng = Rng::new(0xCE_0000_000E);
    for _ in 0..2000 {
        let size = rng.range(2, 64) as c_int;
        let buf = rand_int_buf(&mut rng, size as usize, 4);
        shift_both(&l, &buf, size, size - 1, &format!("shift({size},{})", size - 1));
    }
}

#[test]
fn cfg_c15_shift_random_shapes() {
    let l = libs();
    let mut rng = Rng::new(0xCF_0000_000F);
    for _ in 0..4000 {
        let size = rng.range(2, 256) as c_int;
        let shift_by = rng.range(1, size as i64 - 1) as c_int;
        let buf = rand_int_buf(&mut rng, size as usize, 4);
        shift_both(&l, &buf, size, shift_by, &format!("shift({size},{shift_by})"));
    }
}

#[test]
fn cfg_c16_shift_large() {
    let l = libs();
    let mut rng = Rng::new(0xD0_0000_0010);
    for _ in 0..64 {
        let size: c_int = 4096;
        let shift_by = rng.range(1, 4095) as c_int;
        let buf = rand_int_buf(&mut rng, size as usize, 4);
        shift_both(&l, &buf, size, shift_by, &format!("shift(4096,{shift_by})"));
    }
}

// ---------------------------------------------------------------------------
// C17..C19 — compute_with_dynamic_memory
// ---------------------------------------------------------------------------

fn cwdm_both(l: &Libs, base: c_int, count: c_int) {
    let (c, r) = unsafe {
        (
            (l.c.compute_with_dynamic_memory)(base, count),
            (l.r.compute_with_dynamic_memory)(base, count),
        )
    };
    assert_eq!(c, r, "compute_with_dynamic_memory({base},{count})");
}

#[test]
fn cfg_c17_cwdm_count_one() {
    let l = libs();
    let mut rng = Rng::new(0xD1_0000_0011);
    for &e in EXTREMES.iter() {
        cwdm_both(&l, e, 1);
    }
    for _ in 0..3000 {
        cwdm_both(&l, rng.interesting(), 1);
    }
}

#[test]
fn cfg_c18_cwdm_count_eight() {
    let l = libs();
    let mut rng = Rng::new(0xD2_0000_0012);
    for &e in EXTREMES.iter() {
        cwdm_both(&l, e, 8); // the value hatch() uses (lib.c:172)
    }
    for _ in 0..3000 {
        cwdm_both(&l, rng.interesting(), 8);
    }
}

#[test]
fn cfg_c19_cwdm_random() {
    let l = libs();
    let mut rng = Rng::new(0xD3_0000_0013);
    for _ in 0..2000 {
        let count = rng.range(1, 1024) as c_int;
        cwdm_both(&l, rng.interesting(), count);
    }
}

// ---------------------------------------------------------------------------
// C20 / C21 — get_time_based_value
// ---------------------------------------------------------------------------

fn time_both(l: &Libs, seed: c_int) {
    let (c, r) = unsafe {
        (
            (l.c.get_time_based_value)(seed),
            (l.r.get_time_based_value)(seed),
        )
    };
    assert_eq!(c, r, "get_time_based_value({seed})");
}

#[test]
fn cfg_c20_time_seed_band() {
    let l = libs();
    let mut rng = Rng::new(0xD4_0000_0014);
    for s in -20..=20 {
        time_both(&l, s);
    }
    for _ in 0..4000 {
        time_both(&l, rng.range_i32(-596_523, 596_523));
    }
}

#[test]
fn cfg_c21_time_seed_boundary() {
    let l = libs();
    // INT_MAX / 3600 == 596523; 596524 * 3600 overflows int.
    for s in [
        596_521, 596_522, 596_523, 596_524, 596_525, -596_521, -596_522, -596_523, -596_524,
        -596_525, 596_523 * 2, -596_523 * 2,
    ] {
        time_both(&l, s);
    }
}

// ---------------------------------------------------------------------------
// C22..C26 — manipulate_records, guard TRUE
// ---------------------------------------------------------------------------

/// Runs `manipulate_records` on two identical record buffers and compares the
/// return value plus the full post-`memmove` byte image (which pins the 48-byte
/// struct layout and the 0/4/8/16 field offsets).
fn records_both(l: &Libs, buf: &[DataRecord], num_records: c_int, shift: c_int, ctx: &str) {
    let mut cbuf = buf.to_vec();
    let mut rbuf = buf.to_vec();
    let (c, r) = unsafe {
        (
            (l.c.manipulate_records)(cbuf.as_mut_ptr(), num_records, shift),
            (l.r.manipulate_records)(rbuf.as_mut_ptr(), num_records, shift),
        )
    };
    assert_eq!(c, r, "{ctx}: return value");
    assert_bytes_eq(bytes_of(&cbuf), bytes_of(&rbuf), ctx);
}

#[test]
fn cfg_c22_records_hatch_shape() {
    let l = libs();
    let mut rng = Rng::new(0xD5_0000_0015);
    for _ in 0..2000 {
        // num_records=5, shift=2 is exactly what hatch() uses (lib.c:168).
        let buf = rand_record_buf(&mut rng, 5);
        records_both(&l, &buf, 5, 2, "manipulate_records(5,2)");
    }
}

#[test]
fn cfg_c23_records_min_shape() {
    let l = libs();
    let mut rng = Rng::new(0xD6_0000_0016);
    for _ in 0..2000 {
        let buf = rand_record_buf(&mut rng, 2);
        records_both(&l, &buf, 2, 1, "manipulate_records(2,1)");
    }
}

#[test]
fn cfg_c24_records_max_shift() {
    let l = libs();
    let mut rng = Rng::new(0xD7_0000_0017);
    for _ in 0..2000 {
        let n = rng.range(2, 32) as c_int;
        let buf = rand_record_buf(&mut rng, n as usize);
        records_both(&l, &buf, n, n - 1, &format!("manipulate_records({n},{})", n - 1));
    }
}

#[test]
fn cfg_c25_records_random_shapes() {
    let l = libs();
    let mut rng = Rng::new(0xD8_0000_0018);
    for _ in 0..3000 {
        let n = rng.range(2, 64) as c_int;
        let shift = rng.range(1, n as i64 - 1) as c_int;
        let buf = rand_record_buf(&mut rng, n as usize);
        records_both(&l, &buf, n, shift, &format!("manipulate_records({n},{shift})"));
    }
}

#[test]
fn cfg_c26_records_total_wrap() {
    let l = libs();
    let mut rng = Rng::new(0xD9_0000_0019);
    for _ in 0..2000 {
        let n = rng.range(2, 40) as c_int;
        let shift = rng.range(1, n as i64 - 1) as c_int;
        let mut buf = rand_record_buf(&mut rng, n as usize);
        // Force the running `total` to overflow int.
        for rec in buf.iter_mut() {
            rec.value = if rng.next_u64() & 1 == 0 { c_int::MAX } else { c_int::MAX - 1 };
        }
        records_both(&l, &buf, n, shift, &format!("manipulate_records wrap({n},{shift})"));
    }
}

// ---------------------------------------------------------------------------
// C27..C31 — hatch, the composed pipeline
// ---------------------------------------------------------------------------

fn hatch_both(l: &Libs, p: [c_int; 4], ctx: &str) {
    let (c, r) = unsafe {
        (
            (l.c.hatch)(p[0], p[1], p[2], p[3]),
            (l.r.hatch)(p[0], p[1], p[2], p[3]),
        )
    };
    assert_eq!(c, r, "{ctx}: hatch({},{},{},{})", p[0], p[1], p[2], p[3]);
    assert_eq!(
        l.c.read_counter(),
        l.r.read_counter(),
        "{ctx}: global_counter after hatch({},{},{},{})",
        p[0], p[1], p[2], p[3]
    );
    assert_eq!(
        l.c.read_accumulator(),
        l.r.read_accumulator(),
        "{ctx}: global_accumulator after hatch({},{},{},{})",
        p[0], p[1], p[2], p[3]
    );
}

#[test]
fn cfg_c27_hatch_small_fresh() {
    let l = libs();
    let mut rng = Rng::new(0xDA_0000_001A);
    for _ in 0..2000 {
        l.set_state(0, 0);
        let p = [rng.small(), rng.small(), rng.small(), rng.small()];
        hatch_both(&l, p, "fresh/small");
    }
    l.reset();
}

#[test]
fn cfg_c28_hatch_full_range() {
    let l = libs();
    let mut rng = Rng::new(0xDB_0000_001B);
    for _ in 0..4000 {
        l.set_state(0, 0);
        let p = [rng.i32_full(), rng.i32_full(), rng.i32_full(), rng.i32_full()];
        hatch_both(&l, p, "fresh/full-range");
    }
    l.reset();
}

#[test]
fn cfg_c29_hatch_extreme_grid() {
    let l = libs();
    const G: [c_int; 9] = [
        0,
        1,
        -1,
        2,
        -2,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX / 2,
        c_int::MIN / 2,
    ];
    for &a in G.iter() {
        for &b in G.iter() {
            for &c in G.iter() {
                for &d in G.iter() {
                    l.set_state(0, 0);
                    hatch_both(&l, [a, b, c, d], "extreme-grid");
                }
            }
        }
    }
    l.reset();
}

#[test]
fn cfg_c30_hatch_repeated() {
    let l = libs();
    let mut rng = Rng::new(0xDC_0000_001C);
    // NO reset inside the loop: global_counter / global_accumulator must track
    // identically across the whole run, including the `*2` doubling overflow.
    l.set_state(0, 0);
    for i in 0..512 {
        let p = [rng.interesting(), rng.interesting(), rng.interesting(), rng.interesting()];
        hatch_both(&l, p, &format!("repeat#{i}"));
    }
    l.reset();
}

#[test]
fn cfg_c31_interleaved_script() {
    let l = libs();
    let mut rng = Rng::new(0xDD_0000_001D);
    l.set_state(0, 0);
    for step in 0..3000 {
        match rng.next_u64() % 6 {
            0 => {
                let p = [rng.interesting(), rng.interesting(), rng.interesting(), rng.interesting()];
                hatch_both(&l, p, &format!("script#{step}"));
            }
            1 => {
                let v = rng.interesting();
                unsafe {
                    (l.c.increment_counter)(v, 999);
                    (l.r.increment_counter)(v, 999);
                }
            }
            2 => {
                let v = rng.interesting();
                unsafe {
                    (l.c.update_accumulator)(v, 888);
                    (l.r.update_accumulator)(v, 888);
                }
            }
            3 => {
                let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
                let (c, r) =
                    unsafe { ((l.c.complex_calc)(a, b, c3), (l.r.complex_calc)(a, b, c3)) };
                assert_eq!(c, r, "script#{step} complex_calc");
            }
            4 => {
                let v = rng.interesting();
                let m = rng.interesting();
                let (mut cv, mut rv) = (v, v);
                let (c, r) = unsafe {
                    (
                        (l.c.process_pointer_data)(&mut cv, m),
                        (l.r.process_pointer_data)(&mut rv, m),
                    )
                };
                assert_eq!(c, r, "script#{step} process_pointer_data");
            }
            _ => {
                let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
                let (c, r) = apply_both(&l, b"complex_calc\0", a, b, c3);
                assert_eq!(c, r, "script#{step} apply_operation(complex_calc)");
            }
        }
        assert_eq!(l.c.read_counter(), l.r.read_counter(), "script#{step} counter");
        assert_eq!(
            l.c.read_accumulator(),
            l.r.read_accumulator(),
            "script#{step} accumulator"
        );
    }
    l.reset();
}

// ---------------------------------------------------------------------------
// C32 — one long fuzz script over ALL 12 exported entry points
// ---------------------------------------------------------------------------

#[test]
fn cfg_c32_full_api_fuzz() {
    let l = libs();
    let mut rng = Rng::new(0xDE_0000_001E);
    l.set_state(0, 0);
    // Oversized, fully-initialised backing store so that even the C code's
    // out-of-range reads (shift < 0) are deterministic and comparable.
    const REC_CAP: usize = 128;

    for step in 0..5000u32 {
        match rng.next_u64() % 12 {
            0 => {
                let (v, j) = (rng.interesting(), rng.i32_full());
                unsafe {
                    (l.c.increment_counter)(v, j);
                    (l.r.increment_counter)(v, j);
                }
            }
            1 => {
                let (v, j) = (rng.interesting(), rng.i32_full());
                unsafe {
                    (l.c.update_accumulator)(v, j);
                    (l.r.update_accumulator)(v, j);
                }
            }
            2 => {
                let sym: &[u8] = match rng.next_u64() % 3 {
                    0 => b"add_three\0",
                    1 => b"multiply_add\0",
                    _ => b"complex_calc\0",
                };
                let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
                let (c, r) = apply_both(&l, sym, a, b, c3);
                assert_eq!(c, r, "fuzz#{step} apply_operation");
            }
            3 => {
                let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
                let (c, r) = unsafe { ((l.c.add_three)(a, b, c3), (l.r.add_three)(a, b, c3)) };
                assert_eq!(c, r, "fuzz#{step} add_three");
            }
            4 => {
                let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
                let (c, r) =
                    unsafe { ((l.c.multiply_add)(a, b, c3), (l.r.multiply_add)(a, b, c3)) };
                assert_eq!(c, r, "fuzz#{step} multiply_add");
            }
            5 => {
                let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
                let (c, r) =
                    unsafe { ((l.c.complex_calc)(a, b, c3), (l.r.complex_calc)(a, b, c3)) };
                assert_eq!(c, r, "fuzz#{step} complex_calc");
            }
            6 => {
                // shift_array_data over every shape, guard true AND false.
                let len = rng.range(0, 64) as usize;
                let buf = rand_int_buf(&mut rng, len, 4);
                let size = match rng.next_u64() % 4 {
                    0 => len as c_int,
                    1 => rng.range_i32(-4, len as c_int),
                    2 => 0,
                    _ => rng.range_i32(0, len as c_int),
                };
                let shift_by = match rng.next_u64() % 5 {
                    0 => 0,
                    1 => rng.range_i32(-4, 0),
                    2 => size,
                    3 => size + rng.range_i32(0, 4),
                    _ => rng.range_i32(1, (size - 1).max(1)),
                };
                // Only dereference when the array can actually hold `size`.
                if size <= len as c_int {
                    shift_both(
                        &l,
                        &buf,
                        size,
                        shift_by,
                        &format!("fuzz#{step} shift({size},{shift_by}) len={len}"),
                    );
                }
            }
            7 => {
                let v = rng.interesting();
                let m = rng.interesting();
                let (mut cv, mut rv) = (v, v);
                let (c, r) = unsafe {
                    (
                        (l.c.process_pointer_data)(&mut cv, m),
                        (l.r.process_pointer_data)(&mut rv, m),
                    )
                };
                assert_eq!(c, r, "fuzz#{step} process_pointer_data");
            }
            8 => {
                let count = match rng.next_u64() % 4 {
                    0 => 0,
                    1 => rng.range_i32(-8, -1),
                    2 => rng.range_i32(1, 64),
                    _ => rng.range_i32(1, 2048),
                };
                cwdm_both(&l, rng.interesting(), count);
            }
            9 => {
                let seed = match rng.next_u64() % 3 {
                    0 => rng.range_i32(-596_523, 596_523),
                    1 => rng.i32_full(),
                    _ => rng.small(),
                };
                time_both(&l, seed);
            }
            10 => {
                let n = rng.range(0, 64) as c_int;
                let shift = match rng.next_u64() % 5 {
                    0 => 0,
                    1 => rng.range_i32(-32, 0),
                    2 => n,
                    3 => n + rng.range_i32(0, 8),
                    _ => rng.range_i32(1, (n - 1).max(1)),
                };
                // n - shift <= 64 + 32 = 96 <= REC_CAP, so even the C code's
                // over-read stays inside a fully-initialised buffer.
                let buf = rand_record_buf(&mut rng, REC_CAP);
                records_both(
                    &l,
                    &buf,
                    n,
                    shift,
                    &format!("fuzz#{step} manipulate_records({n},{shift})"),
                );
            }
            _ => {
                let p = [rng.interesting(), rng.interesting(), rng.interesting(), rng.interesting()];
                hatch_both(&l, p, &format!("fuzz#{step}"));
            }
        }
        assert_eq!(l.c.read_counter(), l.r.read_counter(), "fuzz#{step} counter");
        assert_eq!(
            l.c.read_accumulator(),
            l.r.read_accumulator(),
            "fuzz#{step} accumulator"
        );
    }
    l.reset();
}

// A tiny compile-time/ABI guard: the harness's DataRecord must match the C one.
#[test]
fn cfg_datarecord_layout_matches_c() {
    assert_eq!(std::mem::size_of::<DataRecord>(), 48);
    assert_eq!(std::mem::align_of::<DataRecord>(), 8);
    let r = DataRecord::zeroed();
    let base = &r as *const _ as usize;
    assert_eq!(&r.id as *const _ as usize - base, 0);
    assert_eq!(&r.value as *const _ as usize - base, 4);
    assert_eq!(&r.timestamp as *const _ as usize - base, 8);
    assert_eq!(&r.name as *const _ as usize - base, 16);
    let _ = std::mem::size_of::<*const c_void>();
}
