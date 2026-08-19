// Phase C — error-path / rejection differential tests.
//
// One test per row of ERRORS.md. The C library performs *no* validation at all
// (see ERRORS.md for the mechanical derivation), so "same rejection" is checked
// as: the same stdout bytes, the same output-buffer contents, and — for the rows
// where the C faults — the same terminating signal number, obtained by running
// each library's call in a forked child and comparing the `waitpid` status.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Fork-and-compare `driver` on both libraries: identical termination
/// disposition *and* identical stdout.
fn diff_forked_driver(label: &str, data: &[c_int], len: c_int) {
    let p = data.as_ptr();
    let (cd, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(p, len) });
    let (rd, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(p, len) });
    assert_eq!(
        cd, rd,
        "[{label}] driver(len={len}): C {cd}, Rust {rd} — must fail/return identically"
    );
    assert!(
        co == ro,
        "[{label}] driver(len={len}) stdout mismatch: {}",
        describe_diff(&co, &ro)
    );
}

/// Fork-and-compare `fma_array` on both libraries with raw pointers.
fn diff_forked_fma(
    label: &str,
    make: impl Fn() -> (*mut c_int, *const c_int, *const c_int, *const c_int),
    len: c_int,
) {
    let (o, a, b, c) = make();
    let (cd, co) = run_in_child_capturing(|| unsafe { (c_lib().fma_array)(o, a, b, c, len) });
    let (o, a, b, c) = make();
    let (rd, ro) = run_in_child_capturing(|| unsafe { (rust_lib().fma_array)(o, a, b, c, len) });
    assert_eq!(
        cd, rd,
        "[{label}] fma_array(len={len}): C {cd}, Rust {rd} — must fail/return identically"
    );
    assert!(
        co == ro,
        "[{label}] fma_array(len={len}) stdout mismatch: {}",
        describe_diff(&co, &ro)
    );
}

const NULLI: *const c_int = std::ptr::null();
const NULLM: *mut c_int = std::ptr::null_mut();

// ===========================================================================
// Row 1 — fma_array, len == 0, valid pointers: no memory access at all
// ===========================================================================
#[test]
fn err_01_fma_len_zero_no_writes() {
    let mut rng = Rng::new(101);
    for rep in 0..32 {
        let mut scratch = vec![0 as c_int; 32];
        rng.fill_full(&mut scratch);
        let before = scratch.clone();
        for lib in [c_lib(), rust_lib()] {
            let mut buf = before.clone();
            let p = buf.as_mut_ptr();
            let out = capture_stdout(|| unsafe {
                (lib.fma_array)(p, p.add(8), p.add(16), p.add(24), 0)
            });
            assert_eq!(
                buf, before,
                "[row1 rep={rep}] {} modified the buffer for len==0",
                lib.name
            );
            assert!(out.is_empty(), "[row1] {} printed for len==0", lib.name);
        }
        diff_fma_layout(&format!("row1 rep={rep}"), &before, (0, 8, 16, 24), 0);
    }
}

// ===========================================================================
// Row 2 — fma_array, len == 0, all four pointers NULL: must return normally
// ===========================================================================
#[test]
fn err_02_fma_len_zero_all_null() {
    diff_forked_fma("row2", || (NULLM, NULLI, NULLI, NULLI), 0);
    // and assert the exact disposition, not merely "the same"
    let d = run_in_child(|| unsafe { (c_lib().fma_array)(NULLM, NULLI, NULLI, NULLI, 0) });
    assert_eq!(d, Disposition::Exited(0), "C must not deref NULL for len==0");
    let d = run_in_child(|| unsafe { (rust_lib().fma_array)(NULLM, NULLI, NULLI, NULLI, 0) });
    assert_eq!(d, Disposition::Exited(0), "Rust must not deref NULL for len==0");
}

