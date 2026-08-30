//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! Both `driver` and `fma_array` return `void` and the C contains no error
//! macro, sentinel, enum or `assert` (see `ERRORS.md` for the mechanical grep).
//! The observable "result" of a rejection is therefore the triple
//!   (did it return at all?, what did it write?, what did it print?)
//! and every test below asserts all three agree between the two `.so`s — not
//! merely that "both failed somehow".
//!
//! The two rows whose C behaviour is a fatal signal (E12, E14) are pinned in a
//! forked subprocess so the exact disposition (exit code vs. terminating
//! signal) is compared rather than guessed.

mod common;

use common::*;
use std::ffi::c_int;
use std::ptr;

// ---------------------------------------------------------------------------
// Subprocess helper for the rows where the C faults
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn open(path: *const i8, flags: c_int, ...) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn alarm(seconds: u32) -> u32;
}

/// `SIGALRM`, i.e. "the child was still running when the watchdog fired".
const SIGALRM: i32 = 14;

/// How a child process terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Disposition {
    Exited(i32),
    Signalled(i32),
}

impl Disposition {
    fn hung(self) -> bool {
        self == Disposition::Signalled(SIGALRM)
    }
}

/// Run `f` in a forked child with stdout on `/dev/null` and a watchdog alarm,
/// and report how the child terminated. Used for the rows where the C invokes
/// UB, so that the exact disposition is measured rather than assumed.
fn disposition_of<F: FnOnce()>(f: F) -> Disposition {
    disposition_with_timeout(f, 20)
}

fn disposition_with_timeout<F: FnOnce()>(f: F, secs: u32) -> Disposition {
    use std::io::Write;
    std::io::stdout().flush().ok();

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: silence stdout so a 1e6-line run does not flood the log, and
        // arm a watchdog because some UB inputs make the C run essentially
        // forever (a ~1.8e19-byte memcpy).
        let devnull = c"/dev/null".as_ptr();
        let fd = unsafe { open(devnull, 1 /* O_WRONLY */) };
        if fd >= 0 {
            unsafe { dup2(fd, 1) };
        }
        unsafe { alarm(secs) };
        f();
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    if status & 0x7f == 0 {
        Disposition::Exited((status >> 8) & 0xff)
    } else {
        Disposition::Signalled(status & 0x7f)
    }
}

// ===========================================================================
// E1 — fma_array, len == 0: loop guard false on entry, nothing written
// ===========================================================================

#[test]
fn err_e1_fma_len_zero() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..200 {
        let lay = Layout::distinct(16);
        let init = Values::Full.fill(&mut rng, lay.arena);

        let run = |imp: &Impl| -> Vec<i32> {
            let mut buf = init.clone();
            let base = buf.as_mut_ptr();
            let f = imp.fma_sym();
            unsafe {
                f(
                    base.add(lay.out),
                    base.add(lay.mul1),
                    base.add(lay.mul2),
                    base.add(lay.add),
                    0,
                );
            }
            buf
        };
        let got_c = run(&p.c);
        let got_rust = run(&p.rust);

        // Same result on both sides ...
        assert_eq!(as_bytes(&got_c), as_bytes(&got_rust), "E1: C/Rust diverge");
        // ... and that result is "wrote absolutely nothing".
        assert_eq!(as_bytes(&got_c), as_bytes(&init), "E1: C wrote with len=0");
        assert_eq!(
            as_bytes(&got_rust),
            as_bytes(&init),
            "E1: Rust wrote with len=0"
        );
    }
}

// ===========================================================================
// E2 — fma_array, len < 0: silent no-op, NOT an error return
// ===========================================================================

#[test]
fn err_e2_fma_len_negative() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xE2);

    // Every interesting negative bit pattern, incl. one step below the valid
    // boundary (-1) and the extreme (INT_MIN).
    let mut lens: Vec<c_int> = vec![-1, -2, -3, -5, -16, -1000, i32::MIN, i32::MIN + 1];
    for _ in 0..40 {
        lens.push(-(rng.range(1, 1 << 20) as c_int));
    }

    let makers: [fn(usize) -> Layout; 6] = [
        Layout::distinct,
        Layout::out_eq_mul1,
        Layout::out_eq_mul2,
        Layout::out_eq_add,
        Layout::mul1_eq_mul2,
        Layout::all_same,
    ];

    for &len in &lens {
        for mk in makers {
            let lay = mk(8);
            let init = Values::Full.fill(&mut rng, lay.arena);

            let run = |imp: &Impl| -> Vec<i32> {
                let mut buf = init.clone();
                let base = buf.as_mut_ptr();
                let f = imp.fma_sym();
                unsafe {
                    f(
                        base.add(lay.out),
                        base.add(lay.mul1),
                        base.add(lay.mul2),
                        base.add(lay.add),
                        len,
                    );
                }
                buf
            };
            let got_c = run(&p.c);
            let got_rust = run(&p.rust);

            assert_eq!(
                as_bytes(&got_c),
                as_bytes(&got_rust),
                "E2: C/Rust diverge for len={len}"
            );
            assert_eq!(
                as_bytes(&got_c),
                as_bytes(&init),
                "E2: C wrote something for len={len}"
            );
        }
    }
}

