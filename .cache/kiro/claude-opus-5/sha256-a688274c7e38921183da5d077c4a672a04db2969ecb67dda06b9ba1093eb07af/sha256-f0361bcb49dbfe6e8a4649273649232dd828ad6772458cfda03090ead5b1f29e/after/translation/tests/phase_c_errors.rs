//! Phase C — error / rejection-path differential tests.
//! One test per row of ERRORS.md, plus the generic FFI boundaries.

mod common;

use common::*;
use std::ffi::c_int;

// ==========================================================================
// Rows 1-5: fma_array degenerate / non-positive lengths, null pointers
// ==========================================================================

/// Row 1: `len == 0` with valid buffers -> not a single byte written.
#[test]
fn err01_fma_len_zero_no_writes() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..100 {
        let slots: Vec<Vec<i32>> = (0..4).map(|_| Vals::FullRandom.make(&mut rng, 8)).collect();
        // Byte-for-byte equality of *all* backing buffers, and each side must
        // have left the buffers exactly as they were.
        diff_fma(Alias::Disjoint, &slots, 0, "err01");
        let p = pair();
        let after = run_fma(&p.c, Alias::Disjoint, &slots, 0);
        assert_eq!(after, slots, "C fma_array wrote with len==0");
        let after_rs = run_fma(&p.rs, Alias::Disjoint, &slots, 0);
        assert_eq!(after_rs, slots, "Rust fma_array wrote with len==0");
    }
    // Every aliasing configuration too.
    for alias in [
        Alias::Disjoint,
        Alias::Mul1EqMul2,
        Alias::OutEqMul1,
        Alias::OutEqAdd,
        Alias::AllSame,
    ] {
        let slots: Vec<Vec<i32>> = (0..alias.slots())
            .map(|_| Vals::FullRandom.make(&mut rng, 8))
            .collect();
        diff_fma(alias, &slots, 0, "err01-alias");
    }
}

/// Row 2: `len == -1` — one step past the low end. Silently treated as empty.
#[test]
fn err02_fma_len_negative_one() {
    let mut rng = Rng::new(SEED ^ 2);
    let p = pair();
    for _ in 0..100 {
        let slots: Vec<Vec<i32>> = (0..4).map(|_| Vals::FullRandom.make(&mut rng, 8)).collect();
        diff_fma(Alias::Disjoint, &slots, -1, "err02");
        assert_eq!(
            run_fma(&p.c, Alias::Disjoint, &slots, -1),
            slots,
            "C wrote with len==-1"
        );
        assert_eq!(
            run_fma(&p.rs, Alias::Disjoint, &slots, -1),
            slots,
            "Rust wrote with len==-1"
        );
    }
}

/// Row 3: `len == INT_MIN` and a sweep of other negative lengths.
#[test]
fn err03_fma_len_int_min_and_negatives() {
    let mut rng = Rng::new(SEED ^ 3);
    for len in [
        c_int::MIN,
        c_int::MIN + 1,
        -1_000_000,
        -1024,
        -3,
        -2,
        -1,
        0,
    ] {
        let slots: Vec<Vec<i32>> = (0..4).map(|_| Vals::FullRandom.make(&mut rng, 8)).collect();
        diff_fma(Alias::Disjoint, &slots, len, "err03");
        diff_fma(Alias::AllSame, &slots[..1], len, "err03-aliased");
    }
    for _ in 0..100 {
        let len = rng.range(c_int::MIN as i64, -1) as c_int;
        let slots: Vec<Vec<i32>> = (0..4).map(|_| Vals::FullRandom.make(&mut rng, 4)).collect();
        diff_fma(Alias::Disjoint, &slots, len, "err03-random-negative");
    }
}

/// Row 4: `len == 0` with all four pointers NULL — no dereference must happen.
#[test]
fn err04_fma_null_pointers_len_zero() {
    let p = pair();
    let n: *mut c_int = std::ptr::null_mut();
    unsafe { (p.c.fma_array)(n, n, n, n, 0) };
    unsafe { (p.rs.fma_array)(n, n, n, n, 0) };
    // Reaching here at all is the assertion: neither implementation touched the
    // null pointers. Also check every partial-null combination.
    let mut buf = [1i32, 2, 3, 4];
    let b = buf.as_mut_ptr();
    for mask in 0u8..16 {
        let out = if mask & 1 != 0 { n } else { b };
        let m1 = if mask & 2 != 0 { n } else { b };
        let m2 = if mask & 4 != 0 { n } else { b };
        let ad = if mask & 8 != 0 { n } else { b };
        unsafe { (p.c.fma_array)(out, m1, m2, ad, 0) };
        unsafe { (p.rs.fma_array)(out, m1, m2, ad, 0) };
    }
    assert_eq!(buf, [1, 2, 3, 4], "buffer modified with len==0");
}

