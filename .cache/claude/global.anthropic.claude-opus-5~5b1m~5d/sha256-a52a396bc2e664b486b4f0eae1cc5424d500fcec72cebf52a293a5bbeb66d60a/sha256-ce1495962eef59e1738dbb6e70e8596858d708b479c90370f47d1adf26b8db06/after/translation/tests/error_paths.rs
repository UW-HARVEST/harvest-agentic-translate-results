// Phase C — error / rejection / degenerate-input differential tests.
// One test per row of ERRORS.md.
//
// `lib.c` has no error protocol at all (no sentinel, no errno, no assert, no
// null check), so the rejection surface is: guard conditions that no-op, loop
// guards that iterate zero times, the implicit malloc-failure path, and hard
// faults on invalid pointers. The faulting rows are compared in *subprocesses*
// so that the terminating signal itself can be asserted equal.

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const CASE_ENV: &str = "HATCH_CRASH_CASE";

// ---------------------------------------------------------------------------
// subprocess fault harness
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
}

fn run_case(case: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    // These children are *expected* to die from SIGSEGV. On a host whose
    // /proc/sys/kernel/core_pattern pipes to a core-dump handler (systemd-
    // coredump, apport, ...) each fault would otherwise cost seconds of core
    // writing and litter the system, so RLIMIT_CORE is zeroed via `sh` before
    // exec'ing the child. Using `sh -c '...' <exe> <args...>` keeps this
    // dependency-free ($0 is the exe, "$@" the args).
    let out = Command::new("sh")
        .arg("-c")
        .arg("ulimit -c 0 2>/dev/null; exec \"$0\" \"$@\"")
        .arg(exe)
        .args(["--exact", "zz_crash_child", "--test-threads=1", "--quiet"])
        .env(CASE_ENV, case)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn crash child");
    Outcome { code: out.status.code(), signal: out.status.signal() }
}

/// Both libraries must fault the same way (same signal) on the same bad input.
fn assert_same_fault(name: &str, expect_signal: i32) {
    let c = run_case(&format!("c:{name}"));
    let r = run_case(&format!("r:{name}"));
    assert_eq!(c, r, "{name}: C and Rust terminated differently (C={c:?} Rust={r:?})");
    assert_eq!(
        c.signal,
        Some(expect_signal),
        "{name}: expected signal {expect_signal}, got {c:?}"
    );
}

/// Sentinel exit code used by the child when the call returned normally.
const NO_FAULT: i32 = 42;

/// The child entry point. Does nothing during a normal test run.
#[test]
fn zz_crash_child() {
    let case = match std::env::var(CASE_ENV) {
        Ok(c) => c,
        Err(_) => return,
    };
    let l = libs();
    let api = match case.as_bytes()[0] {
        b'c' => &l.c,
        _ => &l.r,
    };
    let name = &case[2..];
    unsafe {
        match name {
            // E8: memmove/memset through a NULL array, guard TRUE.
            "e8_shift_null" => (api.shift_array_data)(std::ptr::null_mut(), 10, 3),
            // E9: `int value = *ptr` with ptr == NULL.
            "e9_ppd_null" => {
                let v = (api.process_pointer_data)(std::ptr::null_mut(), 3);
                std::hint::black_box(v);
            }
            // E25: memmove + records[i].value through a NULL pointer.
            "e25_records_null" => {
                let v = (api.manipulate_records)(std::ptr::null_mut(), 5, 2);
                std::hint::black_box(v);
            }
            // E27: `return op(a,b,c)` with op == NULL.
            "e27_apply_null" => {
                let v = (api.apply_operation)(std::ptr::null(), 1, 2, 3);
                std::hint::black_box(v);
            }
            // E28: op == a bogus, non-executable address.
            "e28_apply_bogus" => {
                let v = (api.apply_operation)(1usize as *const c_void, 1, 2, 3);
                std::hint::black_box(v);
            }
            // E25b: NULL records with shift==0 but a positive num_records:
            // no memmove, but the read loop still dereferences.
            "e25b_records_null_shift0" => {
                let v = (api.manipulate_records)(std::ptr::null_mut(), 5, 0);
                std::hint::black_box(v);
            }
            // E23b/E22b: `num_records - shift` overflows into a huge POSITIVE
            // loop bound (INT_MAX), so the read loop walks ~100 GB past the
            // buffer. Both libraries must fault identically.
            "e23b_records_bound_int_max" => {
                let mut buf = vec![DataRecord::zeroed(); 256];
                let v = (api.manipulate_records)(buf.as_mut_ptr(), -1, c_int::MIN);
                std::hint::black_box(v);
            }
            "e22b_records_num_int_min_shift1" => {
                let mut buf = vec![DataRecord::zeroed(); 256];
                let v = (api.manipulate_records)(buf.as_mut_ptr(), c_int::MIN, 1);
                std::hint::black_box(v);
            }
            // E35: OVERSIZED length — guard true with size == INT_MAX means
            // memmove of (INT_MAX-1)*4 == ~8 GiB out of a 4 KiB buffer.
            "e35_shift_oversized_size" => {
                let mut buf: Vec<c_int> = vec![0; 1024];
                (api.shift_array_data)(buf.as_mut_ptr(), c_int::MAX, 1);
            }
            // E36: OVERSIZED length — memmove of (INT_MAX-1)*48 == ~96 GiB.
            "e36_records_oversized_num" => {
                let mut buf = vec![DataRecord::zeroed(); 256];
                let v = (api.manipulate_records)(buf.as_mut_ptr(), c_int::MAX, 1);
                std::hint::black_box(v);
            }
            other => panic!("unknown crash case {other}"),
        }
    }
    // Reached only if the call did NOT fault.
    std::process::exit(NO_FAULT);
}

