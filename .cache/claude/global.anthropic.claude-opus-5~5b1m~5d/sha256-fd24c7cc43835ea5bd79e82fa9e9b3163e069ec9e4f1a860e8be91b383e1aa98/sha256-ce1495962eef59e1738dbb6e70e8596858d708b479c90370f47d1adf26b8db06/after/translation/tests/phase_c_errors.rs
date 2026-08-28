//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1 .. E20). Every test constructs the exact
//! invalid input/condition the C rejects, calls BOTH `.so`s through
//! `libloading`, and asserts they return the SAME sentinel *and* print the same
//! rejection message.
//!
//! Rows E1/E3/E10/E12 are allocation-failure paths. They are reachable only by
//! actually exhausting the heap, which is done in a re-executed subprocess:
//! `RLIMIT_AS` is lowered, the allocator is drained until `malloc` of the
//! relevant size returns NULL, and only then is the library entry point called.
//! Row E4 is a NULL out-param that the C dereferences without a check, so it is
//! also driven in a subprocess and compared by termination signal.

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

const INT_MAX: i32 = i32::MAX;
const INT_MIN: i32 = i32::MIN;

const READ_PERM: i32 = 0o400;
const WRITE_PERM: i32 = 0o200;
const EXEC_PERM: i32 = 0o100;

// ---------------------------------------------------------------------------
// harness self-check: the stdout comparison must not be vacuous
// ---------------------------------------------------------------------------

/// If `capture_stdout` silently returned nothing, every stdout assertion in
/// Phases B and C would pass trivially. Prove it captures real library bytes.
#[test]
fn e0_harness_captures_library_stdout() {
    let (c, r) = libs();
    let (cv, cout) = capture_stdout(|| unsafe { (c.complexmode)(1, 2, 3, 4) });
    let (rv, rout) = capture_stdout(|| unsafe { (r.complexmode)(1, 2, 3, 4) });
    assert_eq!((cv, rv), (5, 5));
    assert_eq!(
        cout,
        b"Mode 1: Addition\nResult: 5\nOperation performed: addition\n".to_vec(),
        "C stdout capture is wrong: {}",
        show(&cout)
    );
    assert_eq!(cout, rout, "C {} vs Rust {}", show(&cout), show(&rout));
}

// ---------------------------------------------------------------------------
// E2 / E18 — safe_add permission rejection
// ---------------------------------------------------------------------------

#[test]
fn err_e2_safe_add_insufficient_perms() {
    let (c, r) = libs();
    let msg = b"Insufficient permissions for addition\n".to_vec();

    // Every `perms` that does not contain BOTH 0400 and 0200 must reject.
    let mut cases: Vec<i32> = vec![
        0,                       // E18: no bits at all
        READ_PERM,               // 0400 only
        WRITE_PERM,              // 0200 only
        EXEC_PERM,               // wrong bit
        READ_PERM | EXEC_PERM,   // missing WRITE
        WRITE_PERM | EXEC_PERM,  // missing READ
        0o177,                   // low bits only
        0o444,
        0o222,
        !(READ_PERM | WRITE_PERM), // everything except the two required bits
    ];
    let mut rng = Rng::new(0xE2);
    for _ in 0..2_000 {
        let p = rng.i32_any();
        if (p & (READ_PERM | WRITE_PERM)) != (READ_PERM | WRITE_PERM) {
            cases.push(p);
        }
    }

    for &perms in &cases {
        let (a, b) = (rng.i32_any(), rng.i32_any());
        let cr = capture_stdout(|| unsafe { (c.safe_add)(a, b, perms) });
        let rr = capture_stdout(|| unsafe { (r.safe_add)(a, b, perms) });
        assert_eq!(cr.0, 0, "C sentinel for safe_add({a},{b},{perms:#o})");
        assert_eq!(cr.1, msg, "C message for safe_add({a},{b},{perms:#o})");
        assert_same(
            &format!("E2 safe_add({a}, {b}, {perms:#o})"),
            cr,
            rr,
        );
    }

    // E18, accept side: perms == -1 has every bit, so it must NOT reject.
    for perms in [-1, INT_MIN | 0o600, 0o600, 0o644, 0o777] {
        let (a, b) = (rng.i32_any(), rng.i32_any());
        let cr = capture_stdout(|| unsafe { (c.safe_add)(a, b, perms) });
        let rr = capture_stdout(|| unsafe { (r.safe_add)(a, b, perms) });
        assert!(cr.1.is_empty(), "C should not reject perms={perms:#o}");
        assert_eq!(cr.0, a.wrapping_add(b));
        assert_same(&format!("E18 safe_add({a}, {b}, {perms:#o})"), cr, rr);
    }
}

