//! Differential tests: every function is invoked through the exported symbols
//! of the C `.so` and of the Rust `.so` (never called directly), and both the
//! return value and everything written to stdout must match byte-for-byte.

mod common;

use std::ffi::{c_char, c_int, c_void};

use common::{c_free, c_str_bytes, capture_stdout, show, Pair, Side};

type FnCreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
type FnCheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnSafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type FnMultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
type FnCopyAndSum = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
type FnCompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnComplexmode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// Interesting `int` values, including the boundaries.
const INTS: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    7,
    -7,
    42,
    -42,
    255,
    256,
    -256,
    1000,
    -1000,
    65535,
    65536,
    0x0100_0000,
    46341,
    -46341,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
];

// ---------------------------------------------------------------------------
// Level 1: check_permissions  (pure, no output)
// ---------------------------------------------------------------------------

// --- test ---
fn check_permissions_matches() {
    let p = Pair::load();
    let c: libloading::Symbol<FnCheckPermissions> = p.sym(Side::C, "check_permissions");
    let r: libloading::Symbol<FnCheckPermissions> = p.sym(Side::Rust, "check_permissions");

    let mut cases: Vec<(c_int, c_int)> = Vec::new();
    // Exhaustive over the permission-bit space that the library cares about.
    for perms in 0..0o1000 {
        for required in [0, 0o100, 0o200, 0o400, 0o600, 0o700, 0o644, 0o777, 0o1000] {
            cases.push((perms, required));
        }
    }
    // Plus wide-ranging / negative values.
    for &a in INTS {
        for &b in INTS {
            cases.push((a, b));
        }
    }

    for (perms, required) in cases {
        let cv = unsafe { c(perms, required) };
        let rv = unsafe { r(perms, required) };
        assert_eq!(
            cv, rv,
            "check_permissions({perms:#o}, {required:#o}): C={cv} Rust={rv}"
        );
    }
}

// ---------------------------------------------------------------------------
// Level 1: create_result_string  (malloc + snprintf, no output)
// ---------------------------------------------------------------------------

