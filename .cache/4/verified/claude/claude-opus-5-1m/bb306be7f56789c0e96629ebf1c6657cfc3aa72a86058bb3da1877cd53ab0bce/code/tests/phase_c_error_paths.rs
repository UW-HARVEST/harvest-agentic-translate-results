// Phase C -- error-path differential tests. One test per ERRORS.md row.
//
// The C library has no error codes at all (every function returns `void`, and the
// only conditional in the whole library is `driver`'s `if (useGood)` dispatch).
// Its sole way of rejecting bad input is to fault, so "same error" here means
// "killed by the same signal", checked in a forked child.
mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Asserts both implementations reach the same process-level outcome (identical
/// signal, or identical exit code) for a faulting input.
fn assert_same_outcome(c: &Api, r: &Api, label: &str, f: impl Fn(&Api) + Copy) {
    let oc = run_isolated("c", || f(c));
    let or = run_isolated("r", || f(r));
    assert_eq!(
        oc.kind(),
        or.kind(),
        "{label}: outcome differs -- C={} (out={:?}) Rust={} (out={:?})",
        oc.kind(),
        show(oc.out()),
        or.kind(),
        show(or.out())
    );
    eprintln!("  [row ok] {label:<52} both {}", oc.kind());
}

/// Asserts both implementations print identical bytes and neither faults.
fn assert_same_output(c: &Api, r: &Api, label: &str, f: impl Fn(&Api) + Copy) -> Vec<u8> {
    let oc = run_isolated("c", || f(c));
    let or = run_isolated("r", || f(r));
    assert_eq!(oc.kind(), or.kind(), "{label}: outcome differs");
    assert_eq!(
        show(oc.out()),
        show(or.out()),
        "{label}: output differs -- C={:?} Rust={:?}",
        show(oc.out()),
        show(or.out())
    );
    eprintln!(
        "  [row ok] {label:<52} both {} out={}",
        oc.kind(),
        show(oc.out())
    );
    oc.out().to_vec()
}

// ------------------------------------------------------------------- row 1 ---

#[test]
fn row01_null_pointer_into_print_int_ptr_line() {
    let (c, r) = both();
    // driver.c:30 dereferences unconditionally -- there is no null check.
    assert_same_outcome(&c, &r, "row1 printIntPtrLine(NULL)", |a| unsafe {
        (a.print_int_ptr_line)(std::ptr::null())
    });
    // and specifically: killed by SIGSEGV, not merely "failed somehow"
    let oc = run_isolated("c", || unsafe { (c.print_int_ptr_line)(std::ptr::null()) });
    assert_eq!(
        oc.signal(),
        Some(SIGSEGV),
        "C must die with SIGSEGV on NULL, got {}",
        oc.kind()
    );
}

// ------------------------------------------------------------------- row 2 ---

#[test]
fn row02_unmapped_pointers_into_print_int_ptr_line() {
    let (c, r) = both();
    for (label, addr) in [
        ("0x1", 0x1usize),
        ("0x4", 0x4usize),
        ("0xdead_0000", 0xdead_0000usize),
        ("unmapped high", 0x0000_7f00_0000_0000usize),
    ] {
        assert_same_outcome(
            &c,
            &r,
            &format!("row2 printIntPtrLine({label})"),
            move |a| unsafe { (a.print_int_ptr_line)(addr as *const c_int) },
        );
        let oc = run_isolated("c", || unsafe {
            (c.print_int_ptr_line)(addr as *const c_int)
        });
        assert_eq!(
            oc.signal(),
            Some(SIGSEGV),
            "C must SIGSEGV for {label}, got {}",
            oc.kind()
        );
    }
}

// ------------------------------------------------------------------- row 3 ---

#[test]
fn row03_misaligned_pointer_is_not_an_error() {
    let (c, r) = both();
    // GCC lowers `*intNumber` to a plain `mov`, so an unaligned load simply
    // works on x86-64. This row exists because a naive Rust `*ptr` would abort
    // here with a debug-only misaligned-pointer assertion.
    let mut rng = Rng::new(SEED ^ 3);
    for offset in 1..=3usize {
        let v = rng.next_i32();
        let mut buf = [0u8; 16];
        buf[offset..offset + 4].copy_from_slice(&v.to_ne_bytes());
        let p = unsafe { buf.as_ptr().add(offset) } as *const c_int;
        let out = assert_same_output(
            &c,
            &r,
            &format!("row3 misaligned offset={offset}"),
            move |a| unsafe { (a.print_int_ptr_line)(p) },
        );
        assert_eq!(
            String::from_utf8_lossy(&out).trim_end(),
            v.to_string(),
            "misaligned read must yield the stored value"
        );
    }
}

// ------------------------------------------------------------------- row 4 ---

#[test]
fn row04_prot_none_mapping_faults() {
    let (c, r) = both();
    let m = Mapping::new(4096, PROT_NONE);
    let p = m.ptr as *const c_int;
    assert_same_outcome(&c, &r, "row4 printIntPtrLine(PROT_NONE page)", move |a| {
        unsafe { (a.print_int_ptr_line)(p) }
    });
    let oc = run_isolated("c", || unsafe { (c.print_int_ptr_line)(p) });
    assert_eq!(oc.signal(), Some(SIGSEGV), "PROT_NONE read must SIGSEGV");
}

// ---------------------------------------------------------------- rows 5,6 ---

#[test]
fn row05_and_row06_int_min_and_int_max_render_exactly() {
    let (c, r) = both();
    for (label, v, expect) in [
        ("row5 INT_MIN", i32::MIN, "-2147483648"),
        ("row6 INT_MAX", i32::MAX, "2147483647"),
    ] {
        let boxed: c_int = v;
        let p = &boxed as *const c_int;
        let out = assert_same_output(&c, &r, label, move |a| unsafe {
            (a.print_int_ptr_line)(p)
        });
        assert_eq!(
            String::from_utf8_lossy(&out),
            format!("{expect}\n"),
            "{label} must render as {expect}"
        );
    }
}

// ------------------------------------------------------------------- row 7 ---

#[test]
fn row07_last_int_before_unmapped_page_succeeds() {
    let (c, r) = both();
    let page = 4096usize;
    let m = Mapping::new(page, PROT_READ | PROT_WRITE);
    let p = unsafe { m.ptr.add(page - 4) };
    unsafe { std::ptr::write_unaligned(p as *mut c_int, 0x1234_5678) };
    let out = assert_same_output(&c, &r, "row7 last int of mapping", move |a| unsafe {
        (a.print_int_ptr_line)(p as *const c_int)
    });
    assert_eq!(String::from_utf8_lossy(&out), "305419896\n");

    // One step past the documented valid range: the first int of the next,
    // unmapped page must fault identically in both.
    let past = unsafe { m.ptr.add(page) } as *const c_int;
    assert_same_outcome(&c, &r, "row7b one int past the mapping", move |a| unsafe {
        (a.print_int_ptr_line)(past)
    });
}

// ------------------------------------------------------------------- row 8 ---

#[test]
fn row08_one_int_past_a_one_element_allocation() {
    let (c, r) = both();
    // A 2-int buffer with a known value in the second slot models reading index
    // 1 of a 1-element array: out of range for the caller, but in-bounds memory,
    // so both implementations must read the same adjacent bytes.
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..16 {
        let sentinel = rng.next_i32();
        let buf: Box<[c_int; 2]> = Box::new([1, sentinel]);
        let past = unsafe { buf.as_ptr().add(1) };
        let out = assert_same_output(
            &c,
            &r,
            &format!("row8 one past 1-elem alloc i={i}"),
            move |a| unsafe { (a.print_int_ptr_line)(past) },
        );
        assert_eq!(
            String::from_utf8_lossy(&out).trim_end(),
            sentinel.to_string()
        );
    }
}

// ---------------------------------------------------------------- rows 9,11 --

#[test]
fn row09_and_row11_uninitialised_read_in_isolation_is_indeterminate() {
    // ERRORS.md note A: with no preceding same-depth `good()`, `bad()` reads
    // whichever 8 bytes occupy that stack slot. The C is not self-consistent
    // here, so the outcome is recorded rather than asserted equal.
    let (c, r) = both();
    for (label, f) in [
        (
            "row9  bad() in isolation",
            Box::new(|a: &Api| unsafe { (a.bad)() }) as Box<dyn Fn(&Api)>,
        ),
        (
            "row11 driver(0) in isolation",
            Box::new(|a: &Api| unsafe { (a.driver)(0) }),
        ),
    ] {
        let oc = run_isolated("c", || f(&c));
        let or = run_isolated("r", || f(&r));
        eprintln!(
            "  [indeterminate] {label:<40} C: {} out={:<14} | Rust: {} out={}",
            oc.kind(),
            show(oc.out()),
            or.kind(),
            show(or.out())
        );
        // Both must reach a defined outcome: clean exit, or a memory-fault
        // signal -- never a hang or an unexpected signal.
        for (who, o) in [("C", &oc), ("Rust", &or)] {
            if let Outcome::Signaled { sig, .. } = o {
                assert!(
                    *sig == SIGSEGV || *sig == SIGBUS || *sig == 6,
                    "{who} {label}: unexpected signal {sig}"
                );
            }
        }
    }
}

// ------------------------------------------------------------------ row 10 ---

#[test]
fn row10_uninitialised_read_after_good_is_deterministic_five() {
    let (c, r) = both();
    // The one deterministic observable of the defect, asserted byte-for-byte.
    let out = assert_same_output(&c, &r, "row10 good(); bad()", |a| unsafe {
        (a.good)();
        (a.bad)();
    });
    assert_eq!(String::from_utf8_lossy(&out), "5\n5\n");
}

// -------------------------------------------------------------- rows 12,13 ---

#[test]
fn row12_and_row13_out_of_range_enum_values_are_accepted() {
    let (c, r) = both();
    // `driver`'s parameter is a plain `int` and the check is a truthiness test,
    // so every non-zero value -- including ones no enum could name -- is
    // accepted and takes the good() arm. C enums accept any int across FFI.
    let mut rng = Rng::new(SEED ^ 12);
    let mut values = vec![
        2,
        3,
        7,
        -1,
        -2,
        99,
        0x1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for _ in 0..32 {
        values.push(rng.next_nonzero_i32());
    }
    for v in values {
        let out = assert_same_output(&c, &r, &format!("row12 driver({v})"), move |a| unsafe {
            (a.driver)(v)
        });
        assert_eq!(
            String::from_utf8_lossy(&out),
            "5\n",
            "driver({v}) must be accepted and print 5"
        );
    }
}

// ------------------------------------------------------------------ row 14 ---

#[test]
fn row14_only_the_low_32_bits_of_the_int_argument_are_examined() {
    let (c, r) = both();
    // `driver` is compiled as `cmpl $0x0,-0x4(%rbp)` -- it inspects %edi only.
    // Calling through an `extern "C" fn(i64)` signature puts garbage in the high
    // half; both implementations must ignore it identically.
    for hi in [0x1_0000_0000i64, 0x7fff_ffff_0000_0000, -4294967296 /* 0xffffffff_00000000 */] {
        // low 32 bits are zero -> must behave like driver(0), i.e. the bad() arm
        assert_same_outcome(
            &c,
            &r,
            &format!("row14 driver64({hi:#x}) low32=0"),
            move |a| unsafe {
                (a.driver)(1); // make the bad() arm deterministic (ERRORS.md note A)
                (a.driver64)(hi);
            },
        );
        let out = assert_same_output(
            &c,
            &r,
            &format!("row14 driver64({hi:#x}) after driver(1)"),
            move |a| unsafe {
                (a.driver)(1);
                (a.driver64)(hi);
            },
        );
        assert_eq!(
            String::from_utf8_lossy(&out),
            "5\n5\n",
            "high-half garbage must not change the arm taken"
        );

        // and with a non-zero low half, the good() arm is taken
        let with_low = hi | 0x2b;
        let out = assert_same_output(
            &c,
            &r,
            &format!("row14 driver64({with_low:#x}) low32!=0"),
            move |a| unsafe { (a.driver64)(with_low) },
        );
        assert_eq!(String::from_utf8_lossy(&out), "5\n");
    }
}

#[test]
fn row14b_low32_zero_selects_the_same_arm_as_driver_zero() {
    let (c, r) = both();
    // Independent confirmation that truncation really happens: in isolation,
    // driver64(0x1_0000_0000) must behave like driver(0), not like driver(1).
    for a in [&c, &r] {
        let zero_arm = run_isolated("z", || unsafe { (a.driver)(0) });
        let trunc = run_isolated("t", || unsafe { (a.driver64)(0x1_0000_0000) });
        let true_arm = run_isolated("o", || unsafe { (a.driver)(1) });
        eprintln!(
            "  [{}] driver(0)={} driver64(1<<32)={} driver(1)={} out={}",
            a.name,
            zero_arm.kind(),
            trunc.kind(),
            true_arm.kind(),
            show(true_arm.out())
        );
        assert_eq!(
            String::from_utf8_lossy(true_arm.out()),
            "5\n",
            "{}: driver(1) must print exactly 5",
            a.name
        );
        assert_eq!(
            trunc.kind(),
            zero_arm.kind(),
            "{}: driver64(1<<32) must take the same arm as driver(0)",
            a.name
        );
    }
}

// ------------------------------------------------------- generic FFI checks --

#[test]
fn generic_null_and_wild_pointers_agree_across_all_entry_points() {
    let (c, r) = both();
    // A sweep of pointer values that are all invalid, confirming the two
    // implementations fault identically rather than one of them "handling" it.
    let mut rng = Rng::new(SEED ^ 0xffff);
    let mut addrs: Vec<usize> = vec![0, 1, 2, 3, 4, 8, 0xfff, 0x1000, usize::MAX, usize::MAX - 3];
    for _ in 0..8 {
        // low, certainly-unmapped addresses
        addrs.push((rng.next_u32() as usize) & 0xffff);
    }
    for addr in addrs {
        let oc = run_isolated("c", || unsafe {
            (c.print_int_ptr_line)(addr as *const c_int)
        });
        let or = run_isolated("r", || unsafe {
            (r.print_int_ptr_line)(addr as *const c_int)
        });
        assert_eq!(
            oc.kind(),
            or.kind(),
            "printIntPtrLine({addr:#x}): C={} Rust={}",
            oc.kind(),
            or.kind()
        );
    }
}

#[test]
fn generic_void_entry_points_take_no_arguments_and_ignore_extras() {
    let (c, r) = both();
    // `good`/`bad` are `void(void)`; passing arguments through a wrong signature
    // must be ignored identically by both (the callee never reads them).
    type FnWrong = unsafe extern "C" fn(c_int, *mut c_void) -> c_int;
    let cg: FnWrong = unsafe { std::mem::transmute(c.good) };
    let rg: FnWrong = unsafe { std::mem::transmute(r.good) };
    let out_c = capture("c", || unsafe {
        cg(0x7fff_ffff, usize::MAX as *mut c_void);
    });
    let out_r = capture("r", || unsafe {
        rg(0x7fff_ffff, usize::MAX as *mut c_void);
    });
    assert_eq!(out_c, out_r, "good() with spurious arguments diverged");
    assert_eq!(show(&out_c), "5\\n");
}