// ---------------------------------------------------------------------------
// E5 — copy_and_sum NULL src
// ---------------------------------------------------------------------------

#[test]
fn err_e5_copy_and_sum_null_src() {
    let (c, r) = libs();
    let msg = b"Source pointer is NULL\n".to_vec();
    // The NULL check precedes any use of `count`, so it must win for every
    // count, including 0 and the negative values that would otherwise hit E6.
    for count in [0, 1, 3, -1, -1000, INT_MAX, INT_MIN, 7, 1 << 20] {
        let cr = capture_stdout(|| unsafe { (c.copy_and_sum)(std::ptr::null_mut(), count) });
        let rr = capture_stdout(|| unsafe { (r.copy_and_sum)(std::ptr::null_mut(), count) });
        assert_eq!(cr.0, -1, "C sentinel for copy_and_sum(NULL, {count})");
        assert_eq!(cr.1, msg, "C message for copy_and_sum(NULL, {count})");
        assert_same(&format!("E5 copy_and_sum(NULL, {count})"), cr, rr);
    }
}

// ---------------------------------------------------------------------------
// E6 / E19 — copy_and_sum allocation failure via negative count
// ---------------------------------------------------------------------------

#[test]
fn err_e6_copy_and_sum_alloc_failure() {
    let (c, r) = libs();
    let msg = b"Memory allocation failed\n".to_vec();
    let mut buf: Vec<i32> = vec![1, 2, 3, 4, 5, 6, 7, 8];

    let mut counts: Vec<i32> = vec![-1, -2, -3, -4, -5, -8, -1000, -65536, INT_MIN, INT_MIN + 1];
    let mut rng = Rng::new(0xE6);
    for _ in 0..500 {
        // Any negative int sign-extends to a size_t that cannot be allocated.
        counts.push(-(1 + (rng.below(i32::MAX as u64) as i32).abs()));
    }

    for &count in &counts {
        let p = buf.as_mut_ptr();
        let cr = capture_stdout(|| unsafe { (c.copy_and_sum)(p, count) });
        let rr = capture_stdout(|| unsafe { (r.copy_and_sum)(p, count) });
        assert_eq!(cr.0, -1, "C sentinel for copy_and_sum(buf, {count})");
        assert_eq!(
            cr.1,
            msg,
            "C message for copy_and_sum(buf, {count}): {}",
            show(&cr.1)
        );
        assert_same(&format!("E6/E19 copy_and_sum(buf, {count})"), cr, rr);
    }
}

// ---------------------------------------------------------------------------
// E7 / E8 / E9 — compare_operations NULL arguments
// ---------------------------------------------------------------------------