// ===========================================================================
// Row 3 — fma_array, len < 0, valid pointers: silently does nothing
// ===========================================================================
#[test]
fn err_03_fma_len_negative_no_writes() {
    let mut rng = Rng::new(103);
    for &len in &[-1, -2, -7, -1000, -65_536, INT_MIN, INT_MIN + 1] {
        for rep in 0..8 {
            let mut scratch = vec![0 as c_int; 32];
            rng.fill_full(&mut scratch);
            let before = scratch.clone();
            for lib in [c_lib(), rust_lib()] {
                let mut buf = before.clone();
                let p = buf.as_mut_ptr();
                let out = capture_stdout(|| unsafe {
                    (lib.fma_array)(p, p.add(8), p.add(16), p.add(24), len)
                });
                assert_eq!(
                    buf, before,
                    "[row3 len={len} rep={rep}] {} modified the buffer",
                    lib.name
                );
                assert!(out.is_empty(), "[row3] {} printed for len<0", lib.name);
            }
            diff_fma_layout(&format!("row3 len={len} rep={rep}"), &before, (0, 8, 16, 24), len);
        }
    }
}

// ===========================================================================
// Row 4 — fma_array, len < 0, all pointers NULL: still no deref
// ===========================================================================
#[test]
fn err_04_fma_len_negative_all_null() {
    for &len in &[-1, -2, -7, -1000, INT_MIN] {
        diff_forked_fma(&format!("row4 len={len}"), || (NULLM, NULLI, NULLI, NULLI), len);
        let d = run_in_child(|| unsafe { (c_lib().fma_array)(NULLM, NULLI, NULLI, NULLI, len) });
        assert_eq!(d, Disposition::Exited(0), "C, len={len}");
        let d = run_in_child(|| unsafe { (rust_lib().fma_array)(NULLM, NULLI, NULLI, NULLI, len) });
        assert_eq!(d, Disposition::Exited(0), "Rust, len={len}");
    }
}

// ===========================================================================
// Rows 5-8 — fma_array with one NULL pointer and len > 0 -> SIGSEGV
// ===========================================================================
fn null_ptr_row(label: &str, which: usize) {
    for &len in &[1, 2, 8, 1024] {
        let make = move || {
            // A fresh leaked buffer per child so the parent's state is untouched.
            let b: &'static mut [c_int] = Box::leak(vec![7 as c_int; 2048].into_boxed_slice());
            let p = b.as_mut_ptr();
            let mut o: *mut c_int = p;
            let mut a: *const c_int = p;
            let mut m: *const c_int = p;
            let mut d: *const c_int = p;
            match which {
                0 => o = NULLM,
                1 => a = NULLI,
                2 => m = NULLI,
                _ => d = NULLI,
            }
            (o, a, m, d)
        };
        diff_forked_fma(&format!("{label} len={len}"), make, len);
        // ...and assert it really is a fault, not a silent success.
        let (o, a, m, d) = make();
        let disp = run_in_child(|| unsafe { (c_lib().fma_array)(o, a, m, d, len) });
        assert_eq!(
            disp,
            Disposition::Signaled(libc::SIGSEGV),
            "{label}: expected the C library to fault with SIGSEGV, got {disp}"
        );
    }
}

#[test]
fn err_05_fma_out_null() {
    null_ptr_row("row5 out=NULL", 0);
}

#[test]
fn err_06_fma_mul1_null() {
    null_ptr_row("row6 mul1=NULL", 1);
}

#[test]
fn err_07_fma_mul2_null() {
    null_ptr_row("row7 mul2=NULL", 2);
}

#[test]
fn err_08_fma_add_null() {
    null_ptr_row("row8 add=NULL", 3);
}

// ===========================================================================
// Row 9 — fma_array, len one past the end of guard-page-protected buffers
// ===========================================================================
#[test]
fn err_09_fma_len_one_past_end_guarded() {
    for &n in &[1usize, 2, 3, 8, 100, 1024] {
        let len = n as c_int + 1;

        // (a) 4-way aliased, exactly the pattern `inner` uses
        let make_alias = move || {
            let g: &'static mut GuardedInts = Box::leak(Box::new(GuardedInts::new(n)));
            let mut r = Rng::new(109 + n as u64);
            r.fill_full(g.as_mut_slice());
            let p = g.ptr();
            (p, p as *const c_int, p as *const c_int, p as *const c_int)
        };
        diff_forked_fma(&format!("row9a n={n}"), make_alias, len);
        let (o, a, m, d) = make_alias();
        let disp = run_in_child(|| unsafe { (c_lib().fma_array)(o, a, m, d, len) });
        assert_eq!(
            disp,
            Disposition::Signaled(libc::SIGSEGV),
            "row9a n={n}: expected the C library to fault"
        );

        // (b) four independent guarded buffers
        let make_sep = move || {
            let mk = |seed: u64| -> *mut c_int {
                let g: &'static mut GuardedInts = Box::leak(Box::new(GuardedInts::new(n)));
                let mut r = Rng::new(seed);
                r.fill_full(g.as_mut_slice());
                g.ptr()
            };
            (
                mk(1),
                mk(2) as *const c_int,
                mk(3) as *const c_int,
                mk(4) as *const c_int,
            )
        };
        diff_forked_fma(&format!("row9b n={n}"), make_sep, len);
    }
}