// ---------------------------------------------------------------------------
// E1..E7 — shift_array_data guard FALSE  (`shift_by > 0 && shift_by < size`)
// ---------------------------------------------------------------------------

/// Calls `shift_array_data` on two identical copies and asserts (a) both
/// libraries produced the same image and (b) the image is *unchanged*.
fn shift_noop_both(l: &Libs, buf: &[c_int], size: c_int, shift_by: c_int) {
    let mut cbuf = buf.to_vec();
    let mut rbuf = buf.to_vec();
    unsafe {
        (l.c.shift_array_data)(cbuf.as_mut_ptr(), size, shift_by);
        (l.r.shift_array_data)(rbuf.as_mut_ptr(), size, shift_by);
    }
    let ctx = format!("shift_array_data(size={size}, shift_by={shift_by})");
    assert_bytes_eq(bytes_of(&cbuf), bytes_of(&rbuf), &ctx);
    assert_bytes_eq(bytes_of(buf), bytes_of(&cbuf), &format!("{ctx} must be a no-op"));
}

#[test]
fn err_e1_shift_array_shift_zero() {
    let l = libs();
    let mut rng = Rng::new(0xE1);
    for _ in 0..500 {
        let len = rng.range(0, 32) as usize;
        let buf = rand_int_buf(&mut rng, len, 4);
        shift_noop_both(&l, &buf, len as c_int, 0);
    }
}

#[test]
fn err_e2_shift_array_shift_negative() {
    let l = libs();
    let mut rng = Rng::new(0xE2);
    for &s in &[-1, -2, -7, -1000, c_int::MIN, c_int::MIN / 2] {
        for _ in 0..100 {
            let len = rng.range(0, 32) as usize;
            let buf = rand_int_buf(&mut rng, len, 4);
            shift_noop_both(&l, &buf, len as c_int, s);
        }
    }
    for _ in 0..500 {
        let len = rng.range(0, 32) as usize;
        let buf = rand_int_buf(&mut rng, len, 4);
        let s = rng.range_i32(c_int::MIN, -1);
        shift_noop_both(&l, &buf, len as c_int, s);
    }
}

#[test]
fn err_e3_shift_array_shift_eq_size() {
    let l = libs();
    let mut rng = Rng::new(0xE3);
    for _ in 0..500 {
        let len = rng.range(1, 32) as usize;
        let buf = rand_int_buf(&mut rng, len, 4);
        shift_noop_both(&l, &buf, len as c_int, len as c_int);
    }
}

#[test]
fn err_e4_shift_array_shift_gt_size() {
    let l = libs();
    let mut rng = Rng::new(0xE4);
    for _ in 0..500 {
        let len = rng.range(1, 32) as usize;
        let buf = rand_int_buf(&mut rng, len, 4);
        let over = rng.range_i32(1, 1000);
        shift_noop_both(&l, &buf, len as c_int, len as c_int + over);
    }
    for &len in &[1usize, 2, 10] {
        let buf = rand_int_buf(&mut rng, len, 4);
        shift_noop_both(&l, &buf, len as c_int, c_int::MAX);
    }
}

#[test]
fn err_e5_shift_array_size_nonpositive() {
    let l = libs();
    let mut rng = Rng::new(0xE5);
    for &size in &[0, -1, -2, -1000, c_int::MIN, c_int::MIN / 2] {
        for &shift_by in &[c_int::MIN, -1, 0, 1, 2, 1000, c_int::MAX] {
            let buf = rand_int_buf(&mut rng, 8, 4);
            shift_noop_both(&l, &buf, size, shift_by);
        }
    }
}