#[test]
fn err_e7_e8_e9_compare_operations_nulls() {
    let (c, r) = libs();
    let msg = b"One or both operation strings are NULL\n".to_vec();
    let strings: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"none".to_vec(),
        b"multiplication".to_vec(),
        vec![0xFFu8; 40],
    ];

    let null: *const c_char = std::ptr::null();
    for s in &strings {
        let cs = cstring(s);
        // E7: op1 == NULL
        let cr = capture_stdout(|| unsafe { (c.compare_operations)(null, cs.as_ptr()) });
        let rr = capture_stdout(|| unsafe { (r.compare_operations)(null, cs.as_ptr()) });
        assert_eq!(cr.0, -1);
        assert_eq!(cr.1, msg);
        assert_same(&format!("E7 compare_operations(NULL, {})", show(s)), cr, rr);

        // E8: op2 == NULL
        let cr = capture_stdout(|| unsafe { (c.compare_operations)(cs.as_ptr(), null) });
        let rr = capture_stdout(|| unsafe { (r.compare_operations)(cs.as_ptr(), null) });
        assert_eq!(cr.0, -1);
        assert_eq!(cr.1, msg);
        assert_same(&format!("E8 compare_operations({}, NULL)", show(s)), cr, rr);
    }

    // E9: both NULL -> exactly ONE message, not two.
    let cr = capture_stdout(|| unsafe { (c.compare_operations)(null, null) });
    let rr = capture_stdout(|| unsafe { (r.compare_operations)(null, null) });
    assert_eq!(cr.0, -1);
    assert_eq!(cr.1, msg, "C printed {} for both-NULL", show(&cr.1));
    assert_same("E9 compare_operations(NULL, NULL)", cr, rr);
}

// ---------------------------------------------------------------------------
// E11 / E15 — complexmode invalid mode (default arm)
// ---------------------------------------------------------------------------

#[test]
fn err_e11_e15_complexmode_invalid_mode() {
    let (c, r) = libs();
    // `Invalid mode` and NO `Operation performed:` trailer, because `operation`
    // is still "none" at the final strcmp.
    let expected = b"Invalid mode\n".to_vec();

    let mut modes: Vec<i32> = vec![0, -1, 5, 6, 7, -2, 100, INT_MIN, INT_MAX, INT_MIN + 1, INT_MAX - 1];
    let mut rng = Rng::new(0xE11);
    for _ in 0..2_000 {
        let m = rng.i32_any();
        if !(1..=4).contains(&m) {
            modes.push(m);
        }
    }

    for &mode in &modes {
        let (v1, v2, v3) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        let cr = capture_stdout(|| unsafe { (c.complexmode)(mode, v1, v2, v3) });
        let rr = capture_stdout(|| unsafe { (r.complexmode)(mode, v1, v2, v3) });
        assert_eq!(cr.0, -1, "C sentinel for complexmode({mode}, ...)");
        assert_eq!(
            cr.1,
            expected,
            "C message for complexmode({mode}, ...): {}",
            show(&cr.1)
        );
        assert_same(&format!("E11/E15 complexmode({mode}, {v1}, {v2}, {v3})"), cr, rr);
    }
}

// ---------------------------------------------------------------------------
// E13 — copy_and_sum count == 0 boundary (not rejected)
// ---------------------------------------------------------------------------

#[test]
fn err_e13_copy_and_sum_zero_count() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xE13);
    for _ in 0..500 {
        let mut buf: Vec<i32> = (0..4).map(|_| rng.i32_any()).collect();
        let p = buf.as_mut_ptr();
        let cr = capture_stdout(|| unsafe { (c.copy_and_sum)(p, 0) });
        let rr = capture_stdout(|| unsafe { (r.copy_and_sum)(p, 0) });
        assert_eq!(cr.0, 0, "C: malloc(0) is non-NULL so count 0 sums to 0");
        assert!(cr.1.is_empty(), "C printed {} for count 0", show(&cr.1));
        assert_same("E13 copy_and_sum(buf, 0)", cr, rr);
    }
}

// ---------------------------------------------------------------------------
// E14 — check_permissions required == 0 never rejects
// ---------------------------------------------------------------------------

#[test]
fn err_e14_check_permissions_zero_required() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xE14);
    let mut cases: Vec<i32> = vec![0, -1, 1, INT_MIN, INT_MAX];
    for _ in 0..20_000 {
        cases.push(rng.i32_any());
    }
    for &perms in &cases {
        let cv = unsafe { (c.check_permissions)(perms, 0) };
        let rv = unsafe { (r.check_permissions)(perms, 0) };
        assert_eq!(cv, 1, "C: (perms & 0) == 0 always holds, perms={perms}");
        assert_eq!(cv, rv, "E14 check_permissions({perms}, 0)");
    }
}