// ===========================================================================
// E3 — fma_array, len == 0 with every pointer NULL: never dereferenced
// ===========================================================================

#[test]
fn err_e3_fma_len_zero_all_null() {
    let p = pair();
    // If either side dereferenced, the process would fault instead of
    // returning; reaching the assertion below is itself the comparison.
    let d_c = disposition_of(|| {
        let f = p.c.fma_sym();
        unsafe { f(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0) };
    });
    let d_rust = disposition_of(|| {
        let f = p.rust.fma_sym();
        unsafe { f(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0) };
    });
    assert_eq!(d_c, Disposition::Exited(0), "E3: C did not return cleanly");
    assert_eq!(d_c, d_rust, "E3: disposition diverges");

    // Also exercise it in-process (both must simply return).
    unsafe {
        (p.c.fma_sym())(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
        (p.rust.fma_sym())(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
    }

    // Mixed partial-NULL with len == 0 is equally safe on both sides.
    let mut real = vec![1i32, 2, 3, 4];
    let rp = real.as_mut_ptr();
    for imp in [&p.c, &p.rust] {
        let f = imp.fma_sym();
        unsafe {
            f(rp, ptr::null(), ptr::null(), ptr::null(), 0);
            f(ptr::null_mut(), rp, ptr::null(), ptr::null(), 0);
            f(ptr::null_mut(), ptr::null(), rp, ptr::null(), 0);
            f(ptr::null_mut(), ptr::null(), ptr::null(), rp, 0);
        }
    }
    assert_eq!(real, vec![1, 2, 3, 4], "E3: buffer touched with len=0");
}

// ===========================================================================
// E4 — fma_array, len < 0 with every pointer NULL
// ===========================================================================

#[test]
fn err_e4_fma_len_negative_all_null() {
    let p = pair();
    for &len in &[-1, -2, -1000, i32::MIN] {
        let d_c = disposition_of(|| {
            let f = p.c.fma_sym();
            unsafe { f(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), len) };
        });
        let d_rust = disposition_of(|| {
            let f = p.rust.fma_sym();
            unsafe { f(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), len) };
        });
        assert_eq!(
            d_c,
            Disposition::Exited(0),
            "E4: C did not return cleanly for len={len}"
        );
        assert_eq!(d_c, d_rust, "E4: disposition diverges for len={len}");
    }
}

// ===========================================================================
// E7 — multiply overflow wraps two's-complement
// ===========================================================================

#[test]
fn err_e7_fma_mul_overflow() {
    let p = pair();

    // Hand-picked vectors that overflow only in the multiply.
    let cases: &[(i32, i32, i32)] = &[
        (i32::MAX, 2, 1),
        (i32::MAX, i32::MAX, 0),
        (i32::MIN, 2, 0),
        (i32::MIN, -1, 0),
        (i32::MIN, i32::MIN, 0),
        (65536, 65536, 0),
        (46341, 46341, 0),
        (-46341, 46341, 0),
        (i32::MAX, -1, 0),
        (1 << 30, 4, 0),
    ];

    for &(a, b, c) in cases {
        let mut init = vec![0i32; 4];
        init[1] = a;
        init[2] = b;
        init[3] = c;
        let lay = Layout { arena: 4, out: 0, mul1: 1, mul2: 2, add: 3 };

        let run = |imp: &Impl| -> i32 {
            let mut buf = init.clone();
            let base = buf.as_mut_ptr();
            let f = imp.fma_sym();
            unsafe { f(base, base.add(1), base.add(2), base.add(3), 1) };
            buf[0]
        };
        let got_c = run(&p.c);
        let got_rust = run(&p.rust);
        assert_eq!(
            got_c, got_rust,
            "E7: {a} * {b} + {c} -> C {got_c} vs Rust {got_rust}"
        );
        // Pin the C's observed semantics: two's-complement wrap.
        assert_eq!(
            got_c,
            a.wrapping_mul(b).wrapping_add(c),
            "E7: C is not wrapping for {a} * {b} + {c}"
        );
            let _ = lay;
    }

    // The specific value verified against a native C probe:
    // INT_MAX * 2 + 1 == -1.
    let mut init = vec![0i32, i32::MAX, 2, 1];
    let run = |imp: &Impl| -> i32 {
        let mut buf = init.clone();
        let base = buf.as_mut_ptr();
        unsafe { (imp.fma_sym())(base, base.add(1), base.add(2), base.add(3), 1) };
        buf[0]
    };
    assert_eq!(run(&p.c), -1, "E7: C reference value changed");
    assert_eq!(run(&p.rust), -1, "E7: Rust does not match C reference value");
    init.clear();
}