// ===========================================================================
// Row 10 — fma_array with an oversized len (INT_MAX) over a small buffer
// ===========================================================================
#[test]
fn err_10_fma_len_int_max() {
    for &len in &[INT_MAX, INT_MAX - 1, 1 << 24, 1 << 20] {
        let make = || {
            let g: &'static mut GuardedInts = Box::leak(Box::new(GuardedInts::new(64)));
            let p = g.ptr();
            (p, p as *const c_int, p as *const c_int, p as *const c_int)
        };
        diff_forked_fma(&format!("row10 len={len}"), make, len);
        let (o, a, m, d) = make();
        let disp = run_in_child(|| unsafe { (c_lib().fma_array)(o, a, m, d, len) });
        assert_eq!(
            disp,
            Disposition::Signaled(libc::SIGSEGV),
            "row10 len={len}: expected the C library to fault"
        );
    }
}

// ===========================================================================
// Row 11 — driver, len == 0 with valid data: no output, returns normally
// ===========================================================================
#[test]
fn err_11_driver_len_zero() {
    let mut rng = Rng::new(111);
    for rep in 0..32 {
        let mut data = vec![0 as c_int; 16];
        rng.fill_full(&mut data);
        for lib in [c_lib(), rust_lib()] {
            let out = capture_stdout(|| unsafe { (lib.driver)(data.as_ptr(), 0) });
            assert!(
                out.is_empty(),
                "[row11 rep={rep}] {} printed {:?} for len==0",
                lib.name,
                hex_head(&out, 64)
            );
        }
        diff_driver(&format!("row11 rep={rep}"), &data, 0);
    }
    assert_eq!(
        run_in_child(|| unsafe { (c_lib().driver)([1, 2, 3].as_ptr(), 0) }),
        Disposition::Exited(0)
    );
    assert_eq!(
        run_in_child(|| unsafe { (rust_lib().driver)([1, 2, 3].as_ptr(), 0) }),
        Disposition::Exited(0)
    );
}

// ===========================================================================
// Row 12 — driver, len == 0 and data == NULL: memcpy of 0 bytes, no deref
// ===========================================================================
#[test]
fn err_12_driver_len_zero_null_data() {
    let (cd, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(NULLI, 0) });
    let (rd, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(NULLI, 0) });
    assert_eq!(cd, Disposition::Exited(0), "C: driver(NULL, 0) must succeed");
    assert_eq!(rd, Disposition::Exited(0), "Rust: driver(NULL, 0) must succeed");
    assert_eq!(cd, rd);
    assert!(co.is_empty() && ro.is_empty(), "no output expected");
    assert_eq!(co, ro);
    // Also in-process (proves neither implementation touches the NULL pointer).
    for lib in [c_lib(), rust_lib()] {
        let out = capture_stdout(|| unsafe { (lib.driver)(NULLI, 0) });
        assert!(out.is_empty(), "{} printed for driver(NULL,0)", lib.name);
    }
}

// ===========================================================================
// Row 13 — driver, len > 0 and data == NULL -> SIGSEGV
// ===========================================================================
#[test]
fn err_13_driver_null_data() {
    for &len in &[1, 2, 3, 8, 1024, 65_536] {
        diff_forked_driver_raw(&format!("row13 len={len}"), NULLI, len);
        let disp = run_in_child(|| unsafe { (c_lib().driver)(NULLI, len) });
        assert_eq!(
            disp,
            Disposition::Signaled(libc::SIGSEGV),
            "row13 len={len}: expected the C library to fault"
        );
    }
}