// ---------------------------------------------------------------------------
// E16 — create_result_string with NULL op (no null check in the C)
// ---------------------------------------------------------------------------

#[test]
fn err_e16_create_result_string_null_op() {
    let (c, r) = libs();
    let null: *const c_char = std::ptr::null();
    for val in [0, 1, -1, 42, INT_MAX, INT_MIN] {
        let cr = capture_stdout(|| unsafe {
            let p = (c.create_result_string)(null, val);
            let s = read_cstr(p);
            cfree(p);
            s
        });
        let rr = capture_stdout(|| unsafe {
            let p = (r.create_result_string)(null, val);
            let s = read_cstr(p);
            cfree(p);
            s
        });
        assert!(
            cr.0.is_some(),
            "C returns a non-NULL buffer even for op == NULL"
        );
        assert_same(&format!("E16 create_result_string(NULL, {val})"), cr, rr);
    }
}

// ---------------------------------------------------------------------------
// E17 — create_result_string snprintf truncation
// ---------------------------------------------------------------------------

#[test]
fn err_e17_create_result_string_truncation() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xE17);
    for len in [0usize, 40, 48, 49, 50, 51, 52, 53, 60, 63, 64, 65, 100, 1000] {
        for val in [0, 7, -7, INT_MAX, INT_MIN] {
            let op: Vec<u8> = if len > 200 {
                rng.cbytes(len)
            } else {
                vec![b'Z'; len]
            };
            let co = cstring(&op);
            let ro = co.clone();
            let cr = capture_stdout(|| unsafe {
                let p = (c.create_result_string)(co.as_ptr(), val);
                let s = read_cstr(p);
                cfree(p);
                s
            });
            let rr = capture_stdout(|| unsafe {
                let p = (r.create_result_string)(ro.as_ptr(), val);
                let s = read_cstr(p);
                cfree(p);
                s
            });
            let got = cr.0.clone().expect("non-NULL");
            assert!(
                got.len() <= 63,
                "C overflowed the 64-byte buffer: {} bytes",
                got.len()
            );
            assert_same(&format!("E17 create_result_string(len={len}, {val})"), cr, rr);
        }
    }
}

// ---------------------------------------------------------------------------
// E20 — compare_operations degenerate but non-NULL strings
// ---------------------------------------------------------------------------

#[test]
fn err_e20_compare_operations_degenerate() {
    let (c, r) = libs();
    let cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (b"".to_vec(), b"".to_vec()),
        (b"".to_vec(), b"a".to_vec()),
        (b"a".to_vec(), b"".to_vec()),
        (b"".to_vec(), vec![0xFF]),
        (vec![0xFF], b"".to_vec()),
        (vec![0x80], vec![0x7F]),
        (vec![0x7F], vec![0x80]),
        (vec![0xFF], vec![0x01]),
        (vec![0x01], vec![0xFF]),
        (vec![0xFFu8; 64], vec![0xFFu8; 63]),
        (vec![0x80u8; 32], vec![0x80u8; 32]),
    ];
    for (a, b) in &cases {
        let (ca, cb) = (cstring(a), cstring(b));
        let cr = capture_stdout(|| unsafe { (c.compare_operations)(ca.as_ptr(), cb.as_ptr()) });
        let rr = capture_stdout(|| unsafe { (r.compare_operations)(ca.as_ptr(), cb.as_ptr()) });
        assert!(cr.1.is_empty(), "C should not print for non-NULL inputs");
        assert_same(
            &format!("E20 compare_operations({}, {})", show(a), show(b)),
            cr,
            rr,
        );
    }
}

// ===========================================================================
// Subprocess-driven rows: E1, E3, E10, E12 (allocation failure) and E4 (crash)
// ===========================================================================

const ENV_CASE: &str = "HARVEST_DIFF_CHILD_CASE";
const ENV_LIB: &str = "HARVEST_DIFF_CHILD_LIB";
const BEGIN: &[u8] = b"<<<LIBOUT-BEGIN>>>\n";
const END: &[u8] = b"<<<LIBOUT-END>>>\n";