#[test]
fn err_e6_shift_array_size_one() {
    let l = libs();
    let mut rng = Rng::new(0xE6);
    for _ in 0..500 {
        let buf = rand_int_buf(&mut rng, 1, 4);
        // shift_by == 1 == size: one step past the largest accepted shift.
        shift_noop_both(&l, &buf, 1, 1);
        shift_noop_both(&l, &buf, 1, 0);
        shift_noop_both(&l, &buf, 1, 2);
        shift_noop_both(&l, &buf, 1, -1);
    }
}

#[test]
fn err_e7_shift_array_null_ptr_guard_false() {
    let l = libs();
    // Guard false => the pointer is never dereferenced, so NULL is harmless.
    // Both libraries must return normally (this test would crash otherwise).
    for &(size, shift_by) in &[
        (0, 0),
        (0, 1),
        (0, -1),
        (10, 0),
        (10, 10),
        (10, 11),
        (-5, 3),
        (c_int::MIN, c_int::MAX),
        (1, 1),
    ] {
        unsafe {
            (l.c.shift_array_data)(std::ptr::null_mut(), size, shift_by);
            (l.r.shift_array_data)(std::ptr::null_mut(), size, shift_by);
        }
    }
}

#[test]
fn err_e8_shift_array_null_ptr_guard_true() {
    // Guard TRUE with arr == NULL: memmove(NULL, NULL+12, 28) => SIGSEGV.
    assert_same_fault("e8_shift_null", libc_sigsegv());
}

// ---------------------------------------------------------------------------
// E9..E11 — process_pointer_data
// ---------------------------------------------------------------------------

#[test]
fn err_e9_process_pointer_null() {
    assert_same_fault("e9_ppd_null", libc_sigsegv());
}

#[test]
fn err_e10_process_pointer_zero_multiplier() {
    let l = libs();
    let mut rng = Rng::new(0xE10);
    for &(gc, ga) in &[(0, 0), (5, 7), (c_int::MAX, c_int::MIN), (-1, -1)] {
        l.set_state(gc, ga);
        for _ in 0..300 {
            let v = rng.interesting();
            let (mut cv, mut rv) = (v, v);
            let (c, r) = unsafe {
                (
                    (l.c.process_pointer_data)(&mut cv, 0),
                    (l.r.process_pointer_data)(&mut rv, 0),
                )
            };
            assert_eq!(c, r, "process_pointer_data(*{v}, 0) state=({gc},{ga})");
            assert_eq!(c, ga, "multiplier 0 must collapse to global_accumulator");
        }
    }
    l.reset();
}

#[test]
fn err_e11_process_pointer_overflow() {
    let l = libs();
    let mut rng = Rng::new(0xE11);
    for &(gc, ga) in &[(0, 0), (0, c_int::MAX), (0, c_int::MIN)] {
        l.set_state(gc, ga);
        // Deliberate signed-overflow operands.
        for &v in &[c_int::MAX, c_int::MIN, c_int::MAX - 1, 0x4000_0000, -0x4000_0000] {
            for &m in &[c_int::MAX, c_int::MIN, -1, 2, 3, 0x10000] {
                let (mut cv, mut rv) = (v, v);
                let (c, r) = unsafe {
                    (
                        (l.c.process_pointer_data)(&mut cv, m),
                        (l.r.process_pointer_data)(&mut rv, m),
                    )
                };
                assert_eq!(c, r, "process_pointer_data overflow(*{v},{m}) state=({gc},{ga})");
            }
        }
        for _ in 0..1000 {
            let v = rng.i32_full();
            let m = rng.i32_full();
            let (mut cv, mut rv) = (v, v);
            let (c, r) = unsafe {
                (
                    (l.c.process_pointer_data)(&mut cv, m),
                    (l.r.process_pointer_data)(&mut rv, m),
                )
            };
            assert_eq!(c, r, "process_pointer_data overflow(*{v},{m})");
        }
    }
    l.reset();
}

// ---------------------------------------------------------------------------
// E12..E15 — compute_with_dynamic_memory
// ---------------------------------------------------------------------------

fn cwdm_both_eq(l: &Libs, base: c_int, count: c_int) -> c_int {
    let (c, r) = unsafe {
        (
            (l.c.compute_with_dynamic_memory)(base, count),
            (l.r.compute_with_dynamic_memory)(base, count),
        )
    };
    assert_eq!(c, r, "compute_with_dynamic_memory({base},{count})");
    c
}

