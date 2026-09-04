//! Phase C -- error / rejection-path differential tests.
//!
//! One test per row of ERRORS.md (E1..E16). Rows whose trigger makes the C
//! library invoke undefined behaviour that kills the process (E11/E12/E13, and
//! the `len == INT_MAX` out-of-range-index case) are executed OUT OF PROCESS so
//! the exact termination signal of each implementation can be compared.

mod common;
use common::*;
use std::ffi::c_int;

const SENTINEL: c_int = 0x5A5A_5A5A;

// ===========================================================================
// E1 -- fma_array, len == 0: the loop guard rejects everything.
// ===========================================================================
#[test]
fn err_e1_fma_len_zero_is_noop() {
    let p = pair();
    let m1 = [1i32, 2, 3, 4];
    let m2 = [5i32, 6, 7, 8];
    let ad = [9i32, 10, 11, 12];

    for imp in [&p.c, &p.rs] {
        let mut out = vec![SENTINEL; 4];
        unsafe { (imp.fma_array)(out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 0) };
        assert_eq!(
            out,
            vec![SENTINEL; 4],
            "E1: {} wrote to `out` even though len == 0",
            imp.name
        );
    }
    // And identical for every aliasing configuration and value shape.
    let mut rng = Rng::new(1);
    for &a in ALL_ALIASES {
        let (nbufs, _) = alias_layout(a);
        for &shape in ALL_SHAPES {
            let bufs: Vec<Vec<c_int>> = (0..nbufs).map(|_| gen_vals(shape, 8, &mut rng)).collect();
            diff_fma_alias(p, a, &bufs, 0, &format!("E1 a={a:?} shape={shape:?}"));
        }
    }
}

// ===========================================================================
// E2 -- fma_array, len < 0. `len` is never converted to a size here, so the
// loop guard `0 < len` is simply false: a no-op, NOT a crash.
// ===========================================================================
#[test]
fn err_e2_fma_len_negative_is_noop() {
    let p = pair();
    let m1 = [1i32, 2, 3, 4];
    let m2 = [5i32, 6, 7, 8];
    let ad = [9i32, 10, 11, 12];

    for &len in &[-1i32, -2, -3, -100, -1_000_000, i32::MIN + 1, i32::MIN] {
        for imp in [&p.c, &p.rs] {
            let mut out = vec![SENTINEL; 4];
            unsafe {
                (imp.fma_array)(out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), len)
            };
            assert_eq!(
                out,
                vec![SENTINEL; 4],
                "E2: {} wrote to `out` for len == {len}",
                imp.name
            );
        }
        let mut rng = Rng::new(len as u64);
        for &a in ALL_ALIASES {
            let (nbufs, _) = alias_layout(a);
            let bufs: Vec<Vec<c_int>> =
                (0..nbufs).map(|_| gen_vals(Shape::FullRandom, 8, &mut rng)).collect();
            diff_fma_alias(p, a, &bufs, len, &format!("E2 len={len} a={a:?}"));
        }
    }
}

// ===========================================================================
// E3 -- fma_array, all four pointers NULL, len == 0.
// ===========================================================================
#[test]
fn err_e3_fma_all_null_len_zero() {
    let p = pair();
    for imp in [&p.c, &p.rs] {
        unsafe {
            (imp.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
            );
        }
        // Reaching here at all is the assertion: neither faults.
    }
}

// ===========================================================================
// E4 -- fma_array, all four pointers NULL, len < 0.
// ===========================================================================
#[test]
fn err_e4_fma_all_null_len_negative() {
    let p = pair();
    for &len in &[-1i32, -2, -1_000_000, i32::MIN] {
        for imp in [&p.c, &p.rs] {
            unsafe {
                (imp.fma_array)(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    len,
                );
            }
        }
    }
}

// ===========================================================================
// E5 -- fma_array, `out` valid but the input pointers NULL, len == 0.
// ===========================================================================
#[test]
fn err_e5_fma_partial_null_len_zero() {
    let p = pair();
    // Each of the 2^3 - 1 non-empty subsets of {mul1, mul2, add} set to NULL,
    // plus the case where only `out` is NULL.
    for mask in 1u32..8 {
        for imp in [&p.c, &p.rs] {
            let m1 = [1i32, 2, 3, 4];
            let m2 = [5i32, 6, 7, 8];
            let ad = [9i32, 10, 11, 12];
            let mut out = vec![SENTINEL; 4];
            let p1 = if mask & 1 != 0 { std::ptr::null() } else { m1.as_ptr() };
            let p2 = if mask & 2 != 0 { std::ptr::null() } else { m2.as_ptr() };
            let p3 = if mask & 4 != 0 { std::ptr::null() } else { ad.as_ptr() };
            unsafe { (imp.fma_array)(out.as_mut_ptr(), p1, p2, p3, 0) };
            assert_eq!(out, vec![SENTINEL; 4], "E5 mask={mask}: {} wrote to out", imp.name);
        }
    }
    // `out` NULL, inputs valid, len == 0.
    for imp in [&p.c, &p.rs] {
        let m1 = [1i32, 2, 3, 4];
        unsafe { (imp.fma_array)(std::ptr::null_mut(), m1.as_ptr(), m1.as_ptr(), m1.as_ptr(), 0) };
    }
}