struct ChildOut {
    libout: Vec<u8>,
    result: String,
    signal: Option<i32>,
    code: Option<i32>,
}

fn run_child(case: &str, which: &str) -> ChildOut {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["child_worker", "--exact", "--nocapture", "--test-threads=1"])
        .env(ENV_CASE, case)
        .env(ENV_LIB, which)
        // Keep glibc on a single arena so the injected heap-exhaustion state is
        // deterministic and identical for the C and the Rust child.
        .env("MALLOC_ARENA_MAX", "1")
        .output()
        .expect("spawn child");

    let stdout = out.stdout;
    let libout = match (find(&stdout, BEGIN), find(&stdout, END)) {
        (Some(i), Some(j)) if i + BEGIN.len() <= j => stdout[i + BEGIN.len()..j].to_vec(),
        _ => Vec::new(),
    };
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let result = stderr
        .lines()
        .find(|l| l.starts_with("CHILDRESULT "))
        .unwrap_or("")
        .to_string();
    ChildOut {
        libout,
        result,
        signal: out.status.signal(),
        code: out.status.code(),
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Compare the C child and the Rust child for one subprocess case.
///
/// The child exits non-zero (3/4/5) if the OOM state it was supposed to inject
/// did not actually materialise, so a clean exit is itself part of the check --
/// otherwise a differential test could "pass" without ever reaching the C's
/// error branch.
#[track_caller]
fn assert_child_same(case: &str) -> ChildOut {
    let cc = run_child(case, "c");
    let rc = run_child(case, "rust");
    assert!(
        !cc.result.is_empty() || cc.signal.is_some(),
        "C child for {case} produced no result line (exit {:?}/{:?})",
        cc.code,
        cc.signal
    );
    assert_eq!(
        cc.result, rc.result,
        "[{case}] result mismatch:\n  C   : {}\n  Rust: {}",
        cc.result, rc.result
    );
    assert_eq!(
        cc.libout,
        rc.libout,
        "[{case}] library stdout mismatch:\n  C   : {}\n  Rust: {}",
        show(&cc.libout),
        show(&rc.libout)
    );
    assert_eq!(
        (cc.signal, cc.code),
        (rc.signal, rc.code),
        "[{case}] termination mismatch"
    );
    if case != "e4" {
        assert_eq!(
            cc.code,
            Some(0),
            "[{case}] child bailed out ({}): the OOM state was not injected",
            cc.result
        );
    }
    cc
}

/// E1 (`create_result_string` malloc failure), E3 (`multiply_with_log`
/// propagating it) and E10 (`complexmode`'s own `malloc` failing).
#[test]
fn err_e1_e3_e10_alloc_failure() {
    // E1: malloc(64) fails -> NULL, nothing printed.
    let o = assert_child_same("e1");
    assert!(
        o.result.contains("ptr=NULL"),
        "E1 was not actually induced (C child said: {})",
        o.result
    );
    assert!(
        o.libout.is_empty(),
        "E1 must print nothing, got {}",
        show(&o.libout)
    );

    // E3: create_result_string fails -> return 0 and *log_msg == NULL.
    let o = assert_child_same("e3");
    assert!(
        o.result.contains("ret=0") && o.result.contains("msg=NULL"),
        "E3 was not actually induced (C child said: {})",
        o.result
    );

    // E10: complexmode's malloc(sizeof(Result)) fails -> -1 + message, and no
    // `Operation performed:` trailer because of the early return.
    let o = assert_child_same("e10");
    assert!(
        o.result.contains("ret=-1"),
        "E10 was not actually induced (C child said: {})",
        o.result
    );
    assert_eq!(
        o.libout,
        b"Failed to allocate result tracker\n".to_vec(),
        "E10 message from C: {}",
        show(&o.libout)
    );
}

/// E12 — `complexmode` mode 2 where `res_tracker` is allocated but the log
/// message allocation fails: `Log message creation failed` is printed instead of
/// `Mode 2: ...`, and the trailer still appears.
#[test]
fn err_e12_complexmode_mode2_log_failure() {
    let o = assert_child_same("e12");
    assert!(
        o.result.contains("ret=0"),
        "E12 was not actually induced (C child said: {})",
        o.result
    );
    assert_eq!(
        o.libout,
        b"Log message creation failed\nOperation performed: multiplication\n".to_vec(),
        "E12 message from C: {}",
        show(&o.libout)
    );
}

/// E4 — `multiply_with_log(a, b, NULL)`: the C has no null check and writes
/// through the pointer, so both libraries must die with the same signal.
#[test]
fn err_e4_multiply_with_log_null_outparam() {
    let o = assert_child_same("e4");
    assert_eq!(
        o.signal,
        Some(libc::SIGSEGV),
        "C child should have died with SIGSEGV, got signal={:?} code={:?}",
        o.signal,
        o.code
    );
}

// ---------------------------------------------------------------------------
// The child worker itself. A no-op unless the environment selects a case, so
// it is inert during a normal `cargo test` run.
// ---------------------------------------------------------------------------

#[test]
fn child_worker() {
    let Ok(case) = std::env::var(ENV_CASE) else {
        return;
    };
    let which = std::env::var(ENV_LIB).unwrap_or_else(|_| "c".into());
    let lib = if which == "c" {
        Lib::open("C", &c_so_path())
    } else {
        Lib::open("Rust", &rust_so_path())
    };
    child_run(&case, &lib);
}

fn write_fd(fd: c_int, bytes: &[u8]) {
    unsafe {
        libc::write(fd, bytes.as_ptr() as *const c_void, bytes.len());
    }
}

/// Current address-space size in bytes, from `/proc/self/statm` (field 0 is the
/// total program size in pages).
fn vsz_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").expect("statm");
    let pages: u64 = s.split_whitespace().next().unwrap().parse().unwrap();
    pages * 4096
}