fn diff_forked_driver_raw(label: &str, data: *const c_int, len: c_int) {
    let (cd, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(data, len) });
    let (rd, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(data, len) });
    assert_eq!(
        cd, rd,
        "[{label}] driver(len={len}): C {cd}, Rust {rd} — must fail identically"
    );
    assert!(
        co == ro,
        "[{label}] stdout mismatch: {}",
        describe_diff(&co, &ro)
    );
}

// ===========================================================================
// Rows 14-16 — driver with a negative len
//
// `int out[len]` with `len < 0` makes gcc move `%rsp` *upwards* by
// `round16(4*|len|)` bytes, into `driver`'s caller's frame (see the disassembly
// quoted in tests/negative_len_analysis.rs). Everything the C does afterwards
// therefore writes over the caller's stack, and whether the process then dies,
// spins, or returns is decided by the *caller's* frame layout rather than by
// anything the C source states -- `d_neg_01` in tests/negative_len_analysis.rs
// demonstrates that directly by calling the same C `.so` from four call sites
// that differ only in how much stack the caller has in use.
//
// The parts that are well defined, and that are compared exactly here, are:
//   * neither library prints anything (`inner` skips both loops when len < 0);
//   * the byte count is `(size_t)(len * sizeof(int))`, i.e. an unsatisfiable
//     ~2^64, so the copy cannot succeed and the Rust must fault rather than
//     silently pretend the call worked;
//   * the Rust must be deterministic (it must not inherit the C's caller-frame
//     sensitivity).
// ===========================================================================
fn negative_len_row(label: &str, len: c_int) {
    let data: Vec<c_int> = (0..64).collect();
    let p = data.as_ptr();

    // Same output (none) from both.
    let (cd, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(p, len) });
    let (rd, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(p, len) });
    assert!(
        co.is_empty(),
        "{label}: C printed {:?} but `inner` skips both loops for len<0",
        hex_head(&co, 80)
    );
    assert_eq!(
        co, ro,
        "{label}: stdout mismatch (C {:?} vs Rust {:?})",
        hex_head(&co, 80),
        hex_head(&ro, 80)
    );

    // The C's own termination is caller-frame determined (proven by
    // `d_neg_01_c_outcome_depends_on_the_callers_frame` in
    // tests/negative_len_analysis.rs, which observes the same C `.so` both
    // `exited(0)` and `SIGSEGV` for identical arguments), so it is recorded
    // rather than asserted.
    println!("{label}: C terminated as {cd} from this call site (not comparable)");

    assert_eq!(
        rd,
        Disposition::Signaled(libc::SIGSEGV),
        "{label}: the Rust library must fault on the unsatisfiable copy, got {rd}"
    );

    // The Rust must give the same answer every time.
    for _ in 0..3 {
        let d = run_in_child(|| unsafe { (rust_lib().driver)(p, len) });
        assert_eq!(d, rd, "{label}: the Rust library is not deterministic");
    }
}

#[test]
fn err_14_driver_len_negative_one() {
    negative_len_row("row14 len=-1", -1);
}

#[test]
fn err_15_driver_len_negative_seven() {
    negative_len_row("row15 len=-7", -7);
    negative_len_row("row15 len=-2", -2);
    negative_len_row("row15 len=-1000", -1000);
    negative_len_row("row15 len=-65536", -65_536);
}

#[test]
fn err_16_driver_len_int_min() {
    negative_len_row("row16 len=INT_MIN", INT_MIN);
    negative_len_row("row16 len=INT_MIN+1", INT_MIN + 1);
}

// ===========================================================================
// Row 17 — driver with an oversized positive len
// ===========================================================================
#[test]
fn err_17_driver_len_int_max() {
    let data: Vec<c_int> = (0..64).collect();
    for &len in &[INT_MAX, INT_MAX - 1, 1 << 30, 1 << 28, 1 << 24] {
        diff_forked_driver(&format!("row17 len={len}"), &data, len);
        let p = data.as_ptr();
        let disp = run_in_child(|| unsafe { (c_lib().driver)(p, len) });
        assert_eq!(
            disp,
            Disposition::Signaled(libc::SIGSEGV),
            "row17 len={len}: expected the C library to fault, got {disp}"
        );
    }
}

