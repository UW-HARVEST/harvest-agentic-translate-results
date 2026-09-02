//! Phase C — error-path differential tests, one test per `ERRORS.md` row, plus
//! the generic FFI boundary checks the task requires regardless of the table.
//!
//! Both libraries are loaded through `libloading` and driven only via their
//! exported symbols.
//!
//! Anything that can fault runs in a child process ([`common::run_isolated`]),
//! so a `SIGSEGV` is an *observation* — the two implementations must be killed
//! by the same signal and must have flushed the same bytes before dying — rather
//! than something that aborts the test run.

mod common;

use common::*;

const SIGSEGV: i32 = 11;
const SIGBUS: i32 = 7;

/// Asserts both libraries died from the same fatal signal with no output.
fn assert_same_fault(spec: &str) -> Outcome {
    let (c, r) = run_isolated_both(spec);
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "termination diverged for {spec:?}: C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c.stdout, r.stdout,
        "output diverged for {spec:?}: C={:?} Rust={:?}",
        c.text(),
        r.text()
    );
    assert!(
        c.crashed(),
        "expected {spec:?} to fault in the C library, but it exited with {:?} and printed {:?}",
        c.code,
        c.text()
    );
    assert!(
        matches!(c.signal, Some(SIGSEGV) | Some(SIGBUS)),
        "expected SIGSEGV/SIGBUS for {spec:?}, got signal {:?}",
        c.signal
    );
    c
}

// ---------------------------------------------------------------------------
// Row 1 — printIntPtrLine(NULL)
// ---------------------------------------------------------------------------
fn err01_pipl_null() {
    // The C has no null check (driver.c:30 dereferences unconditionally), so
    // both must fault identically and neither may have flushed anything.
    let c = assert_same_fault("pipl_null");
    assert_eq!(c.signal, Some(SIGSEGV));
    assert!(
        c.stdout.is_empty(),
        "nothing should reach stdout before the fault, got {:?}",
        c.text()
    );

    // Also via the raw-address form, which is the same input expressed
    // differently, to confirm the harness is not special-casing NULL.
    let c2 = assert_same_fault("pipl_addr:0");
    assert_eq!(c2.signal, Some(SIGSEGV));
}

// ---------------------------------------------------------------------------
// Row 2 — printIntPtrLine with a non-null unmapped address
// ---------------------------------------------------------------------------
fn err02_pipl_unmapped_addresses() {
    // Fixed low/odd/kernel-ish addresses that are never mapped in a normal
    // process, plus a page that really was mapped and then unmapped.
    for spec in [
        "pipl_addr:1",
        "pipl_addr:2",
        "pipl_addr:3",
        "pipl_addr:4",
        "pipl_addr:8",
        "pipl_addr:4095",
        "pipl_addr:4096",
        // 0xFFFFFFFFFFFFFFFF and 0xFFFFFFFFFFFFFFFC — non-canonical, and the
        // (int*)-1 idiom.
        "pipl_addr:18446744073709551615",
        "pipl_addr:18446744073709551612",
        // A canonical-but-unmapped high userspace address.
        "pipl_addr:140187732541440",
    ] {
        assert_same_fault(spec);
    }
    assert_same_fault("pipl_unmapped");
}

// ---------------------------------------------------------------------------
// Row 3 — misaligned but readable: NOT an error on x86_64
// ---------------------------------------------------------------------------
fn err03_pipl_misaligned_is_not_an_error() {
    // Documented here as an error-surface row because a naive reading expects a
    // fault; the C actually succeeds, and the Rust must succeed identically
    // rather than "fixing" it with an alignment check.
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0xE770_0003);
    for _ in 0..256 {
        let word = rng.next_u64() as u32;
        for off in 1..4usize {
            let spec = format!("pipl_unaligned:{word}:{off}");
            let (out_c, out_r) = run_in_process(&c, &r, &spec);
            assert_eq!(out_c, out_r, "{spec} diverged");
            assert!(
                is_one_int_line(&out_c),
                "{spec} should have printed one int, got {:?}",
                String::from_utf8_lossy(&out_c)
            );
        }
    }
    // And confirm it does not fault in a fresh process either.
    for off in 1..4usize {
        let (oc, or) = run_isolated_both(&format!("pipl_unaligned:305419896:{off}"));
        assert!(!oc.crashed(), "C should not fault on a misaligned read");
        assert!(!or.crashed(), "Rust should not fault on a misaligned read");
        assert_eq!(oc.stdout, or.stdout);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — 4-byte read straddling the end of a mapping
// ---------------------------------------------------------------------------
fn err04_pipl_read_straddles_mapping_end() {
    assert_same_fault("pipl_straddle");
}

// ---------------------------------------------------------------------------
// Row 5 — PROT_NONE / write-only mapping
// ---------------------------------------------------------------------------
fn err05_pipl_unreadable_mapping() {
    assert_same_fault("pipl_protnone");

    // PROT_WRITE without PROT_READ is not expressible on x86_64, so the kernel
    // may or may not make the page readable. Either way both libraries must do
    // the same thing, which is what is asserted.
    let (c, r) = run_isolated_both("pipl_writeonly");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "PROT_WRITE-only page: termination diverged C={c:?} Rust={r:?}"
    );
    assert_eq!(c.stdout, r.stdout, "PROT_WRITE-only page: output diverged");
}