/// Allocation-free variant of `vsz_bytes`, usable while the heap is drained.
fn vsz_bytes_raw() -> u64 {
    let mut buf = [0u8; 128];
    unsafe {
        let fd = libc::open(c"/proc/self/statm".as_ptr(), libc::O_RDONLY);
        if fd < 0 {
            return 0;
        }
        let n = libc::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
        libc::close(fd);
        if n <= 0 {
            return 0;
        }
        let mut pages: u64 = 0;
        for &b in &buf[..n as usize] {
            if b.is_ascii_digit() {
                pages = pages * 10 + (b - b'0') as u64;
            } else {
                break;
            }
        }
        pages * 4096
    }
}

/// Allocation-free diagnostic trace, so a stuck child can be located.
fn trace(step: &str) {
    if std::env::var_os("HARVEST_DIFF_CHILD_TRACE").is_some() {
        write_fd(2, b"CHILDTRACE ");
        write_fd(2, step.as_bytes());
        write_fd(2, b"\n");
    }
}

/// Allocation-free `label=<number>` diagnostic on stderr.
fn trace_num(label: &str, v: u64) {
    if std::env::var_os("HARVEST_DIFF_CHILD_TRACE").is_none() {
        return;
    }
    let mut buf = [0u8; 24];
    let n = u64_to_buf(v, &mut buf);
    write_fd(2, b"CHILDTRACE ");
    write_fd(2, label.as_bytes());
    write_fd(2, b"=");
    write_fd(2, &buf[..n]);
    write_fd(2, b"\n");
}

/// Allocation-free `u64` -> decimal, for reporting from inside the drained state.
fn u64_to_buf(mut v: u64, buf: &mut [u8; 24]) -> usize {
    if v == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 24];
    let mut n = 0;
    while v > 0 {
        tmp[n] = b'0' + (v % 10) as u8;
        v /= 10;
        n += 1;
    }
    for i in 0..n {
        buf[i] = tmp[n - 1 - i];
    }
    n
}

/// Upper bound on drain iterations. 64 MiB of 48-byte chunks is ~1.4M
/// allocations, so 40M is a generous safety valve that still terminates quickly
/// if `RLIMIT_AS` turns out not to be enforced.
const DRAIN_CAP: u64 = 40_000_000;