// --- test ---
fn create_result_string_matches() {
    let p = Pair::load();
    let c: libloading::Symbol<FnCreateResultString> = p.sym(Side::C, "create_result_string");
    let r: libloading::Symbol<FnCreateResultString> = p.sym(Side::Rust, "create_result_string");

    let ops: Vec<String> = vec![
        String::new(),
        "a".into(),
        "multiply".into(),
        "addition".into(),
        "array_sum".into(),
        "complex".into(),
        "none".into(),
        "with space".into(),
        "%d%s%%".into(),           // format chars must be passed through, not expanded
        "\ttab\tand\\backslash".into(),
        "0123456789".into(),
        "x".repeat(31),
        "y".repeat(32),
        "z".repeat(40),
        "w".repeat(63),
        "v".repeat(64),
        "u".repeat(200), // forces snprintf truncation at 64 bytes
        // non-ASCII bytes
        String::from_utf8_lossy(&[0xC3, 0xA9, 0xC3, 0xA8]).to_string(),
    ];

    for op in &ops {
        let opz = cstr(op);
        for &val in INTS {
            let cp = unsafe { c(opz.as_ptr() as *const c_char, val) };
            let rp = unsafe { r(opz.as_ptr() as *const c_char, val) };
            assert!(!cp.is_null(), "C returned NULL");
            assert!(!rp.is_null(), "Rust returned NULL");
            let cb = unsafe { c_str_bytes(cp) };
            let rb = unsafe { c_str_bytes(rp) };
            assert_eq!(
                cb,
                rb,
                "create_result_string(op.len={}, {val}):\n  C   = {:?}\n  Rust= {:?}",
                op.len(),
                show(&cb),
                show(&rb)
            );
            // The whole 64-byte buffer must be identical as far as the string
            // plus its terminator go.
            assert!(cb.len() < 64, "C string exceeded buffer");
            unsafe {
                c_free(cp as *mut c_void);
                c_free(rp as *mut c_void);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: safe_add  (calls check_permissions, may print)
// ---------------------------------------------------------------------------

// --- test ---
fn safe_add_matches() {
    let p = Pair::load();
    let c: libloading::Symbol<FnSafeAdd> = p.sym(Side::C, "safe_add");
    let r: libloading::Symbol<FnSafeAdd> = p.sym(Side::Rust, "safe_add");

    let perms_cases: Vec<c_int> = {
        let mut v: Vec<c_int> = (0..0o1000).collect();
        v.extend_from_slice(&[-1, 0o644, 0o600, 0o777, 0o1777, i32::MIN, i32::MAX]);
        v
    };

    for &perms in &perms_cases {
        for &a in INTS {
            for &b in &[0, 1, -1, 7, i32::MAX, i32::MIN, 1000] {
                let (cv, cout) = capture_stdout(|| unsafe { c(a, b, perms) });
                let (rv, rout) = capture_stdout(|| unsafe { r(a, b, perms) });
                assert_eq!(cv, rv, "safe_add({a}, {b}, {perms:#o}) return value");
                assert_eq!(
                    cout,
                    rout,
                    "safe_add({a}, {b}, {perms:#o}) stdout:\n  C   = {}\n  Rust= {}",
                    show(&cout),
                    show(&rout)
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: multiply_with_log  (calls create_result_string, out-parameter)
// ---------------------------------------------------------------------------

// --- test ---
fn multiply_with_log_matches() {
    let p = Pair::load();
    let c: libloading::Symbol<FnMultiplyWithLog> = p.sym(Side::C, "multiply_with_log");
    let r: libloading::Symbol<FnMultiplyWithLog> = p.sym(Side::Rust, "multiply_with_log");

    for &a in INTS {
        for &b in INTS {
            let mut cmsg: *mut c_char = std::ptr::null_mut();
            let mut rmsg: *mut c_char = std::ptr::null_mut();

            let (cv, cout) = capture_stdout(|| unsafe { c(a, b, &mut cmsg) });
            let (rv, rout) = capture_stdout(|| unsafe { r(a, b, &mut rmsg) });

            assert_eq!(cv, rv, "multiply_with_log({a}, {b}) return value");
            assert_eq!(cout, rout, "multiply_with_log({a}, {b}) stdout");
            assert_eq!(
                cmsg.is_null(),
                rmsg.is_null(),
                "multiply_with_log({a}, {b}) log_msg nullness"
            );
            if !cmsg.is_null() {
                let cb = unsafe { c_str_bytes(cmsg) };
                let rb = unsafe { c_str_bytes(rmsg) };
                assert_eq!(
                    cb,
                    rb,
                    "multiply_with_log({a}, {b}) log_msg:\n  C   = {}\n  Rust= {}",
                    show(&cb),
                    show(&rb)
                );
                unsafe {
                    c_free(cmsg as *mut c_void);
                    c_free(rmsg as *mut c_void);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: copy_and_sum  (NULL check, malloc, memcpy, may print)
// ---------------------------------------------------------------------------

// --- test ---
fn copy_and_sum_matches() {
    let p = Pair::load();
    let c: libloading::Symbol<FnCopyAndSum> = p.sym(Side::C, "copy_and_sum");
    let r: libloading::Symbol<FnCopyAndSum> = p.sym(Side::Rust, "copy_and_sum");

    // NULL source, across several counts (including negative).
    for &count in &[-1, 0, 1, 3, 100, i32::MIN, i32::MAX] {
        let (cv, cout) = capture_stdout(|| unsafe { c(std::ptr::null(), count) });
        let (rv, rout) = capture_stdout(|| unsafe { r(std::ptr::null(), count) });
        assert_eq!(cv, rv, "copy_and_sum(NULL, {count}) return value");
        assert_eq!(
            cout,
            rout,
            "copy_and_sum(NULL, {count}) stdout:\n  C   = {}\n  Rust= {}",
            show(&cout),
            show(&rout)
        );
    }

    // Non-NULL source with a valid count.
    let arrays: Vec<Vec<c_int>> = vec![
        vec![0],
        vec![1, 2, 3],
        vec![-1, -2, -3],
        vec![i32::MAX, 1, 0],
        vec![i32::MIN, -1, 0],
        vec![i32::MAX, i32::MAX, i32::MAX],
        vec![i32::MIN, i32::MIN, i32::MIN],
        (0..64).collect(),
        (0..1000).map(|i| i * 7 - 3).collect(),
        vec![0; 257],
    ];
    for arr in &arrays {
        for &count in &[0usize, 1, arr.len()] {
            if count > arr.len() {
                continue;
            }
            let n = count as c_int;
            let (cv, cout) = capture_stdout(|| unsafe { c(arr.as_ptr(), n) });
            let (rv, rout) = capture_stdout(|| unsafe { r(arr.as_ptr(), n) });
            assert_eq!(cv, rv, "copy_and_sum(len={}, {n}) return value", arr.len());
            assert_eq!(cout, rout, "copy_and_sum(len={}, {n}) stdout", arr.len());
        }
    }

    // Negative counts with a valid pointer: `count * sizeof(int)` wraps to a
    // huge size_t, so malloc fails and the error path is taken.
    let arr = [1, 2, 3];
    for &count in &[-1, -3, -1000, i32::MIN] {
        let (cv, cout) = capture_stdout(|| unsafe { c(arr.as_ptr(), count) });
        let (rv, rout) = capture_stdout(|| unsafe { r(arr.as_ptr(), count) });
        assert_eq!(cv, rv, "copy_and_sum(arr, {count}) return value");
        assert_eq!(
            cout,
            rout,
            "copy_and_sum(arr, {count}) stdout:\n  C   = {}\n  Rust= {}",
            show(&cout),
            show(&rout)
        );
    }
}

// ---------------------------------------------------------------------------
// Level 2: compare_operations  (NULL checks + strcmp)
// ---------------------------------------------------------------------------

// --- test ---
fn compare_operations_matches() {
    let p = Pair::load();
    let c: libloading::Symbol<FnCompareOperations> = p.sym(Side::C, "compare_operations");
    let r: libloading::Symbol<FnCompareOperations> = p.sym(Side::Rust, "compare_operations");

    let strings: Vec<Vec<u8>> = [
        "", "a", "b", "A", "ab", "abc", "abd", "abcd", "none", "addition", "multiplication",
        "array_sum", "complex", "zzzz", "\x01", "\x7f",
    ]
    .iter()
    .map(|s| cstr(s))
    .collect();

    // Both NULL / one NULL.
    let nn: *const c_char = std::ptr::null();
    let some = cstr("abc");
    let null_cases: Vec<(*const c_char, *const c_char)> = vec![
        (nn, nn),
        (nn, some.as_ptr() as *const c_char),
        (some.as_ptr() as *const c_char, nn),
    ];
    for (a, b) in null_cases {
        let (cv, cout) = capture_stdout(|| unsafe { c(a, b) });
        let (rv, rout) = capture_stdout(|| unsafe { r(a, b) });
        assert_eq!(cv, rv, "compare_operations NULL-case return value");
        assert_eq!(
            cout,
            rout,
            "compare_operations NULL-case stdout:\n  C   = {}\n  Rust= {}",
            show(&cout),
            show(&rout)
        );
    }

    for s1 in &strings {
        for s2 in &strings {
            let (cv, cout) =
                capture_stdout(|| unsafe { c(s1.as_ptr() as *const _, s2.as_ptr() as *const _) });
            let (rv, rout) =
                capture_stdout(|| unsafe { r(s1.as_ptr() as *const _, s2.as_ptr() as *const _) });
            assert_eq!(
                cv,
                rv,
                "compare_operations({:?}, {:?}) return value: C={cv} Rust={rv}",
                show(&s1[..s1.len() - 1]),
                show(&s2[..s2.len() - 1])
            );
            assert_eq!(cout, rout, "compare_operations stdout");
        }
    }
}

// ---------------------------------------------------------------------------
// Level 3: complexmode  (top-level entry point from lib.h)
// ---------------------------------------------------------------------------

// --- test ---
fn complexmode_matches() {
    let p = Pair::load();
    let c: libloading::Symbol<FnComplexmode> = p.sym(Side::C, "complexmode");
    let r: libloading::Symbol<FnComplexmode> = p.sym(Side::Rust, "complexmode");

    let modes: Vec<c_int> = vec![
        i32::MIN,
        -100,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        100,
        i32::MAX,
    ];

    let vals: &[c_int] = &[
        0,
        1,
        -1,
        2,
        -3,
        7,
        42,
        -42,
        255,
        1000,
        -1000,
        65536,
        46341,
        -46341,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];

    for &mode in &modes {
        for &v1 in vals {
            for &v2 in vals {
                for &v3 in &[0, 1, -1, 12345, i32::MAX, i32::MIN] {
                    let (cv, cout) = capture_stdout(|| unsafe { c(mode, v1, v2, v3) });
                    let (rv, rout) = capture_stdout(|| unsafe { r(mode, v1, v2, v3) });
                    assert_eq!(
                        cv, rv,
                        "complexmode({mode}, {v1}, {v2}, {v3}) return value: C={cv} Rust={rv}"
                    );
                    assert_eq!(
                        cout,
                        rout,
                        "complexmode({mode}, {v1}, {v2}, {v3}) stdout:\n  C   = {}\n  Rust= {}",
                        show(&cout),
                        show(&rout)
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Export-surface parity: every symbol the C .so exports, the Rust .so must too.
// ---------------------------------------------------------------------------

// --- test ---
fn all_public_symbols_resolvable() {
    let p = Pair::load();
    for name in [
        "create_result_string",
        "check_permissions",
        "safe_add",
        "multiply_with_log",
        "copy_and_sum",
        "compare_operations",
        "complexmode",
    ] {
        let _c: libloading::Symbol<*const c_void> = p.sym(Side::C, name);
        let _r: libloading::Symbol<*const c_void> = p.sym(Side::Rust, name);
    }
}

// ---------------------------------------------------------------------------
// Edge cases an external caller can legitimately trigger
// ---------------------------------------------------------------------------

// --- test ---
fn edge_cases_match() {
    let p = Pair::load();
    let crs_c: libloading::Symbol<FnCreateResultString> = p.sym(Side::C, "create_result_string");
    let crs_r: libloading::Symbol<FnCreateResultString> = p.sym(Side::Rust, "create_result_string");

    // NULL `op`: glibc's snprintf renders "(null)" for %s. Both sides must go
    // through the same libc and produce the same bytes.
    for &val in &[0, -1, i32::MAX, i32::MIN] {
        let cp = unsafe { crs_c(std::ptr::null(), val) };
        let rp = unsafe { crs_r(std::ptr::null(), val) };
        assert!(!cp.is_null() && !rp.is_null());
        let cb = unsafe { c_str_bytes(cp) };
        let rb = unsafe { c_str_bytes(rp) };
        assert_eq!(
            cb,
            rb,
            "create_result_string(NULL, {val}):\n  C   = {}\n  Rust= {}",
            show(&cb),
            show(&rb)
        );
        unsafe {
            c_free(cp as *mut c_void);
            c_free(rp as *mut c_void);
        }
    }

    // strcmp over bytes with the high bit set (unsigned comparison).
    let cmp_c: libloading::Symbol<FnCompareOperations> = p.sym(Side::C, "compare_operations");
    let cmp_r: libloading::Symbol<FnCompareOperations> = p.sym(Side::Rust, "compare_operations");
    let raw: Vec<Vec<u8>> = vec![
        vec![0x80, 0],
        vec![0xff, 0],
        vec![0x7f, 0],
        vec![0x01, 0x80, 0],
        vec![0x01, 0x7f, 0],
        vec![0xc3, 0xa9, 0],
        vec![0],
    ];
    for a in &raw {
        for b in &raw {
            let (cv, cout) = capture_stdout(|| unsafe {
                cmp_c(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char)
            });
            let (rv, rout) = capture_stdout(|| unsafe {
                cmp_r(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char)
            });
            assert_eq!(cv, rv, "compare_operations({a:02x?}, {b:02x?}): C={cv} Rust={rv}");
            assert_eq!(cout, rout, "compare_operations({a:02x?}, {b:02x?}) stdout");
        }
    }
}

// ---------------------------------------------------------------------------
// Randomized sweep over the whole public surface
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes LCG constants.
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn next_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
}

// --- test ---
fn randomized_sweep_matches() {
    let p = Pair::load();
    let cm_c: libloading::Symbol<FnComplexmode> = p.sym(Side::C, "complexmode");
    let cm_r: libloading::Symbol<FnComplexmode> = p.sym(Side::Rust, "complexmode");
    let sa_c: libloading::Symbol<FnSafeAdd> = p.sym(Side::C, "safe_add");
    let sa_r: libloading::Symbol<FnSafeAdd> = p.sym(Side::Rust, "safe_add");
    let cp_c: libloading::Symbol<FnCheckPermissions> = p.sym(Side::C, "check_permissions");
    let cp_r: libloading::Symbol<FnCheckPermissions> = p.sym(Side::Rust, "check_permissions");
    let ml_c: libloading::Symbol<FnMultiplyWithLog> = p.sym(Side::C, "multiply_with_log");
    let ml_r: libloading::Symbol<FnMultiplyWithLog> = p.sym(Side::Rust, "multiply_with_log");
    let cs_c: libloading::Symbol<FnCopyAndSum> = p.sym(Side::C, "copy_and_sum");
    let cs_r: libloading::Symbol<FnCopyAndSum> = p.sym(Side::Rust, "copy_and_sum");

    let mut rng = Lcg(0x1234_5678_9abc_def0);

    for _ in 0..4000 {
        // check_permissions
        let (a, b) = (rng.next_i32(), rng.next_i32());
        assert_eq!(unsafe { cp_c(a, b) }, unsafe { cp_r(a, b) },
            "check_permissions({a}, {b})");

        // safe_add, with permissions that sometimes satisfy the mask
        let perms = if rng.next_u32() & 1 == 0 {
            rng.next_i32()
        } else {
            (rng.next_i32() & !0o600) | 0o600
        };
        let (x, y) = (rng.next_i32(), rng.next_i32());
        let (cv, cout) = capture_stdout(|| unsafe { sa_c(x, y, perms) });
        let (rv, rout) = capture_stdout(|| unsafe { sa_r(x, y, perms) });
        assert_eq!(cv, rv, "safe_add({x}, {y}, {perms})");
        assert_eq!(cout, rout, "safe_add({x}, {y}, {perms}) stdout");

        // multiply_with_log
        let (x, y) = (rng.next_i32(), rng.next_i32());
        let mut cmsg: *mut c_char = std::ptr::null_mut();
        let mut rmsg: *mut c_char = std::ptr::null_mut();
        let (cv, cout) = capture_stdout(|| unsafe { ml_c(x, y, &mut cmsg) });
        let (rv, rout) = capture_stdout(|| unsafe { ml_r(x, y, &mut rmsg) });
        assert_eq!(cv, rv, "multiply_with_log({x}, {y})");
        assert_eq!(cout, rout, "multiply_with_log({x}, {y}) stdout");
        assert_eq!(cmsg.is_null(), rmsg.is_null());
        if !cmsg.is_null() {
            assert_eq!(
                unsafe { c_str_bytes(cmsg) },
                unsafe { c_str_bytes(rmsg) },
                "multiply_with_log({x}, {y}) log message"
            );
            unsafe {
                c_free(cmsg as *mut c_void);
                c_free(rmsg as *mut c_void);
            }
        }

        // copy_and_sum over a random buffer
        let n = (rng.next_u32() % 17) as usize;
        let buf: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let ptr = if n == 0 {
            std::ptr::NonNull::<c_int>::dangling().as_ptr() as *const c_int
        } else {
            buf.as_ptr()
        };
        let ni = n as c_int;
        let (cv, cout) = capture_stdout(|| unsafe { cs_c(ptr, ni) });
        let (rv, rout) = capture_stdout(|| unsafe { cs_r(ptr, ni) });
        assert_eq!(cv, rv, "copy_and_sum(len={n})");
        assert_eq!(cout, rout, "copy_and_sum(len={n}) stdout");

        // complexmode
        let mode = match rng.next_u32() % 8 {
            k @ 0..=5 => k as c_int,
            6 => rng.next_i32(),
            _ => -(rng.next_i32() & 0xffff),
        };
        let (v1, v2, v3) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        let (cv, cout) = capture_stdout(|| unsafe { cm_c(mode, v1, v2, v3) });
        let (rv, rout) = capture_stdout(|| unsafe { cm_r(mode, v1, v2, v3) });
        assert_eq!(cv, rv, "complexmode({mode}, {v1}, {v2}, {v3})");
        assert_eq!(
            cout,
            rout,
            "complexmode({mode}, {v1}, {v2}, {v3}) stdout:\n  C   = {}\n  Rust= {}",
            show(&cout),
            show(&rout)
        );
    }
}

// ---------------------------------------------------------------------------
// Manual harness (harness = false): runs every check sequentially so that no
// other thread writes to fd 1 while it is redirected.
// ---------------------------------------------------------------------------

fn main() {
    let cases: &[(&str, fn())] = &[
        // lowest level first
        ("check_permissions", check_permissions_matches),
        ("create_result_string", create_result_string_matches),
        ("safe_add", safe_add_matches),
        ("multiply_with_log", multiply_with_log_matches),
        ("copy_and_sum", copy_and_sum_matches),
        ("compare_operations", compare_operations_matches),
        // top level
        ("complexmode", complexmode_matches),
        ("edge_cases", edge_cases_match),
        ("randomized_sweep", randomized_sweep_matches),
        ("exported_symbols", all_public_symbols_resolvable),
    ];

    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with("--"));

    // Force the C and Rust libraries to be built/loaded once up front so that
    // any build output happens before stdout redirection starts.
    let _ = Pair::load();

    let mut failed = Vec::new();
    for (name, f) in cases {
        if let Some(ref want) = filter {
            if !name.contains(want.as_str()) {
                continue;
            }
        }
        eprint!("test {name} ... ");
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match res {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failed.push(*name);
            }
        }
    }

    if failed.is_empty() {
        eprintln!("\nall differential checks passed");
    } else {
        eprintln!("\nfailed: {failed:?}");
        std::process::exit(1);
    }
}