// ===========================================================================
// E8 — add overflow wraps two's-complement
// ===========================================================================

#[test]
fn err_e8_fma_add_overflow() {
    let p = pair();

    // Overflow originates in the addition.
    let cases: &[(i32, i32, i32)] = &[
        (1, 1, i32::MAX),
        (1, 1, i32::MIN),
        (-1, 1, i32::MIN),
        (i32::MAX, 1, 1),
        (i32::MAX, 1, i32::MAX),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, 1, i32::MIN),
        (2, 3, i32::MAX - 5),
        (0, 0, i32::MIN),
    ];

    for &(a, b, c) in cases {
        let init = vec![0i32, a, b, c];
        let run = |imp: &Impl| -> i32 {
            let mut buf = init.clone();
            let base = buf.as_mut_ptr();
            unsafe { (imp.fma_sym())(base, base.add(1), base.add(2), base.add(3), 1) };
            buf[0]
        };
        let got_c = run(&p.c);
        let got_rust = run(&p.rust);
        assert_eq!(
            got_c, got_rust,
            "E8: {a} * {b} + {c} -> C {got_c} vs Rust {got_rust}"
        );
        assert_eq!(
            got_c,
            a.wrapping_mul(b).wrapping_add(c),
            "E8: C is not wrapping for {a} * {b} + {c}"
        );
    }

    // Verified against a native C probe:
    // INT_MAX * INT_MAX + INT_MAX == INT_MIN.
    let init = vec![0i32, i32::MAX, i32::MAX, i32::MAX];
    let run = |imp: &Impl| -> i32 {
        let mut buf = init.clone();
        let base = buf.as_mut_ptr();
        unsafe { (imp.fma_sym())(base, base.add(1), base.add(2), base.add(3), 1) };
        buf[0]
    };
    assert_eq!(run(&p.c), i32::MIN, "E8: C reference value changed");
    assert_eq!(run(&p.rust), i32::MIN, "E8: Rust does not match C reference");
}

// ===========================================================================
// E10 — driver, len == 0: prints nothing
// ===========================================================================

#[test]
fn err_e10_driver_len_zero() {
    let mut rng = Rng::new(SEED ^ 0xE10);
    for _ in 0..100 {
        let n = rng.range(1, 40) as usize;
        let data = Values::Full.fill(&mut rng, n);
        let out = diff_driver("E10", &data, 0);
        assert!(
            out.is_empty(),
            "E10: len=0 must produce empty stdout, got {:?}",
            String::from_utf8_lossy(&out)
        );
    }
}

// ===========================================================================
// E11 — driver, len == 0 and data == NULL
// ===========================================================================

#[test]
fn err_e11_driver_len_zero_null_data() {
    let p = pair();

    let run = |imp: &Impl| -> Vec<u8> {
        let f = imp.driver_sym();
        capture_stdout(|| unsafe { f(ptr::null(), 0) })
    };
    let out_c = run(&p.c);
    let out_rust = run(&p.rust);

    assert_eq!(out_c, out_rust, "E11: stdout diverges");
    assert!(out_c.is_empty(), "E11: C printed something for (NULL, 0)");
    assert!(out_rust.is_empty(), "E11: Rust printed something for (NULL, 0)");

    // And neither faults, in a fresh process.
    let d_c = disposition_of(|| unsafe { (p.c.driver_sym())(ptr::null(), 0) });
    let d_rust = disposition_of(|| unsafe { (p.rust.driver_sym())(ptr::null(), 0) });
    assert_eq!(d_c, Disposition::Exited(0), "E11: C faulted on (NULL, 0)");
    assert_eq!(d_c, d_rust, "E11: disposition diverges");
}

