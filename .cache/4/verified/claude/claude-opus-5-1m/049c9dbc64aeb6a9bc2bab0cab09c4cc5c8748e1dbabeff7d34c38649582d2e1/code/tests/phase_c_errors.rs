//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test builds the exact invalid input, calls BOTH `.so`s through `dlsym`
//! and asserts that the same sentinel/return value AND the same diagnostic bytes
//! come back — not merely "both failed".

mod common;

use std::ffi::{c_char, c_int, CString};

use common::{c, capture, diff, iters, r, ComputeState, Lib, OperationFunc, Rng, EDGES, SEED};

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Assert both libraries return NULL from `get_operation` and print nothing.
fn assert_get_operation_null(opcode: c_int, ctx: &str) {
    let (cv, cout) = capture(|| c().get_operation(opcode));
    let (rv, rout) = capture(|| r().get_operation(opcode));
    assert!(cv.is_none(), "C get_operation({opcode}) should be NULL [{ctx}]");
    assert!(
        rv.is_none(),
        "Rust get_operation({opcode}) should be NULL, got {:?} [{ctx}]",
        rv.map(|f| f as usize)
    );
    assert!(cout.is_empty(), "C printed on rejection [{ctx}]: {}", common::show(&cout));
    assert_eq!(cout, rout, "stdout mismatch [{ctx}]");
}

// ===========================================================================
// E1 / E2 / E3 — get_operation range rejection (out-of-range int dispatch tag)
// ===========================================================================

#[test]
fn e1_get_operation_negative_opcode() {
    for opcode in [-1, -2, -3, -4, -5, -100, -0x8000, -0x1_0000] {
        assert_get_operation_null(opcode, "E1");
    }
}

#[test]
fn e2_get_operation_opcode_at_and_past_upper_bound() {
    for opcode in [4, 5, 6, 7, 8, 100, 0x1_0000] {
        assert_get_operation_null(opcode, "E2");
    }
}

#[test]
fn e3_get_operation_extreme_opcodes() {
    for opcode in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        assert_get_operation_null(opcode, "E3 extreme");
    }
    // Dense sweep over the boundary neighbourhood: exactly 0..=3 must be valid.
    for opcode in -8i32..=11 {
        let cv = c().get_operation(opcode);
        let rv = r().get_operation(opcode);
        let valid = (0..4).contains(&opcode);
        assert_eq!(cv.is_some(), valid, "C validity wrong at opcode {opcode}");
        assert_eq!(
            rv.is_some(),
            cv.is_some(),
            "Rust/C NULL-ness disagreement at opcode {opcode}"
        );
    }
    // Fuzz the tag: any `int` with no valid variant must be rejected identically.
    let mut rng = Rng::new(SEED ^ 0xE3);
    for _ in 0..iters(4096) {
        let opcode = rng.next_i32();
        let cv = c().get_operation(opcode);
        let rv = r().get_operation(opcode);
        assert_eq!(
            cv.is_none(),
            rv.is_none(),
            "NULL-ness disagreement at random opcode {opcode}"
        );
        if let (Some(cf), Some(rf)) = (cv, rv) {
            assert!(
                (0..4).contains(&opcode),
                "non-NULL returned for out-of-range opcode {opcode}"
            );
            // and the two must dispatch to equivalent arithmetic
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            assert_eq!(unsafe { cf(a, b) }, unsafe { rf(a, b) });
        }
    }
    // Edge list too.
    for &opcode in EDGES {
        let cv = c().get_operation(opcode);
        let rv = r().get_operation(opcode);
        assert_eq!(cv.is_none(), rv.is_none(), "disagreement at edge opcode {opcode}");
    }
}

// ===========================================================================
// E4..E7 — execute_operation with NULL func
// ===========================================================================