// ===========================================================================
// Row 18 — driver with len one past the end of a guarded source buffer
// ===========================================================================
#[test]
fn err_18_driver_len_one_past_end_guarded() {
    for &n in &[1usize, 2, 3, 4, 8, 16, 100, 1000] {
        let len = n as c_int + 1;
        let mk = || -> *const c_int {
            let g: &'static mut GuardedInts = Box::leak(Box::new(GuardedInts::new(n)));
            let mut r = Rng::new(118 + n as u64);
            r.fill_full(g.as_mut_slice());
            g.ptr() as *const c_int
        };
        let p1 = mk();
        let (cd, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(p1, len) });
        let p2 = mk();
        let (rd, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(p2, len) });
        assert_eq!(cd, rd, "row18 n={n}: C {cd}, Rust {rd}");
        assert_eq!(
            cd,
            Disposition::Signaled(libc::SIGSEGV),
            "row18 n={n}: expected the C library to fault reading past the end"
        );
        assert!(co == ro, "row18 n={n} stdout: {}", describe_diff(&co, &ro));
    }
}

// ===========================================================================
// Row 19 — driver, INT_MIN element (signed-overflow boundary, unchecked)
// ===========================================================================
#[test]
fn err_19_driver_int_min_overflow() {
    let data = [INT_MIN];
    diff_driver("row19", &data, 1);
    let out = capture_stdout(|| unsafe { (c_lib().driver)(data.as_ptr(), 1) });
    assert_eq!(
        out, b"-2147483648\n",
        "C ground truth changed: INT_MIN*INT_MIN + INT_MIN must wrap to INT_MIN"
    );
    let out_r = capture_stdout(|| unsafe { (rust_lib().driver)(data.as_ptr(), 1) });
    assert_eq!(out, out_r);
    // repeated / in an array
    let arr = [INT_MIN; 9];
    diff_driver("row19 array", &arr, 9);
}

// ===========================================================================
// Row 20 — driver, INT_MAX element (signed-overflow boundary, unchecked)
// ===========================================================================
#[test]
fn err_20_driver_int_max_overflow() {
    let data = [INT_MAX];
    diff_driver("row20", &data, 1);
    let out = capture_stdout(|| unsafe { (c_lib().driver)(data.as_ptr(), 1) });
    // INT_MAX*INT_MAX == (2^31-1)^2 == 2^62 - 2^32 + 1, which wraps to 1;
    // 1 + INT_MAX == 0x80000000 == INT_MIN.
    assert_eq!(out, b"-2147483648\n", "C ground truth changed");
    let out_r = capture_stdout(|| unsafe { (rust_lib().driver)(data.as_ptr(), 1) });
    assert_eq!(out, out_r);
    let arr = [INT_MAX; 9];
    diff_driver("row20 array", &arr, 9);
}

// ===========================================================================
// Row 21 — fma_array signed-overflow boundaries, independent pointers
// ===========================================================================
#[test]
fn err_21_fma_overflow_boundaries() {
    let interesting: &[c_int] = &[
        INT_MIN,
        INT_MIN + 1,
        -65_537,
        -65_536,
        -46_341,
        -46_340,
        -2,
        -1,
        0,
        1,
        2,
        46_340,
        46_341,
        65_536,
        65_537,
        INT_MAX - 1,
        INT_MAX,
    ];
    // Full cross product of (mul1, mul2, add) over the boundary values, driven
    // through the low-level entry point with three distinct source buffers.
    let n = interesting.len();
    let mut m1 = Vec::new();
    let mut m2 = Vec::new();
    let mut ad = Vec::new();
    for &a in interesting {
        for &b in interesting {
            for &c in interesting {
                m1.push(a);
                m2.push(b);
                ad.push(c);
            }
        }
    }
    let count = m1.len();
    assert_eq!(count, n * n * n);
    // scratch layout: [out][m1][m2][ad]
    let mut scratch = vec![0 as c_int; 4 * count];
    for i in 0..count {
        scratch[i] = 0x0BAD_BEEFu32 as c_int;
        scratch[count + i] = m1[i];
        scratch[2 * count + i] = m2[i];
        scratch[3 * count + i] = ad[i];
    }
    diff_fma_layout(
        "row21 full-cross",
        &scratch,
        (0, count, 2 * count, 3 * count),
        count as c_int,
    );
    // Also verify against the two's-complement model.
    let mut buf = scratch.clone();
    let p = buf.as_mut_ptr();
    unsafe { (c_lib().fma_array)(p, p.add(count), p.add(2 * count), p.add(3 * count), count as c_int) };
    for i in 0..count {
        let want = m1[i].wrapping_mul(m2[i]).wrapping_add(ad[i]);
        assert_eq!(buf[i], want, "row21: element {i} ({},{},{})", m1[i], m2[i], ad[i]);
    }
}