// ===========================================================================
// E6 -- signed multiply overflow (UB in C; the built object wraps).
// ===========================================================================
#[test]
fn err_e6_fma_mul_overflow_wraps() {
    let p = pair();
    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        // (mul1, mul2, add, expected)
        (65536, 65536, 0, 0),                 // 2^32 wraps to 0
        (65536, 65536, 7, 7),
        (46341, 46341, 0, -2147479015),       // first product past INT_MAX
        (46340, 46340, 0, 2147395600),        // last product that fits
        (-46341, 46341, 0, 2147479015),
        (-46341, -46341, 0, -2147479015),
        (100000, 100000, 0, 1410065408),
        (i32::MAX, 2, 0, -2),
        (i32::MAX, i32::MAX, 0, 1),
        (i32::MAX, 3, 0, 2147483645),
        (0x10000, 0x10001, 0, 65536),
    ];
    for &(a, b, c, want) in cases {
        let m1 = [a];
        let m2 = [b];
        let ad = [c];
        let mut oc = [SENTINEL];
        let mut or = [SENTINEL];
        unsafe {
            (p.c.fma_array)(oc.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 1);
            (p.rs.fma_array)(or.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 1);
        }
        assert_eq!(oc[0], or[0], "E6 divergence: {a} * {b} + {c}");
        assert_eq!(oc[0], want, "E6: C reference value changed for {a} * {b} + {c}");
    }
}

// ===========================================================================
// E7 -- signed add overflow (UB in C; the built object wraps).
// ===========================================================================
#[test]
fn err_e7_fma_add_overflow_wraps() {
    let p = pair();
    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (i32::MAX, 1, 1, i32::MIN),           // INT_MAX + 1 wraps
        (i32::MAX, 1, i32::MAX, -2),
        (i32::MIN, 1, -1, i32::MAX),          // INT_MIN - 1 wraps
        (i32::MIN, 1, i32::MIN, 0),
        (1, 1, i32::MAX, i32::MIN),           // 1 + INT_MAX == INT_MIN
        (2, 3, i32::MAX, i32::MIN + 5),
        (-2, 3, i32::MIN, i32::MAX - 5),
    ];
    for &(a, b, c, want) in cases {
        let m1 = [a];
        let m2 = [b];
        let ad = [c];
        let mut oc = [SENTINEL];
        let mut or = [SENTINEL];
        unsafe {
            (p.c.fma_array)(oc.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 1);
            (p.rs.fma_array)(or.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 1);
        }
        assert_eq!(oc[0], or[0], "E7 divergence: {a} * {b} + {c}");
        assert_eq!(oc[0], want, "E7: C reference value changed for {a} * {b} + {c}");
    }
}

// ===========================================================================
// E8 -- INT_MIN operands (non-representable negation).
// ===========================================================================
#[test]
fn err_e8_fma_int_min_operands() {
    let p = pair();
    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (i32::MIN, -1, 0, i32::MIN),
        (i32::MIN, 1, 0, i32::MIN),
        (i32::MIN, i32::MIN, 0, 0),
        (i32::MIN, 2, 0, 0),
        (i32::MIN, 3, 0, i32::MIN),
        (-1, i32::MIN, 0, i32::MIN),
        (i32::MIN, -1, i32::MIN, 0),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
    ];
    for &(a, b, c, want) in cases {
        let m1 = [a];
        let m2 = [b];
        let ad = [c];
        let mut oc = [SENTINEL];
        let mut or = [SENTINEL];
        unsafe {
            (p.c.fma_array)(oc.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 1);
            (p.rs.fma_array)(or.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 1);
        }
        assert_eq!(oc[0], or[0], "E8 divergence: {a} * {b} + {c}");
        assert_eq!(oc[0], want, "E8: C reference value changed for {a} * {b} + {c}");
    }
}