#[test]
fn e4_execute_operation_null_func() {
    let mut rng = Rng::new(SEED ^ 0xE4);
    for name_s in ["XOR", "SHIFT", "op", "a longer operation name"] {
        let name = cstr(name_s);
        let np = name.as_ptr();
        for i in 0..iters(20) {
            let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
            diff(&format!("E4 {name_s} #{i} ({a},{b})"), move |l: &Lib| unsafe {
                l.execute_operation(None, a, b, np)
            });
        }
    }
    // The sentinel really is 0, and the two `Variable ...` lines are suppressed.
    let name = cstr("XOR");
    let np = name.as_ptr();
    let (cv, cout) = capture(|| unsafe { c().execute_operation(None, 5, 6, np) });
    let (rv, rout) = capture(|| unsafe { r().execute_operation(None, 5, 6, np) });
    assert_eq!(cv, 0, "C sentinel must be 0");
    assert_eq!(rv, 0, "Rust sentinel must be 0");
    assert_eq!(
        cout, b"Error: Operation function pointer is NULL for XOR\n".to_vec(),
        "unexpected C diagnostic: {}",
        common::show(&cout)
    );
    assert_eq!(cout, rout);
    assert!(!cout.windows(8).any(|w| w == b"Variable"));
}

#[test]
fn e5_execute_operation_null_func_null_name() {
    let (cv, cout) = capture(|| unsafe {
        c().execute_operation(None, 1, 2, std::ptr::null::<c_char>())
    });
    let (rv, rout) = capture(|| unsafe {
        r().execute_operation(None, 1, 2, std::ptr::null::<c_char>())
    });
    assert_eq!(cv, 0);
    assert_eq!(cv, rv, "return value mismatch [E5]");
    assert_eq!(
        cout,
        rout,
        "stdout mismatch [E5]\n  C   : {}\n  Rust: {}",
        common::show(&cout),
        common::show(&rout)
    );
    // glibc renders `%s` with NULL as "(null)".
    assert!(
        cout.windows(6).any(|w| w == b"(null)"),
        "expected glibc's (null) rendering, got {}",
        common::show(&cout)
    );

    let mut rng = Rng::new(SEED ^ 0xE5);
    for i in 0..iters(50) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff(&format!("E5 #{i} ({a},{b})"), move |l: &Lib| unsafe {
            l.execute_operation(None, a, b, std::ptr::null::<c_char>())
        });
    }
}

#[test]
fn e6_execute_operation_null_func_empty_name() {
    let name = cstr("");
    let np = name.as_ptr();
    let (cv, cout) = capture(|| unsafe { c().execute_operation(None, -7, 7, np) });
    let (rv, rout) = capture(|| unsafe { r().execute_operation(None, -7, 7, np) });
    assert_eq!(cv, 0);
    assert_eq!(cv, rv);
    assert_eq!(
        cout, b"Error: Operation function pointer is NULL for \n".to_vec(),
        "unexpected C diagnostic: {}",
        common::show(&cout)
    );
    assert_eq!(cout, rout);
}

#[test]
fn e7_execute_operation_with_null_from_get_operation() {
    // Feed the NULL produced by E1/E2 straight back into execute_operation.
    let name = cstr("BAD");
    let np = name.as_ptr();
    for opcode in [-1, 4, 99, i32::MIN, i32::MAX] {
        diff(&format!("E7 opcode {opcode}"), move |l: &Lib| {
            let f: OperationFunc = l.get_operation(opcode);
            assert!(f.is_none(), "{} get_operation({opcode}) should be NULL", l.name);
            unsafe { l.execute_operation(f, 11, 22, np) }
        });
    }
}

// ===========================================================================
// E8..E12 — compute_checksum rejection / clamping
// ===========================================================================

#[test]
fn e8_compute_checksum_null_values() {
    for count in [1i32, 2, 3, 4, 5, 1000, i32::MAX] {
        let (cv, cout) = capture(|| unsafe { c().compute_checksum(std::ptr::null_mut(), count) });
        let (rv, rout) = capture(|| unsafe { r().compute_checksum(std::ptr::null_mut(), count) });
        assert_eq!(cv, 0, "C must return 0 for NULL values (count {count})");
        assert_eq!(cv, rv, "mismatch [E8 count {count}]");
        assert!(cout.is_empty(), "C printed: {}", common::show(&cout));
        assert_eq!(cout, rout);
    }
}

