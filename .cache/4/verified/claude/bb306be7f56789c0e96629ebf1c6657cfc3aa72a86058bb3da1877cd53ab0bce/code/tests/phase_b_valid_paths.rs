// Phase B -- valid-path differential tests. One test per CONFIGS.md row.
//
// Every row drives BOTH the C `.so` and the Rust `.so` through `libloading` and
// compares the bytes they write to stdout. Randomized rows use a fixed seed.
mod common;

use common::*;
use std::ffi::c_int;

/// Runs `printIntPtrLine` on both implementations for the same pointer and
/// asserts the emitted bytes are identical.
fn assert_print_eq(c: &Api, r: &Api, p: *const c_int, ctx: &str) {
    let out_c = capture("c", || unsafe { (c.print_int_ptr_line)(p) });
    let out_r = capture("r", || unsafe { (r.print_int_ptr_line)(p) });
    assert_eq!(
        out_c,
        out_r,
        "printIntPtrLine mismatch for {ctx}: C={:?} Rust={:?}",
        show(&out_c),
        show(&out_r)
    );
    assert!(!out_c.is_empty(), "no output captured for {ctx}");
}

/// Sweeps a list of `int` values through a stack slot.
fn sweep_values(c: &Api, r: &Api, values: &[i32], ctx: &str) {
    for &v in values {
        let boxed: c_int = v;
        assert_print_eq(c, r, &boxed as *const c_int, &format!("{ctx} value={v}"));
    }
}

// ---------------------------------------------------------------- rows 1..7 --

#[test]
fn row01_to_05_print_boundary_values() {
    let (c, r) = both();
    // rows 1,2,3,4,5: zero, the value good() uses, digit-count and sign
    // boundaries, and the two extremes of int.
    sweep_values(
        &c,
        &r,
        &[
            0,
            5,
            1,
            -1,
            9,
            10,
            -9,
            -10,
            99,
            100,
            -99,
            -100,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
        ],
        "stack",
    );
}

#[test]
fn row06_print_powers_of_two_plus_minus_one() {
    let (c, r) = both();
    let mut values = Vec::new();
    for bit in 0..32u32 {
        let base = 1i32.wrapping_shl(bit);
        values.push(base);
        values.push(base.wrapping_sub(1));
        values.push(base.wrapping_add(1));
    }
    assert_eq!(values.len(), 96);
    sweep_values(&c, &r, &values, "pow2");
}

#[test]
fn row07_print_randomized_full_range() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    let values: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    sweep_values(&c, &r, &values, "random");
}

// --------------------------------------------------------------- rows 8..12 --

#[test]
fn row08_print_heap_pointer() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..64 {
        let v = rng.next_i32();
        let boxed = Box::new(v);
        assert_print_eq(
            &c,
            &r,
            &*boxed as *const c_int,
            &format!("heap i={i} value={v}"),
        );
    }
}

#[test]
fn row09_print_static_bss_pointer() {
    static mut SLOT: c_int = 0;
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..64 {
        let v = rng.next_i32();
        let p = unsafe {
            SLOT = v;
            &raw const SLOT
        };
        assert_print_eq(&c, &r, p, &format!("static i={i} value={v}"));
    }
}

#[test]
fn row10_print_array_interior_all_indices() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 10);
    for round in 0..4 {
        let mut arr = [0i32; 16];
        for slot in arr.iter_mut() {
            *slot = rng.next_i32();
        }
        for k in 0..16 {
            assert_print_eq(
                &c,
                &r,
                &arr[k] as *const c_int,
                &format!("array round={round} index={k} value={}", arr[k]),
            );
        }
    }
}

#[test]
fn row11_print_misaligned_pointer() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 11);
    // The C compiles `*intNumber` to a plain `mov`, which loads fine from an
    // unaligned address on x86-64. A naive Rust `*ptr` would instead trip the
    // debug-only "misaligned pointer dereference" assertion and abort.
    for offset in 1..=3usize {
        for i in 0..32 {
            let v = rng.next_i32();
            let mut buf = [0u8; 16];
            buf[offset..offset + 4].copy_from_slice(&v.to_ne_bytes());
            let p = unsafe { buf.as_ptr().add(offset) } as *const c_int;
            assert_print_eq(
                &c,
                &r,
                p,
                &format!("misaligned offset={offset} i={i} value={v}"),
            );
        }
    }
}

#[test]
fn row12_print_last_int_of_a_mapping() {
    let (c, r) = both();
    // One page, read/write; the page after it is unmapped. Reading the final 4
    // bytes must succeed -- one byte further would fault.
    let page = 4096usize;
    let m = Mapping::new(page, PROT_READ | PROT_WRITE);
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..16 {
        let v = rng.next_i32();
        let p = unsafe { m.ptr.add(page - 4) };
        unsafe { std::ptr::write_unaligned(p as *mut c_int, v) };
        assert_print_eq(
            &c,
            &r,
            p as *const c_int,
            &format!("last-int-of-page i={i} value={v}"),
        );
    }
}

