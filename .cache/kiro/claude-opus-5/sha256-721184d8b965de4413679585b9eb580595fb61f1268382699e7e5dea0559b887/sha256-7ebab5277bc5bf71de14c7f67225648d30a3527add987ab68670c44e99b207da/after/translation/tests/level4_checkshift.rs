//! Level 4: the public entry point `checkshift`.
//!
//! Compares the return value and the complete stdout transcript (~15 printf
//! calls per invocation) of the C and Rust shared libraries.

mod common;

use common::*;
use std::ffi::c_int;

fn check(p: [c_int; 4]) {
    let libs = impls();
    let c: libloading::Symbol<FnCheckshift> = libs.sym(Which::C, "checkshift");
    let r: libloading::Symbol<FnCheckshift> = libs.sym(Which::Rust, "checkshift");

    let (cv, cout) = capture_stdout(|| unsafe { c(p[0], p[1], p[2], p[3]) });
    let (rv, rout) = capture_stdout(|| unsafe { r(p[0], p[1], p[2], p[3]) });

    assert_eq!(
        cv, rv,
        "checkshift({}, {}, {}, {}): C returned {cv}, Rust returned {rv}",
        p[0], p[1], p[2], p[3]
    );
    if cout != rout {
        panic!(
            "checkshift({}, {}, {}, {}) stdout differs:\nC   =<<<{}>>>\nRust=<<<{}>>>",
            p[0],
            p[1],
            p[2],
            p[3],
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
    }
    assert!(!cout.is_empty(), "checkshift produced no output at all");
}

fn transcript_shape() {
    // Sanity-check that the capture really is picking up the C library's printf
    // output, so the byte comparisons below are meaningful.
    let libs = impls();
    let c: libloading::Symbol<FnCheckshift> = libs.sym(Which::C, "checkshift");
    let (_, out) = capture_stdout(|| unsafe { c(1, 2, 3, 4) });
    let text = String::from_utf8_lossy(&out).to_string();
    for needle in [
        "=== Starting foo function ===",
        "Parameters: 1, 2, 3, 4",
        "State initialized with accumulator = 1",
        "--- Operation 1: Multiply ---",
        "--- Operation 2: Add ---",
        "--- Operation 3: XOR ---",
        "--- Operation 4: Shift ---",
        "Variable a = ",
        "Variable b = ",
        "Result of XOR: ",
        "Result of SHIFT: ",
        "Computed checksum: 0x",
        "Final accumulator: ",
        "Operation count: 2",
        "Final result: ",
        "=== Ending foo function ===",
    ] {
        assert!(
            text.contains(needle),
            "C transcript missing {needle:?}; got:\n{text}"
        );
    }
}

fn small_exhaustive() {
    let vals: [c_int; 7] = [-3, -1, 0, 1, 2, 3, 4];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    check([a, b, c, d]);
                }
            }
        }
    }
}

fn documented_example() {
    check([1, 2, 3, 4]);
    check([0, 0, 0, 0]);
}

fn sample_grid() {
    let vals = sample_ints();
    // Vary one parameter at a time over the full sample set, keeping the others
    // at values that exercise sign changes and overflow.
    let bases: [[c_int; 4]; 5] = [
        [1, 2, 3, 4],
        [-1, -2, -3, -4],
        [c_int::MAX, c_int::MIN, c_int::MAX, c_int::MIN],
        [0, 0, 0, 0],
        [0x5555_5555, -0x5555_5555, 0x0F0F_0F0F, -1],
    ];
    for base in bases {
        for slot in 0..4 {
            for &v in &vals {
                let mut p = base;
                p[slot] = v;
                check(p);
            }
        }
    }
}

fn extremes() {
    let vals: [c_int; 10] = [
        c_int::MIN,
        c_int::MIN + 1,
        -0x4000_0000,
        -1,
        0,
        1,
        0x3FFF_FFFF,
        0x4000_0000,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                check([a, b, c, vals[(a as usize) % vals.len()]]);
            }
        }
    }
}

fn randomized() {
    let mut rng = Rng::new(0xA5A5_5A5A_1357_9BDF);
    for _ in 0..4000 {
        check([
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ]);
    }
}

/// Values whose low 16 bits drive the checksum, and shift amounts that push bits
/// off both ends of the final `(accumulator + shift_result) ^ checksum`.
fn checksum_sensitive() {
    let mut rng = Rng::new(0x0102_0304_0506_0708);
    for _ in 0..1500 {
        let lo = (rng.next_u64() & 0xFFFF) as c_int;
        let hi = ((rng.next_u64() & 0xFFFF) as c_int) << 16;
        check([lo, hi, lo | hi, !(lo | hi)]);
        check([hi, lo, 0, -1]);
    }
}

/// Repeated invocations must be stateless: `checkshift` allocates, initialises
/// and frees its own state every call.
fn repeated_calls_are_stateless() {
    let libs = impls();
    let c: libloading::Symbol<FnCheckshift> = libs.sym(Which::C, "checkshift");
    let r: libloading::Symbol<FnCheckshift> = libs.sym(Which::Rust, "checkshift");

    let (cv1, cout1) = capture_stdout(|| unsafe { c(5, 6, 7, 8) });
    let (rv1, rout1) = capture_stdout(|| unsafe { r(5, 6, 7, 8) });
    for _ in 0..50 {
        let (cv, cout) = capture_stdout(|| unsafe { c(5, 6, 7, 8) });
        let (rv, rout) = capture_stdout(|| unsafe { r(5, 6, 7, 8) });
        assert_eq!((cv, &cout), (cv1, &cout1), "C checkshift not stateless");
        assert_eq!((rv, &rout), (rv1, &rout1), "Rust checkshift not stateless");
        assert_eq!(cv, rv);
        assert_eq!(cout, rout);
    }
}

fn main() {
    let mut r = Runner::new();
    r.case("transcript_shape", transcript_shape);
    r.case("documented_example", documented_example);
    r.case("small_exhaustive", small_exhaustive);
    r.case("sample_grid", sample_grid);
    r.case("extremes", extremes);
    r.case("checksum_sensitive", checksum_sensitive);
    r.case("randomized", randomized);
    r.case("repeated_calls_are_stateless", repeated_calls_are_stateless);
    r.finish();
}