#[test]
fn e9_compute_checksum_zero_count() {
    let (cv, cout) = capture(|| {
        let mut v: [c_int; 4] = [0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444];
        unsafe { c().compute_checksum(v.as_mut_ptr(), 0) }
    });
    let (rv, rout) = capture(|| {
        let mut v: [c_int; 4] = [0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444];
        unsafe { r().compute_checksum(v.as_mut_ptr(), 0) }
    });
    assert_eq!(cv, 0, "C must return 0 for count == 0");
    assert_eq!(cv, rv);
    assert!(cout.is_empty());
    assert_eq!(cout, rout);
}

#[test]
fn e10_compute_checksum_negative_count() {
    for count in [-1i32, -2, -4, -16, -1000, i32::MIN, i32::MIN + 1] {
        diff(&format!("E10 count {count}"), move |l: &Lib| {
            let mut v: [c_int; 4] = [-1, -1, -1, -1];
            unsafe { l.compute_checksum(v.as_mut_ptr(), count) }
        });
        let (cv, _) = capture(|| {
            let mut v: [c_int; 4] = [-1, -1, -1, -1];
            unsafe { c().compute_checksum(v.as_mut_ptr(), count) }
        });
        assert_eq!(cv, 0, "C must return 0 for negative count {count}");
    }
}

#[test]
fn e11_compute_checksum_null_and_nonpositive() {
    for count in [0i32, -1, -100, i32::MIN] {
        let (cv, cout) = capture(|| unsafe { c().compute_checksum(std::ptr::null_mut(), count) });
        let (rv, rout) = capture(|| unsafe { r().compute_checksum(std::ptr::null_mut(), count) });
        assert_eq!(cv, 0);
        assert_eq!(cv, rv, "mismatch [E11 count {count}]");
        assert_eq!(cout, rout);
        assert!(cout.is_empty());
    }
}

#[test]
fn e12_compute_checksum_oversized_count_clamps() {
    let mut rng = Rng::new(SEED ^ 0xE12);
    for i in 0..iters(400) {
        let vals: [c_int; 4] = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        for count in [5i32, 6, 8, 16, 17, 1024, i32::MAX, i32::MAX - 1] {
            diff(&format!("E12 #{i} count {count}"), move |l: &Lib| {
                let mut v = vals;
                unsafe { l.compute_checksum(v.as_mut_ptr(), count) }
            });
        }
        // Oversized count must be exactly equivalent to count == 4 (clamped),
        // and must not read past the 4th element.
        let (four, _) = capture(|| {
            let mut v = vals;
            unsafe { c().compute_checksum(v.as_mut_ptr(), 4) }
        });
        for count in [5i32, 99, i32::MAX] {
            let (over, _) = capture(|| {
                let mut v = vals;
                unsafe { c().compute_checksum(v.as_mut_ptr(), count) }
            });
            assert_eq!(four, over, "C clamp broken for count {count}");
            let (rover, _) = capture(|| {
                let mut v = vals;
                unsafe { r().compute_checksum(v.as_mut_ptr(), count) }
            });
            assert_eq!(four, rover, "Rust clamp broken for count {count}");
        }
    }
}

// ===========================================================================
// E13 — init_state with NULL state
// ===========================================================================

#[test]
fn e13_init_state_null_state() {
    for v in [0i32, 1, -1, i32::MIN, i32::MAX, 12345] {
        let (_, cout) = capture(|| unsafe { c().init_state(std::ptr::null_mut(), v) });
        let (_, rout) = capture(|| unsafe { r().init_state(std::ptr::null_mut(), v) });
        assert_eq!(
            cout, b"Error: state pointer is NULL in init_state\n".to_vec(),
            "unexpected C diagnostic: {}",
            common::show(&cout)
        );
        assert_eq!(
            cout,
            rout,
            "stdout mismatch [E13 v={v}]\n  C   : {}\n  Rust: {}",
            common::show(&cout),
            common::show(&rout)
        );
        // the success message must NOT appear
        assert!(!cout.windows(5).any(|w| w == b"State"));
    }
}