// ---------------------------------------------------------------------------
// Row 6 — bad(): the unconditional uninitialised read
// ---------------------------------------------------------------------------
fn err06_bad_uninitialised_read() {
    // No input can avoid the defect, and the outcome is unspecified, so the
    // assertion is that the two implementations agree on WHETHER they survive
    // and, when they do, that the output has the exact shape printf("%d\n")
    // produces. Byte equality is not asserted because the C library is not
    // byte-equal to itself across runs (see ERRORS.md).
    for _ in 0..6 {
        let (c, r) = assert_same_termination("bad");
        if c.crashed() {
            assert_eq!(c.signal, r.signal);
            assert!(c.stdout.is_empty() && r.stdout.is_empty());
        } else {
            assert!(is_one_int_line(&c.stdout), "C bad(): {:?}", c.text());
            assert!(is_one_int_line(&r.stdout), "Rust bad(): {:?}", r.text());
        }
    }

    // The residue-controlled variants of the same defect ARE specified, and
    // there byte equality is asserted. These are the checks that would catch a
    // translation that reads a different stack slot.
    assert_same_isolated("bad_bad");
    assert_same_isolated("good_bad");
    for v in [0i32, 1, -1, 12345, i32::MIN, i32::MAX] {
        assert_same_isolated(&format!("pipl_bad:{v}"));
    }
}