/// Row 5: negative `len` with all four pointers NULL.
#[test]
fn err05_fma_null_pointers_len_negative() {
    let p = pair();
    let n: *mut c_int = std::ptr::null_mut();
    for len in [-1, -2, -1024, c_int::MIN + 1, c_int::MIN] {
        unsafe { (p.c.fma_array)(n, n, n, n, len) };
        unsafe { (p.rs.fma_array)(n, n, n, n, len) };
    }
}

// ==========================================================================
// Rows 6-8: signed-overflow behaviour (UB in C, wrapping in the emitted code)
// ==========================================================================

/// Row 6: the multiply overflows `int`.
#[test]
fn err06_fma_multiply_overflow() {
    // Hand-picked products that definitely overflow, plus a random sweep whose
    // product is forced out of range.
    let cases: &[(i32, i32, i32)] = &[
        (i32::MAX, 2, 0),
        (i32::MAX, i32::MAX, 0),
        (i32::MIN, -1, 0),
        (i32::MIN, i32::MIN, 0),
        (65536, 65536, 0),
        (46341, 46341, 0),
        (-46341, 46341, 0),
        (1 << 30, 4, 0),
        (i32::MIN, 2, 0),
    ];
    for &(a, b, c) in cases {
        let slots = vec![vec![0i32], vec![a], vec![b], vec![c]];
        diff_fma(Alias::Disjoint, &slots, 1, "err06");
    }
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..500 {
        // Large magnitudes -> product almost always overflows.
        let a = rng.range(1 << 16, i32::MAX as i64) as i32 * if rng.next_u64() & 1 == 0 { 1 } else { -1 };
        let b = rng.range(1 << 16, i32::MAX as i64) as i32 * if rng.next_u64() & 1 == 0 { 1 } else { -1 };
        let slots = vec![vec![0i32], vec![a], vec![b], vec![0]];
        diff_fma(Alias::Disjoint, &slots, 1, "err06-random");
    }
}

/// Row 7: the multiply is in range but the add overflows.
#[test]
fn err07_fma_add_overflow() {
    let cases: &[(i32, i32, i32)] = &[
        (i32::MAX, 1, 1),
        (i32::MAX, 1, i32::MAX),
        (i32::MIN, 1, -1),
        (i32::MIN, 1, i32::MIN),
        (2, 3, i32::MAX),
        (2, 3, i32::MIN),
        (0, 0, i32::MIN),
        (1, 1, i32::MAX),
    ];
    for &(a, b, c) in cases {
        let slots = vec![vec![0i32], vec![a], vec![b], vec![c]];
        diff_fma(Alias::Disjoint, &slots, 1, "err07");
    }
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..500 {
        let a = rng.range(-100, 100) as i32;
        let b = rng.range(-100, 100) as i32;
        let c = rng.pick(&[i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1]);
        let slots = vec![vec![0i32], vec![a], vec![b], vec![c]];
        diff_fma(Alias::Disjoint, &slots, 1, "err07-random");
    }
}

/// Row 8: multiply *and* add both overflow, at the extremes.
#[test]
fn err08_fma_double_overflow() {
    for a in EXTREMES {
        for b in EXTREMES {
            for c in [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
                let slots = vec![vec![0i32], vec![a], vec![b], vec![c]];
                diff_fma(Alias::Disjoint, &slots, 1, "err08");
            }
        }
    }
}

/// Row 9: full aliasing — writes are observed by the following reads.
#[test]
fn err09_fma_full_alias_inplace_semantics() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..200 {
        let len = rng.range(1, 96) as usize;
        let data = if rng.next_u64() & 1 == 0 {
            Vals::FullRandom.make(&mut rng, len)
        } else {
            Vals::Extremes.make(&mut rng, len)
        };
        diff_fma(Alias::AllSame, &[data.clone()], len as c_int, "err09");
        // And confirm the element-wise semantics the C loop implies.
        let p = pair();
        let got = run_fma(&p.c, Alias::AllSame, &[data.clone()], len as c_int);
        let want: Vec<i32> = data
            .iter()
            .map(|&v| v.wrapping_mul(v).wrapping_add(v))
            .collect();
        assert_eq!(got[0], want, "err09 in-place semantics");
    }
}