// ===========================================================================
// E14..E16 — apply_operation with NULL state / func
// ===========================================================================

#[test]
fn e14_apply_operation_null_state() {
    for opcode in 0..4i32 {
        let (_, cout) = capture(|| unsafe {
            c().apply_operation(std::ptr::null_mut(), 42, c().get_operation(opcode))
        });
        let (_, rout) = capture(|| unsafe {
            r().apply_operation(std::ptr::null_mut(), 42, r().get_operation(opcode))
        });
        assert_eq!(
            cout, b"Error: state pointer is NULL in apply_operation\n".to_vec(),
            "unexpected C diagnostic: {}",
            common::show(&cout)
        );
        assert_eq!(cout, rout, "stdout mismatch [E14 opcode {opcode}]");
    }
}

#[test]
fn e15_apply_operation_null_func() {
    let mut rng = Rng::new(SEED ^ 0xE15);
    for i in 0..iters(100) {
        let st0 = ComputeState {
            accumulator: rng.interesting_i32(),
            operation_count: rng.interesting_i32(),
            checksum: rng.next_u32(),
        };
        common::diff_bytes(&format!("E15 #{i}"), move |l: &Lib| {
            let mut st = st0;
            unsafe { l.apply_operation(&mut st, 999, None) };
            ((), st.bytes().to_vec())
        });
        // and the state really is untouched
        let (cst, cout) = capture(|| {
            let mut st = st0;
            unsafe { c().apply_operation(&mut st, 999, None) };
            st
        });
        assert_eq!(cst, st0, "C must leave the state untouched");
        assert_eq!(
            cout,
            b"Error: operation function pointer is NULL in apply_operation\n".to_vec(),
            "unexpected C diagnostic: {}",
            common::show(&cout)
        );
    }
}

#[test]
fn e16_apply_operation_null_state_and_func() {
    // Precedence: the `state == NULL` check runs first, so only that message.
    let (_, cout) = capture(|| unsafe { c().apply_operation(std::ptr::null_mut(), 7, None) });
    let (_, rout) = capture(|| unsafe { r().apply_operation(std::ptr::null_mut(), 7, None) });
    assert_eq!(
        cout, b"Error: state pointer is NULL in apply_operation\n".to_vec(),
        "unexpected C diagnostic: {}",
        common::show(&cout)
    );
    assert_eq!(cout, rout, "stdout mismatch [E16]");
    assert!(
        !cout.windows(9).any(|w| w == b"operation" ) || !cout.starts_with(b"Error: operation"),
        "the func message must not be emitted first"
    );
}

// ===========================================================================
// E17 — checkshift allocation failure: the `malloc() == NULL` branch, actually
// executed in BOTH libraries via an LD_PRELOAD allocator fault-injector.
// ===========================================================================

/// Env var that turns this test binary into the fault-injection child.
const ALLOC_FAIL_CHILD: &str = "CHECKSHIFT_ALLOC_FAIL_CHILD";

/// Build `tests/fixtures/failmalloc.c` into a preloadable `.so` (once).
fn build_failmalloc_shim() -> Option<std::path::PathBuf> {
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/failmalloc.c");
    let out = std::env::temp_dir().join("libfailmalloc.so");
    let st = std::process::Command::new("cc")
        .args(["-shared", "-fPIC", "-O0", "-o"])
        .arg(&out)
        .arg(src)
        .arg("-ldl")
        .status();
    match st {
        Ok(s) if s.success() && out.exists() => Some(out),
        _ => None,
    }
}