// ===========================================================================
// Row 22 — driver with a misaligned `data` pointer (no alignment check)
// ===========================================================================
#[test]
fn err_22_driver_misaligned_data() {
    let mut rng = Rng::new(122);
    for shift in 1..8usize {
        for &len in &[1, 2, 5, 33] {
            let mut raw = vec![0u8; len as usize * 4 + 16];
            for b in raw.iter_mut() {
                *b = (rng.next_u64() & 0xFF) as u8;
            }
            let p = unsafe { raw.as_ptr().add(shift) } as *const c_int;
            let c = capture_stdout(|| unsafe { (c_lib().driver)(p, len) });
            let r = capture_stdout(|| unsafe { (rust_lib().driver)(p, len) });
            assert!(
                c == r,
                "[row22 shift={shift} len={len}] mismatch: {}",
                describe_diff(&c, &r)
            );
            assert!(
                !c.is_empty(),
                "[row22] the misaligned pointer must still be accepted (memcpy is \
                 alignment-agnostic), but nothing was printed"
            );
        }
    }
}

// ===========================================================================
// Row 23 — fma_array with `out` overlapping the sources at a non-zero offset
// ===========================================================================
#[test]
fn err_23_fma_overlapping_offset() {
    let mut rng = Rng::new(123);
    for &(oo, so) in &[(0usize, 1usize), (1, 0), (0, 2), (2, 0), (0, 3), (3, 0)] {
        for &len in &[1, 2, 3, 4, 8, 33, 64] {
            for rep in 0..8 {
                let n = len as usize;
                let mut scratch = vec![0 as c_int; n + 8];
                rng.fill_full(&mut scratch);
                let label = format!("row23 out+{oo} src+{so} len={len} rep={rep}");
                diff_fma_layout(&label, &scratch, (oo, so, so, so), len);

                // Model: strictly ascending, read-then-write, in place.
                let mut model = scratch.clone();
                for i in 0..n {
                    let a = model[so + i];
                    model[oo + i] = a.wrapping_mul(a).wrapping_add(a);
                }
                let mut buf = scratch.clone();
                let p = buf.as_mut_ptr();
                unsafe {
                    (c_lib().fma_array)(p.add(oo), p.add(so), p.add(so), p.add(so), len);
                }
                assert_eq!(
                    buf, model,
                    "[{label}] the C library's overlap semantics are not \
                     strictly-ascending in-place any more"
                );
            }
        }
    }
}