// ===========================================================================
// E12 — driver, len < 0: C invokes UB and dies; Rust returns benignly
// ===========================================================================

/// `driver(data, len)` with `len < 0` computes `len * sizeof(int)` where the
/// negative `int` converts to `size_t`, so `memcpy` is asked to copy about
/// 1.8e19 bytes (measured: `len = -1` yields `18446744073709551612`). This is
/// unconditional UB and the C has **no reproducible result**: measured over a
/// range of lengths it variously segfaults, takes SIGBUS, runs forever, or
/// returns cleanly, depending purely on the caller's stack layout and the
/// process memory map. See the table in ERRORS.md.
///
/// There is therefore no C error code or sentinel for the Rust to match. What
/// this test pins down is:
///   * the Rust side is *deterministic* and benign for every negative length;
///   * wherever the C does manage to return normally — the only case in which
///     a comparison is even meaningful — the Rust returns normally too, and
///     both print nothing.
#[test]
fn err_e12_driver_len_negative() {
    let p = pair();
    let data = vec![1i32, 2, 3, 4, 5, 6, 7, 8];
    let lens: [c_int; 8] = [-1, -2, -7, -16, -100, -1000, i32::MIN + 1, i32::MIN];

    let mut observed = Vec::new();
    for &len in &lens {
        let d_c = disposition_with_timeout(
            || unsafe { (p.c.driver_sym())(data.as_ptr(), len) },
            5,
        );
        let d_rust = disposition_with_timeout(
            || unsafe { (p.rust.driver_sym())(data.as_ptr(), len) },
            5,
        );
        observed.push((len, d_c, d_rust));

        // The Rust side must be deterministic, benign and silent — never a
        // crash and never a hang.
        assert_eq!(
            d_rust,
            Disposition::Exited(0),
            "E12: Rust must handle len={len} benignly, got {d_rust:?}"
        );
        assert!(!d_rust.hung(), "E12: Rust hung for len={len}");

        // Where the C returns normally, the two agree exactly (both silent).
        if d_c == Disposition::Exited(0) {
            assert_eq!(
                d_c, d_rust,
                "E12: C returned normally for len={len} but Rust did not"
            );
            let out_rust =
                capture_stdout(|| unsafe { (p.rust.driver_sym())(data.as_ptr(), len) });
            assert!(
                out_rust.is_empty(),
                "E12: C returned silently for len={len} but Rust printed {:?}",
                String::from_utf8_lossy(&out_rust)
            );
        } else {
            // Otherwise the C died or hung: UB with nothing to compare against.
            assert!(
                matches!(d_c, Disposition::Signalled(_)),
                "E12: unexpected C disposition {d_c:?} for len={len}"
            );
        }

        // Rust prints nothing regardless, matching the len == 0 behaviour.
        let out = capture_stdout(|| unsafe { (p.rust.driver_sym())(data.as_ptr(), len) });
        assert!(out.is_empty(), "E12: Rust printed something for len={len}");
    }

    // Guard the documentation: at least one length must still show the C
    // faulting, and at least one showed it returning, which is exactly the
    // "no reproducible result" claim ERRORS.md makes.
    assert!(
        observed
            .iter()
            .any(|&(_, c, _)| matches!(c, Disposition::Signalled(_))),
        "E12: the C never faulted on a negative length; ERRORS.md needs updating. \
         Observed: {observed:?}"
    );
}

// ===========================================================================
// E14 — driver, len large enough to overflow the C's VLA stack allocation
// ===========================================================================

#[test]
fn err_e14_driver_len_stack_overflow() {
    let p = pair();
    // `data` is genuinely this long, so the only UB the C hits is the VLA size.
    let len: usize = 1_000_000;
    let data = vec![3i32; len];

    let d_c = disposition_of(|| unsafe { (p.c.driver_sym())(data.as_ptr(), len as c_int) });
    let d_rust = disposition_of(|| unsafe { (p.rust.driver_sym())(data.as_ptr(), len as c_int) });

    assert!(
        matches!(d_c, Disposition::Signalled(_)),
        "E14: C no longer overflows its stack for len={len}; it reported {d_c:?}. \
         ERRORS.md must be updated if the C behaviour changed."
    );
    assert_eq!(
        d_rust,
        Disposition::Exited(0),
        "E14: Rust heap-allocates and must survive len={len}, got {d_rust:?}"
    );
}

// ===========================================================================
// E16 — inner's full self-aliasing fma_array(out, out, out, out, len)
// ===========================================================================