#[test]
fn err_e12_cwdm_count_zero() {
    let l = libs();
    let mut rng = Rng::new(0xE12);
    for &b in EXTREMES.iter() {
        assert_eq!(cwdm_both_eq(&l, b, 0), 0, "count==0 must give 0");
    }
    for _ in 0..500 {
        assert_eq!(cwdm_both_eq(&l, rng.i32_full(), 0), 0);
    }
}

#[test]
fn err_e13_cwdm_count_negative() {
    let l = libs();
    let mut rng = Rng::new(0xE13);
    // (size_t)count is sign-extended, so malloc is asked for ~2^64 bytes and
    // returns NULL; the loops do not run and free(NULL) is a no-op.
    for &count in &[-1, -2, -3, -8, -1000, -0x4000_0000, c_int::MIN + 1] {
        for &b in &[0, 1, -1, c_int::MAX, c_int::MIN] {
            assert_eq!(cwdm_both_eq(&l, b, count), 0, "count<0 must give 0");
        }
    }
    for _ in 0..500 {
        let count = rng.range_i32(c_int::MIN + 1, -1);
        assert_eq!(cwdm_both_eq(&l, rng.i32_full(), count), 0);
    }
}

#[test]
fn err_e14_cwdm_count_int_min() {
    let l = libs();
    for &b in EXTREMES.iter() {
        assert_eq!(cwdm_both_eq(&l, b, c_int::MIN), 0, "count==INT_MIN must give 0");
    }
}

#[test]
fn err_e15_cwdm_sum_overflow() {
    let l = libs();
    let mut rng = Rng::new(0xE15);
    for &base in &[c_int::MAX, c_int::MAX - 1, c_int::MIN, c_int::MIN + 1, 0x4000_0000] {
        for count in 1..=64 {
            cwdm_both_eq(&l, base, count);
        }
    }
    for _ in 0..1000 {
        let base = rng.i32_full();
        let count = rng.range_i32(1, 4096);
        cwdm_both_eq(&l, base, count);
    }
}

// ---------------------------------------------------------------------------
// E16 / E17 — get_time_based_value
// ---------------------------------------------------------------------------

#[test]
fn err_e16_time_seed_overflow() {
    let l = libs();
    let mut rng = Rng::new(0xE16);
    for &s in &[
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        c_int::MAX / 2,
        c_int::MIN / 2,
        596_524,
        -596_524,
        1_000_000,
        -1_000_000,
        0x0080_0000,
    ] {
        let (c, r) = unsafe {
            (
                (l.c.get_time_based_value)(s),
                (l.r.get_time_based_value)(s),
            )
        };
        assert_eq!(c, r, "get_time_based_value overflow({s})");
    }
    for _ in 0..3000 {
        let s = rng.i32_full();
        let (c, r) = unsafe {
            (
                (l.c.get_time_based_value)(s),
                (l.r.get_time_based_value)(s),
            )
        };
        assert_eq!(c, r, "get_time_based_value overflow({s})");
    }
}

#[test]
fn err_e17_time_seed_zero() {
    let l = libs();
    let (c, r) = unsafe { ((l.c.get_time_based_value)(0), (l.r.get_time_based_value)(0)) };
    assert_eq!(c, r);
    assert_eq!(c, 0, "seed 0 => diff 0 => 0");
}

// ---------------------------------------------------------------------------
// E18..E26 — manipulate_records guard FALSE / degenerate shapes
// ---------------------------------------------------------------------------

/// Oversized, fully initialised backing buffer so that the C code's
/// out-of-range reads (`shift < 0`) are deterministic and thus comparable.
const REC_CAP: usize = 256;

fn records_both_eq(l: &Libs, buf: &[DataRecord], n: c_int, shift: c_int) -> c_int {
    let mut cbuf = buf.to_vec();
    let mut rbuf = buf.to_vec();
    let (c, r) = unsafe {
        (
            (l.c.manipulate_records)(cbuf.as_mut_ptr(), n, shift),
            (l.r.manipulate_records)(rbuf.as_mut_ptr(), n, shift),
        )
    };
    let ctx = format!("manipulate_records(n={n}, shift={shift})");
    assert_eq!(c, r, "{ctx}: return value");
    assert_bytes_eq(bytes_of(&cbuf), bytes_of(&rbuf), &ctx);
    c
}

fn records_noop_image(l: &Libs, buf: &[DataRecord], n: c_int, shift: c_int) -> c_int {
    let mut cbuf = buf.to_vec();
    let mut rbuf = buf.to_vec();
    let (c, r) = unsafe {
        (
            (l.c.manipulate_records)(cbuf.as_mut_ptr(), n, shift),
            (l.r.manipulate_records)(rbuf.as_mut_ptr(), n, shift),
        )
    };
    let ctx = format!("manipulate_records(n={n}, shift={shift})");
    assert_eq!(c, r, "{ctx}: return value");
    assert_bytes_eq(bytes_of(&cbuf), bytes_of(&rbuf), &ctx);
    // guard false => no memmove => buffer untouched
    assert_bytes_eq(bytes_of(buf), bytes_of(&cbuf), &format!("{ctx} must not memmove"));
    c
}