// ===========================================================================
// E9 -- driver, len == 0: no stdout bytes at all.
// ===========================================================================
#[test]
fn err_e9_driver_len_zero_no_output() {
    let p = pair();
    for data in [vec![], vec![0i32], vec![i32::MIN, i32::MAX], vec![1i32; 64]] {
        let bytes = diff_driver(p, &data, 0, "E9");
        assert!(bytes.is_empty(), "E9: expected zero stdout bytes, got {bytes:?}");
    }
}

// ===========================================================================
// E10 -- driver, data == NULL and len == 0: memcpy(dst, NULL, 0), no fault.
// ===========================================================================
#[test]
fn err_e10_driver_null_data_len_zero() {
    let p = pair();
    let _g = stdout_guard();
    for imp in [&p.c, &p.rs] {
        let (_, out) = capture_stdout(|| unsafe { (imp.driver)(std::ptr::null(), 0) });
        assert!(
            out.is_empty(),
            "E10: {} produced stdout for (NULL, 0): {out:?}",
            imp.name
        );
    }
}

// ===========================================================================
// E14 -- driver, len == 1: the smallest non-empty input.
// ===========================================================================
#[test]
fn err_e14_driver_len_one_boundary() {
    let p = pair();
    for &x in EXTREMES {
        let data = [x];
        let bytes = diff_driver(p, &data, 1, &format!("E14 x={x}"));
        let want = format!("{}\n", x.wrapping_mul(x).wrapping_add(x));
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            want,
            "E14: wrong single line for x={x}"
        );
    }
    // A longer buffer with len forced to 1: only element 0 may be printed.
    let data = [7i32, 8, 9, 10];
    let bytes = diff_driver(p, &data, 1, "E14 truncated");
    assert_eq!(String::from_utf8(bytes).unwrap(), "56\n");
}

// ===========================================================================
// E15 -- overflow values flowing through driver into `%d`.
// ===========================================================================
#[test]
fn err_e15_driver_overflow_values() {
    let p = pair();
    let data = [
        i32::MAX,
        i32::MIN,
        65536,
        -65536,
        46340,
        46341,
        -46340,
        -46341,
        i32::MAX - 1,
        i32::MIN + 1,
        0,
        1,
        -1,
    ];
    let bytes = diff_driver(p, &data, data.len() as c_int, "E15");
    let want: String = data
        .iter()
        .map(|&x| format!("{}\n", x.wrapping_mul(x).wrapping_add(x)))
        .collect();
    assert_eq!(String::from_utf8(bytes).unwrap(), want, "E15: %d rendering mismatch");
}

// ===========================================================================
// E16 -- there is no enum in this API; the only scalar is `int len`, so the
// "out-of-range value across the FFI boundary" case is every `int` extreme.
// The safe ones are checked in process here; INT_MAX / negatives for `driver`
// are the out-of-process rows E11 and E13.
// ===========================================================================
#[test]
fn err_e16_no_enums_int_extremes() {
    let p = pair();
    let m = [3i32, 4, 5, 6];
    // For `fma_array`, every non-positive `int` (including INT_MIN, which has no
    // representable negation) is safely a no-op in both implementations.
    for &len in &[0i32, -1, i32::MIN, i32::MIN + 1, -0x7fff_ffff] {
        for imp in [&p.c, &p.rs] {
            let mut out = vec![SENTINEL; 4];
            unsafe { (imp.fma_array)(out.as_mut_ptr(), m.as_ptr(), m.as_ptr(), m.as_ptr(), len) };
            assert_eq!(out, vec![SENTINEL; 4], "E16 len={len} impl={}", imp.name);
        }
    }
    // And for `driver`, len == 0 is the only non-UB extreme.
    let bytes = diff_driver(p, &m, 0, "E16 driver len=0");
    assert!(bytes.is_empty());
}

// ===========================================================================
// Out-of-process rows: E11, E12, E13, and fma_array with len == INT_MAX.
//
// The child is this same test binary re-invoked with DIFFTEST_UB set; the
// `ub_child_worker` test below performs the single UB call and never returns
// normally unless the implementation survives it.
// ===========================================================================