// ==========================================================================
// Rows 10-12, 15: driver degenerate lengths and value extremes
// ==========================================================================

/// Row 10: `driver(data, 0)` with a valid pointer -> empty stdout, no crash.
#[test]
fn err10_driver_len_zero() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..50 {
        let data = Vals::FullRandom.make(&mut rng, 8);
        let out = diff_driver(&data, 0, "err10");
        assert!(out.is_empty(), "err10 expected empty stdout, got {out:?}");
    }
}

/// Row 11: `driver(NULL, 0)` -> `memcpy(dst, NULL, 0)`, empty stdout, no crash.
#[test]
fn err11_driver_null_len_zero() {
    let p = pair();
    let c_out = capture_stdout("c", || unsafe { (p.c.driver)(std::ptr::null(), 0) });
    let rs_out = capture_stdout("rs", || unsafe { (p.rs.driver)(std::ptr::null(), 0) });
    assert_eq!(c_out, rs_out, "err11 stdout differs");
    assert!(c_out.is_empty(), "err11 expected empty stdout, got {c_out:?}");
}

/// Row 12: `len == 1`, the minimum non-degenerate length.
#[test]
fn err12_driver_len_one_boundary() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..200 {
        let v = if rng.next_u64() & 1 == 0 {
            rng.next_i32()
        } else {
            rng.pick(&EXTREMES)
        };
        let out = diff_driver(&[v], 1, "err12");
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            1,
            "err12 expected exactly one line"
        );
    }
}

/// Row 15: `driver` values that overflow inside the pipeline.
#[test]
fn err15_driver_overflow_values() {
    for v in EXTREMES {
        diff_driver(&[v], 1, "err15-single");
    }
    diff_driver(&EXTREMES, EXTREMES.len() as c_int, "err15-all");
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..150 {
        let len = rng.range(1, 48) as usize;
        let data = Vals::Extremes.make(&mut rng, len);
        diff_driver(&data, len as c_int, "err15-random");
    }
}

// ==========================================================================
// Generic FFI boundaries beyond the table
// ==========================================================================

/// Out-of-range "enum" values across the FFI boundary. This API declares no
/// enum, so the closest analogue is the full `int` domain of `len`: values with
/// no meaningful interpretation must be handled identically. Only the
/// non-positive half is safe to invoke in-process (positive values index the
/// caller's buffer); the crashing half is covered by the child-process tests.
#[test]
fn err_generic_int_domain_sweep_of_len() {
    let mut rng = Rng::new(SEED ^ 0xEE);
    let p = pair();
    let mut lens: Vec<c_int> = vec![0, -1, -2, -3, -7, -255, -256, -65535, -65536, -1 << 20, c_int::MIN, c_int::MIN + 1];
    for _ in 0..200 {
        lens.push(rng.range(c_int::MIN as i64, 0) as c_int);
    }
    for len in lens {
        // fma_array
        let slots: Vec<Vec<i32>> = (0..4).map(|_| Vals::FullRandom.make(&mut rng, 4)).collect();
        diff_fma(Alias::Disjoint, &slots, len, "err-generic-fma");
        // driver: non-positive lengths that do not trigger the huge memcpy.
        if len == 0 {
            let data = Vals::FullRandom.make(&mut rng, 4);
            diff_driver(&data, len, "err-generic-driver");
        }
        // null pointers with the same out-of-domain length
        let n: *mut c_int = std::ptr::null_mut();
        unsafe { (p.c.fma_array)(n, n, n, n, len) };
        unsafe { (p.rs.fma_array)(n, n, n, n, len) };
    }
}

/// `fma_array` with `len == 1` and every pointer pointing at the same single
/// element, repeatedly — checks there is no hidden per-call state in either lib.
#[test]
fn err_generic_repeated_calls_are_stateless() {
    let p = pair();
    for v in EXTREMES {
        let mut c_buf = [v];
        let mut rs_buf = [v];
        for _ in 0..10 {
            unsafe {
                (p.c.fma_array)(c_buf.as_mut_ptr(), c_buf.as_ptr(), c_buf.as_ptr(), c_buf.as_ptr(), 1)
            };
            unsafe {
                (p.rs.fma_array)(
                    rs_buf.as_mut_ptr(),
                    rs_buf.as_ptr(),
                    rs_buf.as_ptr(),
                    rs_buf.as_ptr(),
                    1,
                )
            };
            assert_eq!(c_buf, rs_buf, "stateless check diverged at start value {v}");
        }
    }
}