#[test]
fn err_e18_records_shift_zero() {
    let l = libs();
    let mut rng = Rng::new(0xE18);
    for _ in 0..500 {
        let n = rng.range(1, 64) as c_int;
        let buf = rand_record_buf(&mut rng, REC_CAP);
        let got = records_noop_image(&l, &buf, n, 0);
        let want = buf[..n as usize]
            .iter()
            .fold(0i32, |acc, r| acc.wrapping_add(r.value));
        assert_eq!(got, want, "shift==0 must sum all n values");
    }
}

#[test]
fn err_e19_records_shift_negative() {
    let l = libs();
    let mut rng = Rng::new(0xE19);
    // n - shift <= 64 + 64 = 128 <= REC_CAP, so the over-read stays inside the
    // fully-initialised buffer and is deterministic for BOTH libraries.
    for &shift in &[-1, -2, -5, -17, -64] {
        for _ in 0..100 {
            let n = rng.range(0, 64) as c_int;
            let buf = rand_record_buf(&mut rng, REC_CAP);
            let got = records_noop_image(&l, &buf, n, shift);
            let upto = (n - shift) as usize;
            let want = buf[..upto].iter().fold(0i32, |acc, r| acc.wrapping_add(r.value));
            assert_eq!(got, want, "shift<0 reads n-shift elements");
        }
    }
}

#[test]
fn err_e20_records_shift_eq_num() {
    let l = libs();
    let mut rng = Rng::new(0xE20);
    for _ in 0..500 {
        let n = rng.range(0, 64) as c_int;
        let buf = rand_record_buf(&mut rng, REC_CAP);
        assert_eq!(records_noop_image(&l, &buf, n, n), 0, "shift==n => 0");
    }
}

#[test]
fn err_e21_records_shift_gt_num() {
    let l = libs();
    let mut rng = Rng::new(0xE21);
    for _ in 0..500 {
        let n = rng.range(0, 64) as c_int;
        let over = rng.range_i32(1, 1000);
        let buf = rand_record_buf(&mut rng, REC_CAP);
        assert_eq!(records_noop_image(&l, &buf, n, n + over), 0, "shift>n => 0");
    }
    let buf = rand_record_buf(&mut rng, REC_CAP);
    assert_eq!(records_noop_image(&l, &buf, 5, c_int::MAX), 0);
}

/// The exact loop bound the C code computes: `num_records - shift` in wrapping
/// `int` arithmetic. The read loop performs `max(0, bound)` dereferences, so a
/// pair is only safe to run in-process while `bound <= REC_CAP`.
fn wrapped_bound(n: c_int, shift: c_int) -> i32 {
    n.wrapping_sub(shift)
}

#[test]
fn err_e22_records_num_nonpositive() {
    let l = libs();
    let mut rng = Rng::new(0xE22);
    for &n in &[0, -1, -2, -1000, c_int::MIN, c_int::MIN / 2] {
        for &shift in &[0, 1, 2, 1000, c_int::MAX] {
            let bound = wrapped_bound(n, shift);
            if bound > REC_CAP as i32 {
                // Wraps into a huge positive bound => covered by the faulting
                // subprocess case below, not runnable in-process.
                continue;
            }
            let buf = rand_record_buf(&mut rng, REC_CAP);
            let got = records_noop_image(&l, &buf, n, shift);
            let want = if bound <= 0 {
                0
            } else {
                buf[..bound as usize].iter().fold(0i32, |a, r| a.wrapping_add(r.value))
            };
            assert_eq!(got, want, "manipulate_records({n},{shift}) bound={bound}");
        }
    }
    // n == INT_MIN with shift in {1, 2, 1000} wraps the bound to ~INT_MAX.
    assert_same_fault("e22b_records_num_int_min_shift1", libc_sigsegv());
}