/// Drain the heap until `libc::malloc(chunk)` returns NULL.
///
/// Each block is written to and each pointer is passed through
/// `std::hint::black_box`. Both are essential: with a plain
/// `let p = malloc(n); if p.is_null() { .. }` loop, `-O` deletes the whole loop
/// as an unused allocation and folds `is_null()` to `false`, which silently
/// turns the OOM injection into a no-op that never reaches the C's error branch.
///
/// Returns `(mid1, mid2, hit_cap)`. The two `mid` pointers are chunks taken
/// early in the drain, i.e. deep inside the heap and far from the top chunk.
/// Releasing THOSE puts `chunk`-sized blocks back into tcache without letting
/// them coalesce with the top chunk (which would enlarge the top, or trigger a
/// `systrim` that hands address space back and undoes the whole exhaustion).
///
/// No Rust allocation may happen between this call and the `setrlimit` restore.
unsafe fn drain(chunk: usize, label: &str) -> (*mut c_void, *mut c_void, bool) {
    assert!(chunk >= std::mem::size_of::<u64>());
    const MID1_AT: u64 = 4096;
    const MID2_AT: u64 = 4097;
    let mut mid1: *mut c_void = std::ptr::null_mut();
    let mut mid2: *mut c_void = std::ptr::null_mut();
    let mut n: u64 = 0;
    loop {
        let p = std::hint::black_box(unsafe { libc::malloc(chunk) });
        if p.is_null() {
            trace_num(label, n);
            return (mid1, mid2, false);
        }
        // A real store, so the page is faulted in and the block is observably
        // used; then let the pointer escape so the allocation cannot be removed.
        unsafe { (p as *mut u64).write(n) };
        std::hint::black_box(p);
        n += 1;
        if n == MID1_AT {
            mid1 = p;
        } else if n == MID2_AT {
            mid2 = p;
        }
        if n >= DRAIN_CAP {
            trace_num(label, n);
            return (mid1, mid2, true);
        }
    }
}