#[test]
fn err_e16_inner_self_aliasing() {
    let mut rng = Rng::new(SEED ^ 0xE16);

    for _ in 0..150 {
        let len = rng.range(1, 64) as usize;
        let data: Vec<i32> = (0..len)
            .map(|_| {
                if rng.below(2) == 0 {
                    rng.next_i32()
                } else {
                    rng.pick(&BOUNDARY_VALUES)
                }
            })
            .collect();

        // driver must print exactly x*x + x for each element, on both sides.
        let out = diff_driver("E16", &data, len as c_int);
        let expected: String = data
            .iter()
            .map(|&x| format!("{}\n", x.wrapping_mul(x).wrapping_add(x)))
            .collect();
        assert_eq!(
            String::from_utf8_lossy(&out),
            expected,
            "E16: self-aliased result is not x*x + x"
        );

        // And the low-level entry point reproduces it for the same layout.
        let lay = Layout::all_same(len);
        let mut init = data.clone();
        init.resize(lay.arena, 0);
        diff_fma("E16/low-level", &init, lay, len as c_int);
    }
}

// ===========================================================================
// Generic boundaries required by Phase C (see the table at the end of
// ERRORS.md), including out-of-range values crossing the FFI boundary.
// ===========================================================================

/// The C API declares no enum parameter, so the analogue of "out-of-range enum
/// value" here is an arbitrary `int` bit pattern in the `len` position. Every
/// pattern that is safe to observe (`len <= 0`, plus `len` within the real
/// buffer) must behave identically on both sides.
#[test]
fn err_generic_len_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xB0);

    let mut lens: Vec<c_int> = vec![
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0000,
        -1_000_000,
        -65_537,
        -65_536,
        -257,
        -256,
        -2,
        -1,
        0,
    ];
    for _ in 0..64 {
        // Random negative patterns, i.e. arbitrary "invalid enum"-style ints.
        let v = rng.next_i32();
        lens.push(if v > 0 { -v } else { v });
    }

    let buf_len = 32usize;
    for &len in &lens {
        // fma_array
        let lay = Layout::distinct(buf_len);
        let init = Values::Boundary.fill(&mut rng, lay.arena);
        let run_fma = |imp: &Impl| -> Vec<i32> {
            let mut b = init.clone();
            let base = b.as_mut_ptr();
            unsafe {
                (imp.fma_sym())(
                    base.add(lay.out),
                    base.add(lay.mul1),
                    base.add(lay.mul2),
                    base.add(lay.add),
                    len,
                )
            };
            b
        };
        assert_eq!(
            as_bytes(&run_fma(&p.c)),
            as_bytes(&run_fma(&p.rust)),
            "generic: fma_array diverges for len={len}"
        );

        // driver — only the non-faulting patterns can be observed in-process.
        if len == 0 {
            let data = Values::Full.fill(&mut rng, buf_len);
            let out = diff_driver("generic", &data, 0);
            assert!(out.is_empty());
        }
    }
}

/// Zero and oversized lengths at the exact boundary of a real buffer: `len`
/// equal to the buffer size is valid, `len` of 0 is the empty boundary. (One
/// past the buffer is row E9/E15 — unguarded OOB, documented not asserted.)
#[test]
fn err_generic_len_exact_buffer_boundary() {
    let mut rng = Rng::new(SEED ^ 0xB1);
    for n in [1usize, 2, 3, 8, 33, 64, 255, 256] {
        let data = Values::Full.fill(&mut rng, n);
        // Exactly the buffer length: valid.
        diff_driver("boundary/full", &data, n as c_int);
        // One below: valid, must print n-1 lines.
        if n >= 1 {
            let out = diff_driver("boundary/n-1", &data, (n - 1) as c_int);
            assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), n - 1);
        }
        // Zero: valid, prints nothing.
        let out = diff_driver("boundary/zero", &data, 0);
        assert!(out.is_empty());

        let lay = Layout::distinct(n);
        let init = Values::Full.fill(&mut rng, lay.arena);
        diff_fma("boundary/fma-full", &init, lay, n as c_int);
        diff_fma("boundary/fma-n-1", &init, lay, (n - 1) as c_int);
        diff_fma("boundary/fma-zero", &init, lay, 0);
    }
}

/// Neither `.so` may export the `static` C function `inner`.
#[test]
fn err_static_inner_not_exported() {
    let p = pair();
    assert!(!p.c.has_inner(), "C .so unexpectedly exports `inner`");
    assert!(
        !p.rust.has_inner(),
        "Rust .so exports `inner`, but it is `static` in the C"
    );
}