#[test]
fn err_e23_records_num_minus_shift_overflow() {
    let l = libs();
    let mut rng = Rng::new(0xE23);
    // `num_records - shift` overflows int. At -O0 gcc wraps; depending on the
    // pair the wrapped bound is either negative (zero iterations) or a small
    // positive number (a few in-range reads).
    let buf = rand_record_buf(&mut rng, REC_CAP);
    for &(n, shift) in &[
        (0, c_int::MIN),         // bound INT_MIN  -> 0 iterations
        (1, c_int::MIN),         // bound INT_MIN+1 -> 0
        (5, c_int::MIN),         // bound INT_MIN+5 -> 0
        (c_int::MAX, c_int::MIN),// bound -1        -> 0
        (c_int::MIN, c_int::MAX),// bound 1         -> reads records[0]
        (c_int::MIN + 1, c_int::MAX), // bound 2    -> reads records[0..2]
    ] {
        let bound = wrapped_bound(n, shift);
        assert!(bound <= REC_CAP as i32, "test pair ({n},{shift}) is unsafe in-process");
        let got = records_noop_image(&l, &buf, n, shift);
        let want = if bound <= 0 {
            0
        } else {
            buf[..bound as usize].iter().fold(0i32, |a, r| a.wrapping_add(r.value))
        };
        assert_eq!(got, want, "manipulate_records({n},{shift}) bound={bound}");
    }
    // (-1, INT_MIN) wraps the bound to exactly INT_MAX: both must fault.
    assert_same_fault("e23b_records_bound_int_max", libc_sigsegv());
}

#[test]
fn err_e24_records_null_ptr_guard_false() {
    let l = libs();
    // Guard false AND loop bound <= 0 => the pointer is never dereferenced.
    for &(n, shift) in &[
        (0, 0),
        (0, 1),
        (-1, 0),
        (-5, 2),
        (5, 5),
        (5, 6),
        (5, c_int::MAX),
        (0, c_int::MIN),
        (c_int::MIN, 0),
        (c_int::MAX, c_int::MIN),
    ] {
        assert!(
            wrapped_bound(n, shift) <= 0,
            "({n},{shift}) would dereference NULL"
        );
        let c = unsafe { (l.c.manipulate_records)(std::ptr::null_mut(), n, shift) };
        let r = unsafe { (l.r.manipulate_records)(std::ptr::null_mut(), n, shift) };
        assert_eq!(c, r, "manipulate_records(NULL,{n},{shift})");
        assert_eq!(c, 0, "manipulate_records(NULL,{n},{shift}) must be 0");
    }
}

#[test]
fn err_e25_records_null_ptr_deref() {
    assert_same_fault("e25_records_null", libc_sigsegv());
    // ... and the same with shift == 0 (no memmove, but the loop still reads).
    assert_same_fault("e25b_records_null_shift0", libc_sigsegv());
}

#[test]
fn err_e26_records_boundary_one() {
    let l = libs();
    let mut rng = Rng::new(0xE26);
    for _ in 0..500 {
        let buf = rand_record_buf(&mut rng, REC_CAP);
        // n == 1: shift == 1 is one step past the largest accepted shift.
        assert_eq!(records_noop_image(&l, &buf, 1, 1), 0);
        assert_eq!(records_noop_image(&l, &buf, 1, 2), 0);
        assert_eq!(records_both_eq(&l, &buf, 2, 1), buf[1].value);
    }
}

// ---------------------------------------------------------------------------
// E27..E29 — apply_operation, the only "value with no valid variant" channel
// ---------------------------------------------------------------------------

#[test]
fn err_e27_apply_operation_null_fnptr() {
    assert_same_fault("e27_apply_null", libc_sigsegv());
}

#[test]
fn err_e28_apply_operation_bogus_fnptr() {
    assert_same_fault("e28_apply_bogus", libc_sigsegv());
}

#[test]
fn err_e29_apply_operation_cross_library() {
    let l = libs();
    let mut rng = Rng::new(0xE29);
    // Any code address is a "valid variant" for this API: passing the OTHER
    // library's callee must work and produce the identical result.
    for sym in [&b"add_three\0"[..], &b"multiply_add\0"[..]] {
        let cfp = l.c.addr(sym);
        let rfp = l.r.addr(sym);
        for _ in 0..1000 {
            let (a, b, c3) = (rng.interesting(), rng.interesting(), rng.interesting());
            let cc = unsafe { (l.c.apply_operation)(cfp, a, b, c3) };
            let cr = unsafe { (l.c.apply_operation)(rfp, a, b, c3) };
            let rc = unsafe { (l.r.apply_operation)(cfp, a, b, c3) };
            let rr = unsafe { (l.r.apply_operation)(rfp, a, b, c3) };
            assert_eq!((cc, cr, rc), (cc, cc, cc), "cross-library dispatch");
            assert_eq!(rr, cc, "cross-library dispatch");
        }
    }
}

// ---------------------------------------------------------------------------
// E30..E33 — signed-overflow (UB) paths
// ---------------------------------------------------------------------------