// ==========================================================================
// Rows 13-14: crash-equivalence, run in child processes
// ==========================================================================
//
// `driver` with a negative `len` computes `len * sizeof(int)` as a `size_t`
// (sign-extended), so `memcpy` is asked to copy ~2^64 bytes and the process
// dies. An oversized `len` overflows the VLA / the copy source. These are real
// inputs an external caller can pass, so instead of skipping them we re-exec
// this test binary as a child and require the C and the Rust `.so` to fail
// *identically* (same terminating signal, same stdout bytes).

const CRASH_ENV: &str = "DRIVER_CRASH_CASE";

/// Crash cases: (name, len). `data` is always a small valid buffer.
const CRASH_CASES: &[(&str, i64)] = &[
    ("neg1", -1),
    ("neg2", -2),
    ("neg1024", -1024),
    ("intmin", i32::MIN as i64),
    ("intmin1", i32::MIN as i64 + 1),
    ("huge_1g", 1 << 30),
    ("huge_256m", 1 << 26),
    ("intmax", i32::MAX as i64),
];

#[derive(Debug, PartialEq, Eq)]
struct ChildOutcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
}

fn run_crash_child(which: &str, case: &str) -> ChildOutcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args([
            "--exact",
            "crash_worker",
            "--include-ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(CRASH_ENV, format!("{which}:{case}"))
        .output()
        .expect("spawn crash child");
    // Keep only what the driver call printed: the worker frames it with markers.
    let stdout = extract_between(&out.stdout, b"<<<BEGIN>>>\n", b"<<<END>>>\n");
    ChildOutcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout,
    }
}

fn extract_between(hay: &[u8], start: &[u8], end: &[u8]) -> Vec<u8> {
    let s = match find(hay, start) {
        Some(i) => i + start.len(),
        None => return Vec::new(),
    };
    let e = find(&hay[s..], end).map(|i| s + i).unwrap_or(hay.len());
    hay[s..e].to_vec()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

/// The child half of the crash tests. Ignored so it never runs during a normal
/// `cargo test`; the parent invokes it explicitly with `--include-ignored`.
#[test]
#[ignore = "child process helper, driven by the row13/row14 crash tests"]
fn crash_worker() {
    let Ok(spec) = std::env::var(CRASH_ENV) else {
        return; // invoked without a case: nothing to do
    };
    let (which, case) = spec.split_once(':').expect("bad crash spec");
    let len = CRASH_CASES
        .iter()
        .find(|(n, _)| *n == case)
        .map(|(_, l)| *l)
        .unwrap_or_else(|| panic!("unknown crash case {case}"));

    let p = pair();
    let f = match which {
        "c" => p.c.driver,
        "rs" => p.rs.driver,
        other => panic!("unknown impl {other}"),
    };

    // A small, valid source buffer: the crash comes from `len`, not from `data`.
    let data: Vec<c_int> = (0..8).collect();

    print!("<<<BEGIN>>>\n");
    use std::io::Write;
    std::io::stdout().flush().unwrap();
    unsafe { f(data.as_ptr(), len as c_int) };
    // If we survive, mark it and report the exit cleanly.
    print!("<<<END>>>\n");
    std::io::stdout().flush().unwrap();
}

/// Row 13: negative `len` — C and Rust must fail identically.
#[test]
fn err13_driver_negative_len_crash_equivalence() {
    for (name, len) in CRASH_CASES.iter().filter(|(_, l)| *l < 0) {
        let c = run_crash_child("c", name);
        let rs = run_crash_child("rs", name);
        assert_eq!(
            (c.signal, &c.stdout),
            (rs.signal, &rs.stdout),
            "row13 crash-equivalence divergence for len={len} ({name}):\n  C   ={c:?}\n  Rust={rs:?}"
        );
        assert!(
            c.signal.is_some() || c.code == Some(0),
            "row13 unexpected child outcome for {name}: {c:?}"
        );
    }
}

/// Row 14: `len` far larger than the stack / the source buffer.
#[test]
fn err14_driver_oversized_len_crash_equivalence() {
    for (name, len) in CRASH_CASES.iter().filter(|(_, l)| *l > 0) {
        let c = run_crash_child("c", name);
        let rs = run_crash_child("rs", name);
        assert_eq!(
            (c.signal, &c.stdout),
            (rs.signal, &rs.stdout),
            "row14 crash-equivalence divergence for len={len} ({name}):\n  C   ={c:?}\n  Rust={rs:?}"
        );
    }
}