/// The child half: dlopen one library, arm the fault injector around the single
/// `checkshift` call, and print the transcript plus the return value.
fn alloc_fail_child(which: &str) -> ! {
    let path = if which == "C" {
        common::c_so_path()
    } else {
        common::rust_so_path()
    };

    // dlopen BEFORE arming, so loader allocations are untouched.
    let lib = unsafe { libloading::Library::new(&path) }.expect("dlopen target");
    let f: libloading::Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"checkshift\0") }.expect("dlsym checkshift");
    let checkshift = *f;

    // Force glibc to allocate the stdout buffer now, so no stdio allocation
    // happens inside the armed window. (`which` must be NUL-terminated before
    // it can be handed to `%s`.)
    let which_c = cstr(which);
    unsafe {
        common::printf(
            b"child %s ready\n\0".as_ptr() as *const c_char,
            which_c.as_ptr(),
        )
    };
    unsafe { libc_fflush_all() };

    // The arm/disarm hooks live in the LD_PRELOAD'ed shim, i.e. in this
    // process's own global symbol scope.
    let this = libloading::os::unix::Library::this();
    let arm: libloading::os::unix::Symbol<unsafe extern "C" fn(usize)> =
        unsafe { this.get(b"arm_fail_malloc\0") }.expect("arm_fail_malloc not preloaded");
    let disarm: libloading::os::unix::Symbol<unsafe extern "C" fn()> =
        unsafe { this.get(b"disarm_fail_malloc\0") }.expect("disarm_fail_malloc not preloaded");

    // sizeof(ComputeState) == 12
    unsafe { arm(std::mem::size_of::<ComputeState>()) };
    let ret = unsafe { checkshift(1, 2, 3, 4) };
    unsafe { disarm() };

    unsafe { common::printf(b"RET=%d\n\0".as_ptr() as *const c_char, ret) };
    unsafe { libc_fflush_all() };
    std::process::exit(0);
}

extern "C" {
    #[link_name = "fflush"]
    fn fflush_raw(s: *mut std::ffi::c_void) -> c_int;
}
unsafe fn libc_fflush_all() {
    fflush_raw(std::ptr::null_mut());
}

/// Spawn this test binary again, under LD_PRELOAD, to run `alloc_fail_child`.
fn run_alloc_fail_child(shim: &std::path::Path, which: &str) -> (String, bool) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args([
            "--exact",
            "e17_checkshift_malloc_failure",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("LD_PRELOAD", shim)
        .env(ALLOC_FAIL_CHILD, which)
        .output()
        .expect("spawn child");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.success(),
    )
}