#[test]
fn err_e30_arith_overflow_extremes() {
    let l = libs();
    let ext = [
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN / 2,
        -1,
        0,
        1,
        c_int::MAX / 2,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    l.set_state(c_int::MAX, 0);
    for &a in ext.iter() {
        for &b in ext.iter() {
            for &c3 in ext.iter() {
                unsafe {
                    assert_eq!(
                        (l.c.add_three)(a, b, c3),
                        (l.r.add_three)(a, b, c3),
                        "add_three overflow({a},{b},{c3})"
                    );
                    assert_eq!(
                        (l.c.multiply_add)(a, b, c3),
                        (l.r.multiply_add)(a, b, c3),
                        "multiply_add overflow({a},{b},{c3})"
                    );
                    assert_eq!(
                        (l.c.complex_calc)(a, b, c3),
                        (l.r.complex_calc)(a, b, c3),
                        "complex_calc overflow({a},{b},{c3})"
                    );
                }
            }
        }
    }
    l.reset();
}

#[test]
fn err_e31_increment_counter_overflow() {
    let l = libs();
    l.set_state(c_int::MAX, 0);
    for &v in &[1, 2, c_int::MAX, c_int::MIN, -1, 1000] {
        unsafe {
            (l.c.increment_counter)(v, 999);
            (l.r.increment_counter)(v, 999);
        }
        assert_eq!(
            l.c.read_counter(),
            l.r.read_counter(),
            "global_counter wrap after += {v}"
        );
    }
    l.set_state(c_int::MIN, 0);
    for &v in &[-1, -2, c_int::MIN, c_int::MAX] {
        unsafe {
            (l.c.increment_counter)(v, 0);
            (l.r.increment_counter)(v, 0);
        }
        assert_eq!(l.c.read_counter(), l.r.read_counter());
    }
    l.reset();
}

#[test]
fn err_e32_update_accumulator_overflow() {
    let l = libs();
    // `acc*2` alone overflows once |acc| > INT_MAX/2.
    for &start in &[c_int::MAX, c_int::MIN, c_int::MAX / 2 + 1, c_int::MIN / 2 - 1] {
        l.set_state(0, start);
        for &v in &[0, 1, -1, c_int::MAX, c_int::MIN] {
            unsafe {
                (l.c.update_accumulator)(v, 888);
                (l.r.update_accumulator)(v, 888);
            }
            assert_eq!(
                l.c.read_accumulator(),
                l.r.read_accumulator(),
                "global_accumulator wrap from {start} with value {v}"
            );
        }
    }
    // Repeated doubling drives it through many wraps.
    l.set_state(0, 1);
    for i in 0..200 {
        unsafe {
            (l.c.update_accumulator)(i, 0);
            (l.r.update_accumulator)(i, 0);
        }
        assert_eq!(l.c.read_accumulator(), l.r.read_accumulator(), "doubling step {i}");
    }
    l.reset();
}

#[test]
fn err_e33_hatch_extremes() {
    let l = libs();
    let ext = [0, 1, -1, 2, -2, c_int::MAX, c_int::MIN, c_int::MAX / 2, c_int::MIN / 2];
    for &a in ext.iter() {
        for &b in ext.iter() {
            for &c in ext.iter() {
                for &d in ext.iter() {
                    l.set_state(0, 0);
                    let (cr, rr) = unsafe { ((l.c.hatch)(a, b, c, d), (l.r.hatch)(a, b, c, d)) };
                    assert_eq!(cr, rr, "hatch({a},{b},{c},{d})");
                    assert_eq!(l.c.read_counter(), l.r.read_counter(), "hatch({a},{b},{c},{d})");
                    assert_eq!(
                        l.c.read_accumulator(),
                        l.r.read_accumulator(),
                        "hatch({a},{b},{c},{d})"
                    );
                }
            }
        }
    }
    // ...and from a pre-dirtied state.
    l.set_state(c_int::MAX, c_int::MIN);
    for &a in ext.iter() {
        let (cr, rr) = unsafe {
            (
                (l.c.hatch)(a, c_int::MAX, c_int::MIN, a),
                (l.r.hatch)(a, c_int::MAX, c_int::MIN, a),
            )
        };
        assert_eq!(cr, rr, "dirty-state hatch({a},MAX,MIN,{a})");
    }
    l.reset();
}

/// E34 is a documented negative result: `lib.c` has no error protocol, so there
/// is no error code to compare. Asserted here so the row is not silently
/// forgotten: every entry point returns for *all* in-range inputs.
#[test]
fn err_e34_no_error_protocol_exists() {
    let l = libs();
    l.set_state(0, 0);
    // No function can signal failure; e.g. a failed malloc still returns 0
    // rather than an error code (see E13).
    assert_eq!(cwdm_both_eq(&l, 1234, -1), 0);
    l.reset();
}

// ---------------------------------------------------------------------------
// E35..E37 — oversized / large length boundaries
// ---------------------------------------------------------------------------

#[test]
fn err_e35_shift_array_oversized_size() {
    // Guard TRUE with size == INT_MAX: memmove copies ~8 GiB out of a small
    // buffer. Both libraries must fault the same way.
    assert_same_fault("e35_shift_oversized_size", libc_sigsegv());
}

#[test]
fn err_e36_records_oversized_num() {
    // Guard TRUE with num_records == INT_MAX: memmove copies ~96 GiB.
    assert_same_fault("e36_records_oversized_num", libc_sigsegv());
}

#[test]
fn err_e37_cwdm_large_but_valid_count() {
    let l = libs();
    // Large-but-reliable allocations (up to 4 MiB): the malloc path that
    // actually succeeds, with a `sum` that wraps many times over.
    for &count in &[1 << 10, 1 << 14, 1 << 16, 1 << 18, 1 << 20] {
        for &base in &[0, 1, -1, c_int::MAX, c_int::MIN, 12345] {
            cwdm_both_eq(&l, base, count);
        }
    }
    // NOTE: `count` near INT_MAX is deliberately NOT tested — whether
    // `malloc(count*4)` succeeds depends on the machine's memory state at that
    // instant, so the outcome is allocator-dependent rather than a property of
    // the translation. Both libraries call the *same* libc malloc (verified in
    // SYMBOLS.md), so they take whichever branch the allocator dictates.
}

// ---------------------------------------------------------------------------
// E39 / E40 — MISALIGNED pointers. The C declares `int *` / `DataRecord *` and
// never checks alignment, so a misaligned pointer is UB in C too, but on x86-64
// it simply produces an unaligned load/store. The Rust must do the same rather
// than trapping or producing different bytes.
// ---------------------------------------------------------------------------

#[test]
fn err_e39_misaligned_int_pointer() {
    let l = libs();
    let mut rng = Rng::new(0xE39);
    l.set_state(31337, -4242);
    for off in 1..8usize {
        for _ in 0..200 {
            // A byte buffer gives us a deliberately misaligned `int *`.
            let n = 16usize;
            let mut craw = vec![0u8; n * 4 + 8];
            rng.bytes(&mut craw);
            let rraw = craw.clone();
            let mut rraw = rraw;

            let m = rng.interesting();
            let (c, r) = unsafe {
                (
                    (l.c.process_pointer_data)(craw.as_mut_ptr().add(off) as *mut c_int, m),
                    (l.r.process_pointer_data)(rraw.as_mut_ptr().add(off) as *mut c_int, m),
                )
            };
            assert_eq!(c, r, "process_pointer_data misaligned(off={off}, mult={m})");
            assert_bytes_eq(&craw, &rraw, "misaligned read must not write");

            // ...and a misaligned array for shift_array_data (memmove/memset
            // are byte-wise, so this must work identically).
            let size = 12i32;
            let shift_by = rng.range_i32(1, size - 1);
            unsafe {
                (l.c.shift_array_data)(craw.as_mut_ptr().add(off) as *mut c_int, size, shift_by);
                (l.r.shift_array_data)(rraw.as_mut_ptr().add(off) as *mut c_int, size, shift_by);
            }
            assert_bytes_eq(
                &craw,
                &rraw,
                &format!("shift_array_data misaligned(off={off}, size={size}, shift={shift_by})"),
            );
        }
    }
    l.reset();
}

#[test]
fn err_e40_misaligned_record_pointer() {
    let l = libs();
    let mut rng = Rng::new(0xE40);
    for off in 1..8usize {
        for _ in 0..200 {
            let n = 6i32;
            let mut craw = vec![0u8; n as usize * 48 + 8];
            rng.bytes(&mut craw);
            let mut rraw = craw.clone();
            let shift = rng.range_i32(1, n - 1);
            let (c, r) = unsafe {
                (
                    (l.c.manipulate_records)(
                        craw.as_mut_ptr().add(off) as *mut DataRecord,
                        n,
                        shift,
                    ),
                    (l.r.manipulate_records)(
                        rraw.as_mut_ptr().add(off) as *mut DataRecord,
                        n,
                        shift,
                    ),
                )
            };
            assert_eq!(c, r, "manipulate_records misaligned(off={off}, shift={shift})");
            assert_bytes_eq(
                &craw,
                &rraw,
                &format!("manipulate_records misaligned image(off={off}, shift={shift})"),
            );
        }
    }
}

fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}