// ===========================================================================
// Generic FFI boundary sweep required by Phase C.
//
// The public API has NO enum parameters (grep -n "enum" over c_src finds
// nothing), so the "out-of-range enum value across the FFI boundary" class
// degenerates to "an arbitrary `int` in the `len` parameter". This test sweeps a
// wide, deliberately hostile set of `int` bit patterns through `len` on BOTH
// entry points and requires identical dispositions and identical stdout.
// ===========================================================================
#[test]
fn err_24_wild_int_len_sweep() {
    let mut lens: Vec<c_int> = vec![
        INT_MIN,
        INT_MIN + 1,
        INT_MIN + 2,
        -0x4000_0000,
        -0x1_0000,
        -1000,
        -256,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        7,
        8,
        15,
        16,
        17,
        31,
        32,
        63,
        64,
    ];
    for k in 0..31 {
        lens.push(1 << k);
        lens.push(-(1 << k));
    }
    lens.push(INT_MAX - 1);
    lens.push(INT_MAX);
    lens.sort_unstable();
    lens.dedup();

    // A 4096-element source, so every len in 0..=4096 is a *valid* call and
    // everything beyond it is out of range; both must match.
    const N: usize = 4096;
    let mut rng = Rng::new(124);
    let mut data = vec![0 as c_int; N];
    rng.fill_small(&mut data);
    let dp = data.as_ptr();

    // One guard-page-backed copy of the source, reused for every out-of-range
    // length: an overrun then *must* fault, at the same offset, before anything
    // is printed, which makes those lengths exactly comparable (with a plain heap
    // buffer they would not be -- see d_oob_01 / d_oob_04).
    let mut guarded = GuardedInts::new(N);
    guarded.as_mut_slice().copy_from_slice(&data);
    let gp = guarded.ptr() as *const c_int;

    for &len in &lens {
        if len >= 0 && (len as usize) <= N {
            // ---- in range: fully specified, and safe to run in-process -------
            diff_driver(&format!("row24 driver len={len}"), &data, len);
            diff_fma_layout(&format!("row24 fma len={len}"), &data, (0, 0, 0, 0), len);

            let mut a = data.clone();
            let mut b = data.clone();
            let (pa, pb) = (a.as_mut_ptr(), b.as_mut_ptr());
            unsafe {
                (c_lib().fma_array)(pa, dp, dp, dp, len);
                (rust_lib().fma_array)(pb, dp, dp, dp, len);
            }
            assert_eq!(a, b, "row24 fma_array len={len}: destination mismatch");
        } else if len > 0 {
            // ---- out of range, made deterministic with a guard page ---------
            let (cd, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(gp, len) });
            let (rd, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(gp, len) });
            assert_eq!(
                cd, rd,
                "row24 driver len={len} (guarded {N}-element source): C {cd}, Rust {rd}"
            );
            assert_eq!(
                cd,
                Disposition::Signaled(libc::SIGSEGV),
                "row24 driver len={len}: the guarded out-of-range read must fault"
            );
            assert_eq!(co, ro, "row24 driver len={len}: stdout mismatch");
        } else {
            // ---- negative -----------------------------------------------------
            // `driver`: only the specified half is comparable (see the note above
            // rows 14-16); run it in a child because it destroys its own stack.
            let (_, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(dp, len) });
            let (rdisp, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(dp, len) });
            assert!(
                co.is_empty() && ro.is_empty(),
                "row24 driver len={len}: neither library may print for len<0"
            );
            assert_eq!(co, ro, "row24 driver len={len}: stdout mismatch");
            assert_eq!(
                rdisp,
                Disposition::Signaled(libc::SIGSEGV),
                "row24 driver len={len}: the Rust library must fault"
            );

            // `fma_array` has no VLA, so a negative length there is fully
            // specified: the loop guard is false and nothing at all happens.
            let mut a = data.clone();
            let mut b = data.clone();
            let (pa, pb) = (a.as_mut_ptr(), b.as_mut_ptr());
            let co = capture_stdout(|| unsafe { (c_lib().fma_array)(pa, dp, dp, dp, len) });
            let ro = capture_stdout(|| unsafe { (rust_lib().fma_array)(pb, dp, dp, dp, len) });
            assert!(co.is_empty() && ro.is_empty(), "fma_array must never print");
            assert_eq!(a, data, "row24 fma_array len={len}: C modified the buffer");
            assert_eq!(b, data, "row24 fma_array len={len}: Rust modified the buffer");
        }
    }
}

// ===========================================================================
// Extra: NULL in every subset of fma_array's four pointers, for len 0 and 1.
// (2^4 = 16 combinations x 2 lens; a superset of rows 2, 4, 5-8.)
// ===========================================================================
#[test]
fn err_25_fma_null_pointer_powerset() {
    for mask in 0u32..16 {
        for &len in &[0 as c_int, -1, 1, 4] {
            let make = move || {
                let b: &'static mut [c_int] = Box::leak(vec![3 as c_int; 64].into_boxed_slice());
                let p = b.as_mut_ptr();
                let o = if mask & 1 != 0 { NULLM } else { p };
                let a = if mask & 2 != 0 { NULLI } else { p as *const c_int };
                let m = if mask & 4 != 0 { NULLI } else { p as *const c_int };
                let d = if mask & 8 != 0 { NULLI } else { p as *const c_int };
                (o, a, m, d)
            };
            diff_forked_fma(&format!("row25 mask={mask:04b} len={len}"), make, len);
        }
    }
}