// -------------------------------------------------------------- rows 13..14 --

#[test]
fn row13_many_sequential_calls_one_buffered_stream() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 13);
    let values: Vec<i32> = (0..32).map(|_| rng.next_i32()).collect();
    let out_c = capture("c", || {
        for v in &values {
            unsafe { (c.print_int_ptr_line)(v as *const c_int) };
        }
    });
    let out_r = capture("r", || {
        for v in &values {
            unsafe { (r.print_int_ptr_line)(v as *const c_int) };
        }
    });
    assert_eq!(out_c, out_r, "32-call sequence diverged");
    assert_eq!(
        out_c.iter().filter(|&&b| b == b'\n').count(),
        32,
        "expected 32 lines"
    );
}

#[test]
fn row14_c_and_rust_interleaved_share_one_stdout_buffer() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 14);
    let values: Vec<i32> = (0..24).map(|_| rng.next_i32()).collect();
    // Interleaving proves neither implementation disturbs the shared libc
    // stdout buffer differently from the other.
    let interleaved = capture("mix", || {
        for v in &values {
            unsafe { (c.print_int_ptr_line)(v as *const c_int) };
            unsafe { (r.print_int_ptr_line)(v as *const c_int) };
        }
    });
    let mut expected = Vec::new();
    for v in &values {
        let line = format!("{v}\n");
        expected.extend_from_slice(line.as_bytes());
        expected.extend_from_slice(line.as_bytes());
    }
    assert_eq!(
        show(&interleaved),
        show(&expected),
        "interleaved C/Rust output diverged"
    );
}

// -------------------------------------------------------------- rows 15..16 --

#[test]
fn row15_good_single_call() {
    let (c, r) = both();
    let out_c = capture("c", || unsafe { (c.good)() });
    let out_r = capture("r", || unsafe { (r.good)() });
    assert_eq!(out_c, out_r, "good() diverged");
    assert_eq!(show(&out_c), "5\\n", "good() must print 5");
}

#[test]
fn row16_good_repeated() {
    let (c, r) = both();
    for n in [1usize, 2, 8] {
        let out_c = capture("c", || {
            for _ in 0..n {
                unsafe { (c.good)() }
            }
        });
        let out_r = capture("r", || {
            for _ in 0..n {
                unsafe { (r.good)() }
            }
        });
        assert_eq!(out_c, out_r, "good() x{n} diverged");
        assert_eq!(show(&out_c), "5\\n".repeat(n), "good() x{n} content");
    }
}

// -------------------------------------------------------------- rows 17..20 --

#[test]
fn row17_driver_canonical_true() {
    let (c, r) = both();
    let out_c = capture("c", || unsafe { (c.driver)(1) });
    let out_r = capture("r", || unsafe { (r.driver)(1) });
    assert_eq!(out_c, out_r, "driver(1) diverged");
    assert_eq!(show(&out_c), "5\\n");
}

#[test]
fn row18_driver_nonzero_including_out_of_range_enum_values() {
    let (c, r) = both();
    // `if (useGood)` is a truthiness test, so every one of these -- including
    // values that would be invalid for any enum -- takes the good() arm.
    for v in [
        1,
        -1,
        2,
        3,
        7,
        42,
        -42,
        255,
        256,
        -256,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x7fff_ffff,
        -0x8000_0000,
    ] {
        let out_c = capture("c", || unsafe { (c.driver)(v) });
        let out_r = capture("r", || unsafe { (r.driver)(v) });
        assert_eq!(out_c, out_r, "driver({v}) diverged");
        assert_eq!(show(&out_c), "5\\n", "driver({v}) must take the good arm");
    }
}

#[test]
fn row19_driver_randomized_nonzero() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..256 {
        let v = rng.next_nonzero_i32();
        let out_c = capture("c", || unsafe { (c.driver)(v) });
        let out_r = capture("r", || unsafe { (r.driver)(v) });
        assert_eq!(out_c, out_r, "driver({v}) diverged at i={i}");
        assert_eq!(show(&out_c), "5\\n", "driver({v}) must print 5");
    }
}

#[test]
fn row20_driver_true_repeated() {
    let (c, r) = both();
    for n in [1usize, 2, 8] {
        let out_c = capture("c", || {
            for _ in 0..n {
                unsafe { (c.driver)(1) }
            }
        });
        let out_r = capture("r", || {
            for _ in 0..n {
                unsafe { (r.driver)(1) }
            }
        });
        assert_eq!(out_c, out_r, "driver(1) x{n} diverged");
        assert_eq!(show(&out_c), "5\\n".repeat(n));
    }
}