mod ub {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};

    #[derive(Debug, PartialEq, Eq)]
    pub enum Outcome {
        /// Terminated by signal N (11 == SIGSEGV, 6 == SIGABRT).
        Signal(i32),
        /// Ran to completion and printed the SURVIVED marker.
        Survived,
        /// Exited with a status but no marker.
        Exited(i32),
    }

    pub fn run(spec: &str) -> Outcome {
        run_with_stack(spec, None)
    }

    /// Run the child, optionally under an explicit `ulimit -s <kb>` so the VLA
    /// stack-overflow row (E13) has a deterministic threshold instead of one
    /// that depends on the ambient stack limit.
    pub fn run_with_stack(spec: &str, stack_kb: Option<u32>) -> Outcome {
        let exe = std::env::current_exe().expect("current_exe");
        let mut cmd;
        match stack_kb {
            None => {
                cmd = Command::new(exe);
                cmd.args(["--exact", "ub_child_worker", "--nocapture"]);
            }
            Some(kb) => {
                cmd = Command::new("/bin/sh");
                cmd.arg("-c").arg(format!(
                    "ulimit -s {kb}; exec '{}' --exact ub_child_worker --nocapture",
                    exe.display()
                ));
            }
        }
        let out = cmd
            .env("DIFFTEST_UB", spec)
            .env("RUST_TEST_THREADS", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to spawn child");

        if let Some(sig) = out.status.signal() {
            return Outcome::Signal(sig);
        }
        let text = String::from_utf8_lossy(&out.stderr);
        if text.contains("DIFFTEST_UB_SURVIVED") {
            Outcome::Survived
        } else {
            Outcome::Exited(out.status.code().unwrap_or(-1))
        }
    }

    impl Outcome {
        /// True if the process was killed by a memory-safety trap.
        ///
        /// A guard-page hit is delivered as `SIGSEGV`, but this test binary is a
        /// Rust program and the Rust runtime installs a `SIGSEGV` handler that
        /// recognises stack-guard faults, prints "has overflowed its stack" and
        /// calls `abort()`. So a stack overflow shows up as `SIGABRT` while a
        /// plain wild-pointer dereference (which the handler re-raises) shows up
        /// as `SIGSEGV`. Both mean "trapped".
        pub fn trapped(&self) -> bool {
            matches!(self, Outcome::Signal(11) | Outcome::Signal(6))
        }
    }
}

/// Child-side worker. A no-op unless `DIFFTEST_UB` is set, so it is harmless
/// during normal test runs.
#[test]
fn ub_child_worker() {
    let Ok(spec) = std::env::var("DIFFTEST_UB") else {
        return; // parent-side run: nothing to do
    };
    let mut it = spec.split(':');
    let which = it.next().unwrap();
    let case = it.next().unwrap();
    let len: i64 = it.next().unwrap().parse().unwrap();

    let p = pair();
    let imp = match which {
        "c" => &p.c,
        "rs" => &p.rs,
        other => panic!("bad impl {other}"),
    };

    // Send the library's stdout to /dev/null: for the huge-len case it would be
    // hundreds of megabytes, and its contents are not what this row asserts.
    unsafe {
        let devnull = b"/dev/null\0";
        let fd = libc_open(devnull.as_ptr() as *const std::ffi::c_char, 1 /* O_WRONLY */);
        if fd >= 0 {
            libc_dup2(fd, 1);
        }
    }

    match case {
        // driver with a negative len
        "driver_neg" => unsafe {
            let data = vec![1i32, 2, 3, 4];
            (imp.driver)(data.as_ptr(), len as c_int);
        },
        // driver with a NULL data pointer and a positive len
        "driver_null" => unsafe {
            (imp.driver)(std::ptr::null(), len as c_int);
        },
        // driver with a len whose VLA overflows the 8 MiB stack
        "driver_huge" => unsafe {
            let data = vec![1i32; len as usize];
            (imp.driver)(data.as_ptr(), len as c_int);
        },
        // fma_array with len == INT_MAX over a tiny buffer: an out-of-range
        // index walk that must fault in both implementations
        "fma_intmax" => unsafe {
            let m = vec![1i32; 16];
            let mut out = vec![0i32; 16];
            (imp.fma_array)(out.as_mut_ptr(), m.as_ptr(), m.as_ptr(), m.as_ptr(), len as c_int);
        },
        other => panic!("bad case {other}"),
    }

    eprintln!("DIFFTEST_UB_SURVIVED {spec}");
    // Skip libtest's own teardown/reporting so the marker is the only signal.
    std::process::exit(0);
}

unsafe extern "C" {
    #[link_name = "open"]
    fn libc_open(path: *const std::ffi::c_char, flags: c_int, ...) -> c_int;
    #[link_name = "dup2"]
    fn libc_dup2(a: c_int, b: c_int) -> c_int;
}

const SIGSEGV: i32 = 11;