#[test]
fn e17_checkshift_malloc_failure() {
    // --- child mode -------------------------------------------------------
    if let Ok(which) = std::env::var(ALLOC_FAIL_CHILD) {
        alloc_fail_child(&which);
    }

    // --- parent mode ------------------------------------------------------
    let Some(shim) = build_failmalloc_shim() else {
        panic!(
            "could not build the LD_PRELOAD fault injector \
             (tests/fixtures/failmalloc.c) - E17 cannot be verified"
        );
    };

    let (c_out, c_ok) = run_alloc_fail_child(&shim, "C");
    let (r_out, r_ok) = run_alloc_fail_child(&shim, "Rust");
    assert!(c_ok, "C child failed:\n{c_out}");
    assert!(r_ok, "Rust child failed:\n{r_out}");

    // Keep only the library transcript: from the banner through RET=.
    let extract = |s: &str| -> String {
        let start = s.find("=== Starting foo function ===").or_else(|| s.find("Error: Failed"));
        let end = s.find("RET=").map(|i| {
            s[i..].find('\n').map(|j| i + j + 1).unwrap_or(s.len())
        });
        match (start, end) {
            (Some(a), Some(b)) if b > a => s[a..b].to_string(),
            _ => {
                // allocation failed before the banner flushed; take from RET back
                let i = s.find("Parameters:").unwrap_or(0);
                let j = s.find("RET=").map(|k| {
                    s[k..].find('\n').map(|m| k + m + 1).unwrap_or(s.len())
                }).unwrap_or(s.len());
                s[i..j].to_string()
            }
        }
    };
    let ct = extract(&c_out);
    let rt = extract(&r_out);

    // The branch really was taken, with the exact C diagnostic and sentinel.
    assert!(
        ct.contains("Error: Failed to allocate memory for state"),
        "the C allocation-failure branch was NOT exercised; transcript:\n{c_out}"
    );
    assert!(
        ct.contains("RET=-1"),
        "C must return the -1 sentinel; transcript:\n{c_out}"
    );
    // ... and it must NOT have continued into the normal pipeline
    assert!(
        !ct.contains("State initialized with accumulator"),
        "C should have bailed out before init_state; transcript:\n{c_out}"
    );

    assert_eq!(
        ct, rt,
        "E17 allocation-failure transcript differs\n  C   :\n{ct}\n  Rust:\n{rt}"
    );

    // Sanity: without arming, neither library ever takes that branch.
    let (_, out) = capture(|| {
        c().checkshift(1, 2, 3, 4);
        r().checkshift(1, 2, 3, 4);
    });
    assert!(!out
        .windows(42)
        .any(|w| w == b"Error: Failed to allocate memory for state"));

    // And on a healthy allocator both agree on every input, including the ones
    // that legitimately return -1 (so -1 is never ambiguous between the two).
    let mut rng = Rng::new(SEED ^ 0xE17);
    for _ in 0..iters(2000) {
        let (a, b, cc, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let (cv, cout) = capture(|| c().checkshift(a, b, cc, d));
        let (rv, rout) = capture(|| r().checkshift(a, b, cc, d));
        assert_eq!(cv, rv, "E17 checkshift({a},{b},{cc},{d})");
        assert_eq!(cout, rout, "E17 stdout checkshift({a},{b},{cc},{d})");
    }
}

// ===========================================================================
// Generic FFI boundaries (ERRORS.md G1..G7) not tied to a single row
// ===========================================================================

#[test]
fn g_null_pointers_in_every_pointer_parameter() {
    // execute_operation: func NULL, name NULL, both NULL
    let name = cstr("N");
    let np = name.as_ptr();
    let f0 = c().get_operation(0);
    let rf0 = r().get_operation(0);
    // valid func + NULL name -> `%s` with NULL on the *success* path
    let (cv, cout) = capture(|| unsafe { c().execute_operation(f0, 3, 4, std::ptr::null()) });
    let (rv, rout) = capture(|| unsafe { r().execute_operation(rf0, 3, 4, std::ptr::null()) });
    assert_eq!(cv, rv, "valid func + NULL name: value");
    assert_eq!(
        cout,
        rout,
        "valid func + NULL name: stdout\n  C   : {}\n  Rust: {}",
        common::show(&cout),
        common::show(&rout)
    );

    // NULL func + valid name already covered (E4); NULL func + NULL name (E5).
    let (cv, cout) = capture(|| unsafe { c().execute_operation(None, 3, 4, np) });
    let (rv, rout) = capture(|| unsafe { r().execute_operation(None, 3, 4, np) });
    assert_eq!((cv, cout), (rv, rout));

    // compute_checksum NULL (E8), init_state NULL (E13), apply_operation NULL (E14/15/16)
    let (_, cout) = capture(|| unsafe { c().init_state(std::ptr::null_mut(), 0) });
    let (_, rout) = capture(|| unsafe { r().init_state(std::ptr::null_mut(), 0) });
    assert_eq!(cout, rout);
}

#[test]
fn g_extreme_scalars_every_entry_point() {
    for &a in EDGES {
        for &b in EDGES {
            diff(&format!("G7 mult({a},{b})"), move |l: &Lib| {
                l.multiply_with_static(a, b)
            });
            diff(&format!("G7 add({a},{b})"), move |l: &Lib| {
                l.add_with_static(a, b)
            });
            diff(&format!("G7 xor({a},{b})"), move |l: &Lib| {
                l.xor_operation(a, b)
            });
            diff(&format!("G7 shift({a},{b})"), move |l: &Lib| {
                l.shift_with_static(a, b)
            });
        }
    }
    for &v in EDGES {
        diff(&format!("G7 checkshift({v},{v},{v},{v})"), move |l: &Lib| {
            l.checkshift(v, v, v, v)
        });
    }
}