// ---------------------------------------------------------------------------
// Row 7 — driver(0): the same defect one frame deeper
// ---------------------------------------------------------------------------
fn err07_driver_zero_reaches_the_defect() {
    for _ in 0..6 {
        let (c, r) = assert_same_termination("driver:0");
        if c.crashed() {
            assert!(c.stdout.is_empty() && r.stdout.is_empty());
        } else {
            assert!(is_one_int_line(&c.stdout), "C driver(0): {:?}", c.text());
            assert!(is_one_int_line(&r.stdout), "Rust driver(0): {:?}", r.text());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — out-of-range / unexpected int for useGood is NOT rejected
// ---------------------------------------------------------------------------
fn err08_driver_out_of_range_values_accepted() {
    // `driver` takes `int`, not an enum, so there is no invalid value. Every
    // non-zero bit pattern must take the `good` arm in both implementations; a
    // translation that compared against 1, or that validated the argument, fails
    // here.
    let (c, r) = (load_c(), load_rust());
    let mut vals = vec![
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        2,
        3,
        1000,
        i32::MAX - 1,
        i32::MAX,
        // Bit patterns a C enum-domain check would reject.
        0x7FFF_FFFF,
        -0x8000_0000,
        0x0001_0000,
        0x00FF_00FF,
    ];
    vals.extend(boundary_i32s().into_iter().filter(|&v| v != 0));
    for v in vals {
        let spec = format!("driver:{v}");
        let (out_c, out_r) = run_in_process(&c, &r, &spec);
        assert_eq!(out_c, out_r, "{spec} diverged");
        assert_eq!(
            out_c, b"5\n",
            "{spec}: non-zero must take the good arm and print 5, not be rejected"
        );
    }

    // And exactly zero — the one value that behaves differently — is still not
    // "rejected"; it just runs the other arm.
    let (oc, _) = assert_same_termination("driver:0");
    assert!(
        oc.code == Some(0) || oc.crashed(),
        "driver(0) either prints garbage or faults; it never reports an error"
    );
}

// ---------------------------------------------------------------------------
// Row 9 — garbage in the upper 32 bits of rdi
// ---------------------------------------------------------------------------
fn err09_driver_wide_rdi_upper_bits_ignored() {
    let (c, r) = (load_c(), load_rust());

    // Upper bits set, lower bits non-zero: good arm.
    for v in [
        0x0000_0001_0000_0001u64,
        0xFFFF_FFFF_0000_0001,
        0xDEAD_BEEF_0000_0005,
        0xFFFF_FFFF_FFFF_FFFF,
    ] {
        let spec = format!("driver_wide:{v}");
        let (out_c, out_r) = run_in_process(&c, &r, &spec);
        assert_eq!(out_c, out_r, "{spec} diverged");
        assert_eq!(out_c, b"5\n", "{spec}: low half non-zero => good arm");
    }

    // Upper bits set, lower 32 bits zero: `cmpl $0,-0x4(%rbp)` sees zero, so the
    // bad arm runs even though rdi is non-zero. A translation that tested the
    // whole 64-bit register would take the good arm and print 5.
    for v in [
        0x0000_0001_0000_0000u64,
        0xFFFF_FFFF_0000_0000,
        0xDEAD_BEEF_0000_0000,
    ] {
        let spec = format!("driver_wide:{v}");
        let (oc, or) = assert_same_termination(&spec);
        for (out, who) in [(&oc, "C"), (&or, "Rust")] {
            assert_ne!(
                out.stdout,
                b"5\n",
                "{who} {spec}: low half is zero, so the bad arm must run, \
                 not the good arm"
            );
            if !out.crashed() {
                assert!(
                    is_one_int_line(&out.stdout),
                    "{who} {spec}: {:?}",
                    out.text()
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — good() has no reachable failure
// ---------------------------------------------------------------------------
fn err10_good_cannot_fail() {
    let (oc, or) = run_isolated_both("good");
    assert_eq!((oc.code, oc.signal), (Some(0), None), "C good() must succeed");
    assert_eq!(
        (or.code, or.signal),
        (Some(0), None),
        "Rust good() must succeed"
    );
    assert_eq!(oc.stdout, or.stdout);
    assert_eq!(oc.stdout, b"5\n");

    // Repeated calls stay infallible.
    let (c, r) = (load_c(), load_rust());
    for _ in 0..64 {
        assert_same_in_process(&c, &r, "good");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — printf's return value is discarded (unwritable stdout)
// ---------------------------------------------------------------------------
fn err11_unwritable_stdout_is_not_reported() {
    // driver.c:30 ignores printf's result, so a failing write must not change
    // the library's behaviour: both must still return normally. The child is run
    // with fd 1 pointing at a closed/unwritable target by giving it an output
    // path under a directory that does not exist, which makes the child's own
    // `open` fail identically for both libraries.
    //
    // Rather than relying on that, drive it in-process: redirect fd 1 to
    // /dev/full, where every write fails with ENOSPC.
    use std::os::unix::io::AsRawFd;

    let (c, r) = (load_c(), load_rust());
    let dev_full = match std::fs::OpenOptions::new().write(true).open("/dev/full") {
        Ok(f) => f,
        Err(_) => {
            eprintln!("skipping: /dev/full unavailable");
            return;
        }
    };

    let run = |api: &Api| -> bool {
        unsafe {
            let saved = libc_dup(1);
            libc_fflush();
            libc_dup2(dev_full.as_raw_fd(), 1);
            let v: core::ffi::c_int = 424242;
            (api.print_int_ptr_line)(&v as *const core::ffi::c_int);
            // The flush is where ENOSPC surfaces; the library must not care.
            libc_fflush();
            libc_dup2(saved, 1);
            libc_close(saved);
        }
        true // reaching here at all is the assertion: no abort, no error path
    };
    assert!(run(&c), "C survived an unwritable stdout");
    assert!(run(&r), "Rust survived an unwritable stdout");

    // Both are still functional afterwards and still agree.
    assert_same_in_process(&c, &r, "pipl:99");
}

unsafe extern "C" {
    #[link_name = "dup"]
    fn libc_dup(fd: i32) -> i32;
    #[link_name = "dup2"]
    fn libc_dup2(old: i32, new: i32) -> i32;
    #[link_name = "close"]
    fn libc_close(fd: i32) -> i32;
}
unsafe fn libc_fflush() {
    unsafe extern "C" {
        fn fflush(s: *mut core::ffi::c_void) -> i32;
    }
    unsafe { fflush(std::ptr::null_mut()) };
}

// ---------------------------------------------------------------------------
// Row 12 — no enum exists; the nearest analogue is the unrestricted int domain
// ---------------------------------------------------------------------------
fn err12_no_enum_domain_but_check_the_int_domain_anyway() {
    // The task asks for out-of-range enum values across the FFI boundary. This
    // library declares no enum (`grep -c enum` over both C files is 0), so the
    // check is applied to the only integer parameter there is: `driver`'s
    // `useGood`. Values one step past every "documented" value (0 and 1) and
    // past the type's range are all exercised, including the values a caller
    // would produce by passing a wider or narrower type.
    let (c, r) = (load_c(), load_rust());

    // One step past the two meaningful values.
    for v in [-1i32, 2] {
        let spec = format!("driver:{v}");
        let (out_c, out_r) = run_in_process(&c, &r, &spec);
        assert_eq!(out_c, out_r, "{spec} diverged");
        assert_eq!(out_c, b"5\n");
    }

    // Truncation behaviour: values that are non-zero as u64 but zero as i32, and
    // vice versa. Confirms both read exactly 32 bits.
    for v in [0x1_0000_0000u64, 0x0000_0000_FFFF_FFFF, 0x1_0000_0001] {
        let spec = format!("driver_wide:{v}");
        let low_is_zero = (v as u32) == 0;
        let (oc, or) = assert_same_termination(&spec);
        if !low_is_zero {
            assert_eq!(oc.stdout, b"5\n", "{spec}: C");
            assert_eq!(or.stdout, b"5\n", "{spec}: Rust");
        } else {
            assert_ne!(oc.stdout, b"5\n", "{spec}: C must take the bad arm");
            assert_ne!(or.stdout, b"5\n", "{spec}: Rust must take the bad arm");
        }
    }

    // Every single-bit pattern in the 32-bit domain: none is rejected.
    for bit in 0..32u32 {
        let v = 1i32.wrapping_shl(bit);
        let spec = format!("driver:{v}");
        let (out_c, out_r) = run_in_process(&c, &r, &spec);
        assert_eq!(out_c, out_r, "{spec} diverged");
        assert_eq!(out_c, b"5\n", "{spec}: no bit pattern is invalid");
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries required regardless of the table
// ---------------------------------------------------------------------------
fn generic_null_and_extreme_pointers() {
    // Null, one-past-null, the maximum address, and a sampling of addresses in
    // between. All must fault identically; none may be silently tolerated by one
    // side only.
    let mut rng = Rng::new(0xB0_1111);
    let mut specs: Vec<String> = vec![
        "pipl_addr:0".into(),
        "pipl_addr:1".into(),
        "pipl_addr:7".into(),
        "pipl_addr:18446744073709551615".into(),
        // Just past the canonical userspace boundary on x86_64.
        "pipl_addr:140737488355328".into(),
        "pipl_addr:9223372036854775808".into(),
    ];
    for _ in 0..6 {
        // Random low addresses inside the never-mapped first page.
        specs.push(format!("pipl_addr:{}", rng.below(4096)));
    }
    for spec in specs {
        let (c, r) = run_isolated_both(&spec);
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "{spec}: termination diverged C={c:?} Rust={r:?}"
        );
        assert_eq!(c.stdout, r.stdout, "{spec}: output diverged");
    }
}

fn generic_zero_and_oversized_lengths_do_not_exist() {
    // The API has no length, size or count parameter anywhere (the only
    // parameters are one `const int *` and one `int`), so there is no
    // zero-length or oversized-length input to construct. What can be checked is
    // that the read width really is fixed at 4 bytes and identical in both: a
    // 4-byte-only buffer at the very end of a mapping succeeds, while starting
    // one byte later straddles and faults.
    let (c, r) = (load_c(), load_rust());
    for word in [0u32, 1, u32::MAX, 0x1234_5678] {
        let spec = format!("pipl_unaligned:{word}:0");
        assert_same_in_process(&c, &r, &spec);
    }
    // The straddling case is the "oversized read" analogue.
    assert_same_fault("pipl_straddle");
}

// ---------------------------------------------------------------------------
// Entry point (`harness = false`; see the comment in Cargo.toml)
// ---------------------------------------------------------------------------
fn main() -> ! {
    common::run_tests(driver_tests())
}

fn driver_tests() -> &'static [common::Test] {
    tests![
        err01_pipl_null,
        err02_pipl_unmapped_addresses,
        err03_pipl_misaligned_is_not_an_error,
        err04_pipl_read_straddles_mapping_end,
        err05_pipl_unreadable_mapping,
        err06_bad_uninitialised_read,
        err07_driver_zero_reaches_the_defect,
        err08_driver_out_of_range_values_accepted,
        err09_driver_wide_rdi_upper_bits_ignored,
        err10_good_cannot_fail,
        err11_unwritable_stdout_is_not_reported,
        err12_no_enum_domain_but_check_the_int_domain_anyway,
        generic_null_and_extreme_pointers,
        generic_zero_and_oversized_lengths_do_not_exist,
    ]
}