fn child_run(case: &str, lib: &Lib) -> ! {
    unsafe {
        trace("start");
        // E4 needs no memory pressure: just deref NULL through the library.
        if case == "e4" {
            libc::printf(c"<<<LIBOUT-BEGIN>>>\n".as_ptr());
            libc::fflush(std::ptr::null_mut());
            let _ = (lib.multiply_with_log)(6, 7, std::ptr::null_mut());
            // Unreachable if the deref faulted as the C does.
            write_fd(2, b"CHILDRESULT survived-null-outparam\n");
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }

        // Warm up stdout so that its FILE buffer is allocated BEFORE the heap
        // is drained (otherwise the library's own printf could not run). This
        // doubles as the begin marker the parent scans for.
        libc::printf(c"<<<LIBOUT-BEGIN>>>\n".as_ptr());
        libc::fflush(std::ptr::null_mut());
        trace("warmed-stdout");

        // Snapshot and lower the address-space limit so that draining is quick.
        let mut orig = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        assert_eq!(libc::getrlimit(libc::RLIMIT_AS, &mut orig), 0);
        trace("got-rlimit");
        let target = vsz_bytes() + 64 * 1024 * 1024;
        trace("read-vsz");
        let capped = if orig.rlim_max == libc::RLIM_INFINITY {
            target
        } else {
            target.min(orig.rlim_max)
        };
        let tight = libc::rlimit {
            rlim_cur: capped,
            rlim_max: orig.rlim_max,
        };
        trace_num("vsz", vsz_bytes());
        trace_num("cap", capped);
        assert_eq!(libc::setrlimit(libc::RLIMIT_AS, &tight), 0);
        let mut back = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::getrlimit(libc::RLIMIT_AS, &mut back);
        trace_num("rlimit-now", back.rlim_cur);
        trace("set-rlimit");

        // glibc chunk sizes: a 40-byte request (`sizeof(Result)`) needs a
        // 48-byte chunk, a 64-byte request (`create_result_string`) needs an
        // 80-byte chunk. Drain BOTH size classes so that neither request can be
        // served: first fill every 48-byte hole, then every 80-byte hole. Doing
        // it in one pass is not enough -- in a multi-threaded process glibc uses
        // a secondary arena where a failed 40-byte request does not imply a
        // failed 64-byte one.
        let (mid1, mid2, cap_a) = drain(40, "drained40");
        trace_num("vsz-after-40", vsz_bytes_raw());
        let (_, _, cap_b) = drain(64, "drained64");
        trace_num("vsz-after-64", vsz_bytes_raw());
        trace("drained");
        if cap_a || cap_b {
            // setrlimit is not being enforced -- fail loudly instead of
            // pretending the OOM path was exercised.
            libc::setrlimit(libc::RLIMIT_AS, &orig);
            write_fd(2, b"CHILDRESULT drain-cap-hit\n");
            libc::fflush(std::ptr::null_mut());
            libc::_exit(3);
        }

        // Row E12 needs `malloc(40)` to SUCCEED (so `complexmode` gets its
        // `Result`) while `malloc(64)` still FAILS (so `create_result_string`
        // returns NULL). Hand exactly two 48-byte chunks back from deep inside
        // the heap: one is consumed by the verification probe below, the other
        // is what the library will take.
        if case == "e12" {
            if mid1.is_null() || mid2.is_null() {
                libc::setrlimit(libc::RLIMIT_AS, &orig);
                write_fd(2, b"CHILDRESULT drain-too-short\n");
                libc::fflush(std::ptr::null_mut());
                libc::_exit(4);
            }
            libc::free(mid1);
            libc::free(mid2);
            trace("freed-two-chunks");
        }

        // Verify the injected state is EXACTLY what the row requires, before
        // calling the library. Otherwise a silently-not-induced OOM would make
        // the differential test pass vacuously.
        let want40 = case == "e12"; // must succeed only for E12
        let probe40 = libc::malloc(40);
        let probe64 = libc::malloc(64);
        trace_num("probe40", probe40 as u64);
        trace_num("probe64", probe64 as u64);
        if probe40.is_null() == want40 || !probe64.is_null() {
            libc::setrlimit(libc::RLIMIT_AS, &orig);
            write_fd(2, b"CHILDRESULT oom-setup-failed\n");
            libc::fflush(std::ptr::null_mut());
            libc::_exit(5);
        }

        // ---- the call under test (no Rust allocation in here) ----
        let mut ret: c_int = 0;
        let mut ptr: *mut c_char = std::ptr::null_mut();
        let mut msg: *mut c_char = 1usize as *mut c_char;
        match case {
            "e1" => {
                ptr = (lib.create_result_string)(c"multiply".as_ptr(), 42);
            }
            "e3" => {
                ret = (lib.multiply_with_log)(6, 7, &mut msg);
            }
            "e10" => {
                ret = (lib.complexmode)(1, 2, 3, 4);
            }
            "e12" => {
                ret = (lib.complexmode)(2, 6, 7, 0);
            }
            _ => {
                write_fd(2, b"CHILDRESULT unknown-case\n");
                libc::_exit(2);
            }
        }
        trace("called");
        // Flush whatever the library printed, then emit the end marker with a
        // raw write (printf could need to allocate while the heap is drained).
        libc::fflush(std::ptr::null_mut());
        write_fd(1, END);
        trace("flushed");

        // Restore the limit; from here on normal allocation is safe again.
        assert_eq!(libc::setrlimit(libc::RLIMIT_AS, &orig), 0);
        trace("restored");

        let ptr_desc = if ptr.is_null() {
            "ptr=NULL".to_string()
        } else {
            format!("ptr={:?}", read_cstr(ptr))
        };
        let msg_desc = if msg as usize == 1 {
            "msg=UNWRITTEN".to_string()
        } else if msg.is_null() {
            "msg=NULL".to_string()
        } else {
            format!("msg={:?}", read_cstr(msg))
        };
        let line = format!("CHILDRESULT ret={ret} {ptr_desc} {msg_desc}\n");
        write_fd(2, line.as_bytes());
        libc::fflush(std::ptr::null_mut());
        libc::_exit(0);
    }
}