// ------------------------------------------------------------------- row 21 --

#[test]
fn row21_mixed_entry_points_top_and_mid() {
    let (c, r) = both();
    let out_c = capture("c", || unsafe {
        (c.driver)(1);
        (c.good)();
        (c.driver)(3);
    });
    let out_r = capture("r", || unsafe {
        (r.driver)(1);
        (r.good)();
        (r.driver)(3);
    });
    assert_eq!(out_c, out_r, "mixed driver/good sequence diverged");
    assert_eq!(show(&out_c), "5\\n5\\n5\\n");
}

// -------------------------------------------------------------- rows 22..25 --
//
// The deterministic part of the CWE-457 defect: `good()` stores `&data` into the
// same stack slot that a following `bad()` -- entered at the same depth -- reads
// back, so the pair prints `5` twice. See ERRORS.md note A.

#[test]
fn row22_good_then_bad_same_depth() {
    let (c, r) = both();
    let out_c = capture("c", || unsafe {
        (c.good)();
        (c.bad)();
    });
    let out_r = capture("r", || unsafe {
        (r.good)();
        (r.bad)();
    });
    assert_eq!(
        show(&out_c),
        "5\\n5\\n",
        "C good();bad() should be deterministic"
    );
    assert_eq!(out_c, out_r, "good();bad() diverged");
}

#[test]
fn row23_good_then_bad_twice() {
    let (c, r) = both();
    let out_c = capture("c", || unsafe {
        (c.good)();
        (c.bad)();
        (c.bad)();
    });
    let out_r = capture("r", || unsafe {
        (r.good)();
        (r.bad)();
        (r.bad)();
    });
    assert_eq!(show(&out_c), "5\\n5\\n5\\n");
    assert_eq!(out_c, out_r, "good();bad();bad() diverged");
}

#[test]
fn row24_driver_true_then_driver_false() {
    let (c, r) = both();
    let out_c = capture("c", || unsafe {
        (c.driver)(1);
        (c.driver)(0);
    });
    let out_r = capture("r", || unsafe {
        (r.driver)(1);
        (r.driver)(0);
    });
    assert_eq!(show(&out_c), "5\\n5\\n");
    assert_eq!(out_c, out_r, "driver(1);driver(0) diverged");
}

#[test]
fn row25_driver_true_then_driver_false_twice() {
    let (c, r) = both();
    let out_c = capture("c", || unsafe {
        (c.driver)(1);
        (c.driver)(0);
        (c.driver)(0);
    });
    let out_r = capture("r", || unsafe {
        (r.driver)(1);
        (r.driver)(0);
        (r.driver)(0);
    });
    assert_eq!(show(&out_c), "5\\n5\\n5\\n");
    assert_eq!(out_c, out_r, "driver(1);driver(0);driver(0) diverged");
}

// -------------------------------------------------------------- rows 26..28 --

#[test]
fn rows26_28_indeterminate_ub_paths_are_recorded_not_asserted() {
    // `bad()` reached without a preceding same-depth `good()` reads whichever 8
    // bytes happen to occupy that stack slot. The C's own output is not
    // reproducible there (observed values include 0, 3, -2040302194,
    // 1420842379, and outright SIGSEGV for the same sequence), so asserting
    // byte equality would be asserting a fiction. The outcomes are executed
    // against both `.so`s and recorded.
    let (c, r) = both();
    let cases: Vec<(&str, Box<dyn Fn(&Api)>)> = vec![
        ("bad() alone", Box::new(|a: &Api| unsafe { (a.bad)() })),
        ("driver(0) alone", Box::new(|a: &Api| unsafe { (a.driver)(0) })),
        (
            "driver(1) then bad() [depth mismatch]",
            Box::new(|a: &Api| unsafe {
                (a.driver)(1);
                (a.bad)();
            }),
        ),
        (
            "good() then driver(0) [depth mismatch]",
            Box::new(|a: &Api| unsafe {
                (a.good)();
                (a.driver)(0);
            }),
        ),
    ];
    for (label, f) in cases {
        let oc = run_isolated("c", || f(&c));
        let or = run_isolated("r", || f(&r));
        eprintln!(
            "  [indeterminate] {label:<40} C: {} out={:<16} | Rust: {} out={}",
            oc.kind(),
            show(oc.out()),
            or.kind(),
            show(or.out())
        );
        // Both must at least reach a defined process outcome rather than hang.
        for (who, o) in [("C", &oc), ("Rust", &or)] {
            match o {
                Outcome::Exited { .. } => {}
                Outcome::Signaled { sig, .. } => assert!(
                    *sig == SIGSEGV || *sig == SIGBUS || *sig == 6,
                    "{who} {label}: unexpected signal {sig}"
                ),
            }
        }
    }
}