// ---------------------------------------------------------------------------
// E11 -- driver with len < 0. `len * sizeof(int)` converts a negative int to
// size_t, so memcpy gets ~2^64 and the C process dies with SIGSEGV.
// DOCUMENTED DIVERGENCE (see ERRORS.md): the Rust translation clamps to 0.
// ---------------------------------------------------------------------------
#[test]
fn err_e11_driver_len_negative_c_traps() {
    for len in [-1i64, -2, -1000, -1_000_000, i32::MIN as i64] {
        let c = ub::run(&format!("c:driver_neg:{len}"));
        assert_eq!(
            c,
            ub::Outcome::Signal(SIGSEGV),
            "E11: the C library no longer traps on len={len}; \
             re-evaluate the documented divergence in ERRORS.md"
        );

        let rs = ub::run(&format!("rs:driver_neg:{len}"));
        assert_eq!(
            rs,
            ub::Outcome::Survived,
            "E11: the Rust library changed behaviour on len={len}"
        );
        // C emits nothing before dying, so Rust must not emit anything either:
        // the two never produce *different* bytes.
        let p = pair();
        let _g = stdout_guard();
        let data = vec![1i32, 2, 3, 4];
        let (_, out) = capture_stdout(|| unsafe { (p.rs.driver)(data.as_ptr(), len as c_int) });
        assert!(
            out.is_empty(),
            "E11: Rust produced stdout for len={len}, but C produces none before trapping: {out:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// E12 -- driver with data == NULL and len > 0: both must fault identically.
// ---------------------------------------------------------------------------
#[test]
fn err_e12_driver_null_data_len_positive() {
    for len in [1i64, 2, 4, 100, 1000] {
        let c = ub::run(&format!("c:driver_null:{len}"));
        let rs = ub::run(&format!("rs:driver_null:{len}"));
        assert_eq!(
            c,
            ub::Outcome::Signal(SIGSEGV),
            "E12: C did not SIGSEGV on (NULL, {len})"
        );
        assert_eq!(
            rs, c,
            "E12: len={len}: Rust outcome {rs:?} != C outcome {c:?} for (NULL, len)"
        );
    }
}

// ---------------------------------------------------------------------------
// E13 -- driver with a len whose VLA overflows the stack (measured threshold on
// this host: 2_000_000 survives, 2_100_000 faults, with an 8 MiB stack).
// DOCUMENTED DIVERGENCE: Rust heap-allocates and survives.
// ---------------------------------------------------------------------------
#[test]
fn err_e13_driver_len_huge_c_stack_overflow() {
    // The child runs with an explicit 2 MiB stack so the threshold does not
    // depend on the ambient `ulimit -s`.
    const STACK_KB: u32 = 2048;

    // Comfortably BELOW the limit (128 KiB VLA): both must survive.
    let below = 32_768i64;
    for w in ["c", "rs"] {
        let o = ub::run_with_stack(&format!("{w}:driver_huge:{below}"), Some(STACK_KB));
        assert_eq!(
            o,
            ub::Outcome::Survived,
            "E13: {w} unexpectedly failed at len={below} with a {STACK_KB} KiB stack"
        );
    }

    // Comfortably ABOVE the limit (8 MiB VLA in a 2 MiB stack): C's VLA
    // overflows the stack, Rust heap-allocates and survives.
    let above = 2_000_000i64;
    let c = ub::run_with_stack(&format!("c:driver_huge:{above}"), Some(STACK_KB));
    assert!(
        c.trapped(),
        "E13: C no longer traps at len={above} with a {STACK_KB} KiB stack (got {c:?}); \
         re-evaluate the documented divergence in ERRORS.md"
    );
    let rs = ub::run_with_stack(&format!("rs:driver_huge:{above}"), Some(STACK_KB));
    assert_eq!(
        rs,
        ub::Outcome::Survived,
        "E13: Rust changed behaviour at len={above} (documented divergence: it uses the heap)"
    );
}

// ---------------------------------------------------------------------------
// Generic boundary: fma_array with len == INT_MAX over a 16-element buffer.
// This is the out-of-range-index walk; both must fault the same way.
// ---------------------------------------------------------------------------
#[test]
fn err_fma_len_intmax_out_of_range_index() {
    let len = i32::MAX as i64;
    let c = ub::run(&format!("c:fma_intmax:{len}"));
    let rs = ub::run(&format!("rs:fma_intmax:{len}"));
    assert_eq!(c, ub::Outcome::Signal(SIGSEGV), "expected C to fault, got {c:?}");
    assert_eq!(rs, c, "len=INT_MAX: Rust outcome {rs:?} != C outcome {c:?}");
}
