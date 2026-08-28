// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md. Rows 1 and 2 are the two `goto cleanup`
// branches of `cleanup`; neither is reachable from the public arguments, so they
// are forced by interposing the libc calls both `.so`s make (`strncmp` and
// `malloc`) with tests/support/fault_shim.c under LD_PRELOAD. Because the shim
// must be preloaded before the process starts, those two rows re-exec this test
// binary as a child and the assertions run inside the child.

#[path = "common/mod.rs"]
mod common;

use common::*;
use core::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::process::Command;

fn nul(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

const CHILD_ENV: &str = "HARVEST_FAULT_CHILD";

// ---------------------------------------------------------------------------
// Fault-shim plumbing
// ---------------------------------------------------------------------------

fn profile_dir() -> PathBuf {
    std::env::current_exe()
        .expect("current_exe")
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

/// Compiles tests/support/fault_shim.c next to the test binary (once).
fn build_fault_shim() -> PathBuf {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/fault_shim.c");
    let out = profile_dir().join("harvest_fault_shim.so");
    let rebuild = match (std::fs::metadata(&out), std::fs::metadata(&src)) {
        (Ok(o), Ok(s)) => match (o.modified(), s.modified()) {
            (Ok(om), Ok(sm)) => om < sm,
            _ => true,
        },
        _ => true,
    };
    if rebuild {
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let st = Command::new(&cc)
            .args(["-shared", "-fPIC", "-O2", "-o"])
            .arg(&out)
            .arg(&src)
            .arg("-ldl")
            .status()
            .unwrap_or_else(|e| panic!("cannot run {cc}: {e}"));
        assert!(st.success(), "compiling {} failed", src.display());
    }
    out
}

struct Shim {
    arm: unsafe extern "C" fn(c_int, c_int),
    malloc50_hits: unsafe extern "C" fn() -> u64,
    strncmp_hits: unsafe extern "C" fn() -> u64,
    free_hits: unsafe extern "C" fn() -> u64,
    free50_hits: unsafe extern "C" fn() -> u64,
    reset: unsafe extern "C" fn(),
    _lib: libloading::Library,
}

impl Shim {
    /// Grabs the control surface of the already-preloaded shim. `dlopen` on an
    /// object that is already in the link map just returns the same handle.
    fn open() -> Shim {
        let path = std::env::var("HARVEST_FAULT_SHIM").expect("HARVEST_FAULT_SHIM not set");
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({path}): {e}"));
        unsafe {
            let present = *lib
                .get::<unsafe extern "C" fn() -> c_int>(b"harvest_shim_present\0")
                .expect("harvest_shim_present");
            assert_eq!(present(), 1);
            Shim {
                arm: *lib.get(b"harvest_shim_arm\0").unwrap(),
                malloc50_hits: *lib.get(b"harvest_shim_malloc50_hits\0").unwrap(),
                strncmp_hits: *lib.get(b"harvest_shim_strncmp_hits\0").unwrap(),
                free_hits: *lib.get(b"harvest_shim_free_hits\0").unwrap(),
                free50_hits: *lib.get(b"harvest_shim_free50_hits\0").unwrap(),
                reset: *lib.get(b"harvest_shim_reset_counters\0").unwrap(),
                _lib: lib,
            }
        }
    }
}

fn in_child() -> bool {
    std::env::var(CHILD_ENV).is_ok()
}

/// Re-runs this very test binary, with the fault shim preloaded, restricted to
/// `child_test`. The child performs the differential assertions itself.
fn run_in_child(child_test: &str) {
    if in_child() {
        return; // never recurse
    }
    let shim = build_fault_shim();
    let exe = std::env::current_exe().expect("current_exe");
    let mut ld_preload = shim.display().to_string();
    if let Ok(existing) = std::env::var("LD_PRELOAD") {
        if !existing.is_empty() {
            ld_preload = format!("{ld_preload}:{existing}");
        }
    }
    let out = Command::new(&exe)
        .arg(child_test)
        .arg("--exact")
        .env(CHILD_ENV, "1")
        .env("LD_PRELOAD", &ld_preload)
        .env("HARVEST_FAULT_SHIM", shim.display().to_string())
        .env("HARVEST_C_SO", c_so_path())
        .env("HARVEST_RUST_SO", rust_so_path())
        .output()
        .unwrap_or_else(|e| panic!("spawning child {}: {e}", exe.display()));
    if !out.status.success() {
        eprintln!("--- child stdout ---\n{}", String::from_utf8_lossy(&out.stdout));
        eprintln!("--- child stderr ---\n{}", String::from_utf8_lossy(&out.stderr));
        panic!("child test `{child_test}` failed with {:?}", out.status);
    }
    // Surface the child's report so the parent log shows what really ran.
    eprint!("[child ok] ");
}

/// Make sure glibc has already allocated the stdout buffer (a 4 KiB request, so
/// it is unaffected by the 50-byte fault) before any fault is armed.
fn warm_up_stdio(cap: &mut Capture) {
    let l = nul("warmup");
    unsafe { (c().print_result)(l.as_ptr() as *const c_char, 0) };
    unsafe { (rs().print_result)(l.as_ptr() as *const c_char, 0) };
    let _ = cap.take();
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — input-string validation failure (`strncmp` != 0)
// ---------------------------------------------------------------------------

fn row01_strncmp_validation_failure() {
    run_in_child("child_row01_strncmp_fail");
}

fn child_row01_strncmp_fail() {
    if !in_child() {
        eprint!("[skipped: driven by row01_strncmp_validation_failure] ");
        return;
    }
    let shim = Shim::open();
    preload_both();
    let mut cap = Capture::new("c01");
    warm_up_stdio(&mut cap);

    for (a, b, c0, d) in [
        (1, 2, 3, 4),
        (10, 20, 30, 40),
        (0, 0, 0, 0),
        (i32::MAX, i32::MIN, 10, 30),
    ] {
        let _ = cap.take();

        unsafe { (shim.reset)() };
        unsafe { (shim.arm)(0, 1) };
        let rc_c = unsafe { (c().cleanup)(a, b, c0, d) };
        unsafe { (shim.arm)(0, 0) };
        let hits_c = (unsafe { (shim.strncmp_hits)() }, unsafe { (shim.malloc50_hits)() });
        let out_c = cap.take();

        unsafe { (shim.reset)() };
        unsafe { (shim.arm)(0, 1) };
        let rc_r = unsafe { (rs().cleanup)(a, b, c0, d) };
        unsafe { (shim.arm)(0, 0) };
        let hits_r = (unsafe { (shim.strncmp_hits)() }, unsafe { (shim.malloc50_hits)() });
        let out_r = cap.take();

        assert_eq!(
            rc_c, rc_r,
            "validation-failure return mismatch for ({a},{b},{c0},{d}): C={rc_c} Rust={rc_r}"
        );
        assert_eq!(
            rc_c, 0,
            "the C returns the pre-switch value of `result` (0) on validation failure"
        );
        assert_eq!(
            out_c,
            out_r,
            "validation-failure stdout mismatch:\n  C   = \"{}\"\n  Rust= \"{}\"",
            show(&out_c),
            show(&out_r)
        );
        assert_eq!(
            out_c,
            b"Input string validation failed.\n",
            "unexpected message: \"{}\"",
            show(&out_c)
        );
        assert_eq!(hits_c.0, 1, "C must call strncmp(\"VALID\",\"VALID\",5) once");
        assert_eq!(hits_r.0, 1, "Rust must call strncmp(\"VALID\",\"VALID\",5) once");
        assert_eq!(hits_c.1, 0, "C must not reach malloc(50) after the goto");
        assert_eq!(hits_r.1, 0, "Rust must not reach malloc(50) after the goto");
    }

    // With the fault disarmed both libraries must be back on the happy path.
    let (rc, out) = diff_cleanup_out(&mut cap, 10, 20, 30, 40);
    assert_eq!(rc, 160, "10=>+30, 20=>+20, 30=>+70, 40=>+40");
    assert_eq!(out, EXPECTED_CLEANUP_STDOUT);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — malloc(50) returns NULL
// ---------------------------------------------------------------------------

fn row02_malloc_failure() {
    run_in_child("child_row02_malloc_fail");
}

fn child_row02_malloc_fail() {
    if !in_child() {
        eprint!("[skipped: driven by row02_malloc_failure] ");
        return;
    }
    let shim = Shim::open();
    preload_both();
    let mut cap = Capture::new("c02");
    warm_up_stdio(&mut cap);

    // The switch loop runs *before* the allocation, so the returned value is the
    // fully accumulated `result` even though the function failed.
    for (a, b, c0, d, expect) in [
        (1, 2, 3, 4, 10),
        (10, 20, 30, 40, 160),
        (0, 0, 0, 0, 0),
        (10, 10, 10, 10, 120),
        (30, 30, 30, 30, 280),
        (i32::MAX, i32::MAX, 0, 0, -2),
    ] {
        let _ = cap.take();

        unsafe { (shim.reset)() };
        unsafe { (shim.arm)(1, 0) };
        let rc_c = unsafe { (c().cleanup)(a, b, c0, d) };
        unsafe { (shim.arm)(0, 0) };
        let hits_c = unsafe { (shim.malloc50_hits)() };
        let out_c = cap.take();

        unsafe { (shim.reset)() };
        unsafe { (shim.arm)(1, 0) };
        let rc_r = unsafe { (rs().cleanup)(a, b, c0, d) };
        unsafe { (shim.arm)(0, 0) };
        let hits_r = unsafe { (shim.malloc50_hits)() };
        let out_r = cap.take();

        assert_eq!(
            rc_c, rc_r,
            "malloc-failure return mismatch for ({a},{b},{c0},{d}): C={rc_c} Rust={rc_r}"
        );
        assert_eq!(rc_c, expect, "malloc-failure return for ({a},{b},{c0},{d})");
        assert_eq!(
            out_c,
            out_r,
            "malloc-failure stdout mismatch:\n  C   = \"{}\"\n  Rust= \"{}\"",
            show(&out_c),
            show(&out_r)
        );
        assert_eq!(
            out_c,
            b"Memory allocation failed.\n",
            "unexpected message: \"{}\"",
            show(&out_c)
        );
        assert_eq!(hits_c, 1, "C must request exactly one 50-byte block");
        assert_eq!(hits_r, 1, "Rust must request exactly one 50-byte block");
    }

    // CONFIGS.md row 27: state after the failure path must be identical.
    for (a, b, c0, d) in [(10, 20, 30, 40), (1, 2, 3, 4), (40, 40, 40, 40)] {
        let (rc, out) = diff_cleanup_out(&mut cap, a, b, c0, d);
        assert_eq!(rc, model_cleanup(a, b, c0, d));
        assert_eq!(out, EXPECTED_CLEANUP_STDOUT);
    }
    let l = nul("after-failure");
    let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, -7, "after-failure");
    assert_eq!(out, b"after-failure: -7\n");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — cleanup_resources(NULL): the explicit null check
// ---------------------------------------------------------------------------

fn row03_cleanup_resources_null() {
    let mut cap = Capture::new("c03");
    let _ = cap.take();
    for _ in 0..64 {
        unsafe {
            (c().cleanup_resources)(std::ptr::null_mut());
            let out_c = cap.take();
            (rs().cleanup_resources)(std::ptr::null_mut());
            let out_r = cap.take();
            assert_eq!(out_c, out_r, "cleanup_resources(NULL) stdout mismatch");
            assert!(out_c.is_empty(), "cleanup_resources(NULL) must print nothing");
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — cleanup_resources on a live block (incl. malloc(0))
// ---------------------------------------------------------------------------

fn row04_cleanup_resources_frees() {
    let mut cap = Capture::new("c04");
    for size in [0usize, 1, 49, 50, 51, 4096] {
        diff_cleanup_resources(&mut cap, size);
    }
    // The `dynamic_str = NULL` store inside the C touches only the local
    // parameter copy, so the value the caller passed is untouched.
    let _ = cap.take();
    unsafe {
        let p = malloc(50) as *mut c_char;
        let before = p;
        (c().cleanup_resources)(p);
        assert_eq!(before, p, "caller's pointer variable must be unchanged");
        let q = malloc(50) as *mut c_char;
        let before_q = q;
        (rs().cleanup_resources)(q);
        assert_eq!(before_q, q, "caller's pointer variable must be unchanged");
    }
    assert!(cap.take().is_empty());
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 3+4 (strengthened) — the release side of `cleanup_resources`
// is otherwise unobservable: a translation that simply *did not free* would pass
// every output comparison. The shim's allocation bookkeeping closes that hole.
// ---------------------------------------------------------------------------

fn row0304_free_accounting() {
    run_in_child("child_row0304_free_accounting");
}

fn child_row0304_free_accounting() {
    if !in_child() {
        eprint!("[skipped: driven by row0304_free_accounting] ");
        return;
    }
    let shim = Shim::open();
    preload_both();
    let mut cap = Capture::new("c0304");
    warm_up_stdio(&mut cap);

    // (a) cleanup_resources(NULL) must not free anything, in either library.
    for imp in [c(), rs()] {
        unsafe { (shim.reset)() };
        unsafe { (imp.cleanup_resources)(std::ptr::null_mut()) };
        let (f, f50) = unsafe { ((shim.free_hits)(), (shim.free50_hits)()) };
        assert_eq!(f, 0, "{}: cleanup_resources(NULL) must not call free", imp.name);
        assert_eq!(f50, 0, "{}: cleanup_resources(NULL) must not call free", imp.name);
    }

    // (b) cleanup_resources(p) must free exactly the block it was handed.
    for imp in [c(), rs()] {
        unsafe { (shim.reset)() };
        let p = unsafe { malloc(50) } as *mut c_char;
        assert!(!p.is_null());
        assert_eq!(
            unsafe { (shim.malloc50_hits)() },
            1,
            "the shim must have tracked the 50-byte block"
        );
        unsafe { (imp.cleanup_resources)(p) };
        let (f, f50) = unsafe { ((shim.free_hits)(), (shim.free50_hits)()) };
        assert_eq!(f, 1, "{}: cleanup_resources(p) must call free exactly once", imp.name);
        assert_eq!(
            f50, 1,
            "{}: the freed pointer must be the very block that was allocated",
            imp.name
        );
    }

    // (c) the happy path of `cleanup` allocates one 50-byte block and releases
    //     exactly that block before returning — no leak, no double free.
    for imp in [c(), rs()] {
        for (a, b, c0, d) in [(1, 2, 3, 4), (10, 20, 30, 40), (i32::MAX, 0, 0, 0)] {
            let _ = cap.take();
            unsafe { (shim.reset)() };
            let rc = unsafe { (imp.cleanup)(a, b, c0, d) };
            let (m50, f50) = unsafe { ((shim.malloc50_hits)(), (shim.free50_hits)()) };
            let out = cap.take();
            assert_eq!(rc, model_cleanup(a, b, c0, d), "{}", imp.name);
            assert_eq!(out, EXPECTED_CLEANUP_STDOUT, "{}", imp.name);
            assert_eq!(m50, 1, "{}: exactly one 50-byte allocation", imp.name);
            assert_eq!(
                f50, 1,
                "{}: the 50-byte buffer must be freed before returning (leak?)",
                imp.name
            );
        }
    }

    // (d) on the malloc-failure path nothing may be freed (the C hands NULL to
    //     cleanup_resources), for either library.
    for imp in [c(), rs()] {
        let _ = cap.take();
        unsafe { (shim.reset)() };
        unsafe { (shim.arm)(1, 0) };
        let rc = unsafe { (imp.cleanup)(10, 20, 30, 40) };
        unsafe { (shim.arm)(0, 0) };
        let (m50, f50, f) = unsafe {
            ((shim.malloc50_hits)(), (shim.free50_hits)(), (shim.free_hits)())
        };
        let out = cap.take();
        assert_eq!(rc, 160, "{}", imp.name);
        assert_eq!(out, b"Memory allocation failed.\n", "{}", imp.name);
        assert_eq!(m50, 1, "{}: the allocation was attempted", imp.name);
        assert_eq!(f50, 0, "{}: nothing to free on the failure path", imp.name);
        assert_eq!(f, 0, "{}: free must not be called with a bogus pointer", imp.name);
    }

    // (e) on the validation-failure path likewise nothing is allocated or freed.
    for imp in [c(), rs()] {
        let _ = cap.take();
        unsafe { (shim.reset)() };
        unsafe { (shim.arm)(0, 1) };
        let rc = unsafe { (imp.cleanup)(10, 20, 30, 40) };
        unsafe { (shim.arm)(0, 0) };
        let (m50, f) = unsafe { ((shim.malloc50_hits)(), (shim.free_hits)()) };
        let out = cap.take();
        assert_eq!(rc, 0, "{}", imp.name);
        assert_eq!(out, b"Input string validation failed.\n", "{}", imp.name);
        assert_eq!(m50, 0, "{}: no allocation before the goto", imp.name);
        assert_eq!(f, 0, "{}: no free before the goto", imp.name);
    }

    // (f) many back-to-back calls: allocations and frees must stay balanced.
    let _ = cap.take();
    unsafe { (shim.reset)() };
    for _ in 0..200 {
        unsafe { (c().cleanup)(10, 1, 30, 2) };
        unsafe { (rs().cleanup)(10, 1, 30, 2) };
    }
    let (m50, f50) = unsafe { ((shim.malloc50_hits)(), (shim.free50_hits)()) };
    let _ = cap.take();
    assert_eq!(m50, 400, "400 calls => 400 fifty-byte allocations");
    assert_eq!(f50, 400, "every one of them must be freed");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 — values with no matching `case` label (the out-of-range
// "variant" class for this API's implicit {10,20,30,40} enum)
// ---------------------------------------------------------------------------

fn row05_default_arm_non_case_values() {
    let mut cap = Capture::new("c05");
    let mut vals: Vec<i32> = Vec::new();
    vals.extend_from_slice(&NEAR_CASE);
    vals.extend_from_slice(&NEGATED_CASE);
    vals.extend_from_slice(&EXTREMES);
    vals.extend_from_slice(&[2, 5, 15, 25, 35, 45, 50, 100, 400, -100, 1 << 20, -(1 << 20)]);
    for &v in &vals {
        // Alone in each slot, so the default arm is the only contributor.
        for slot in 0..4 {
            let mut args = [0i32; 4];
            args[slot] = v;
            let rc = diff_cleanup(&mut cap, args[0], args[1], args[2], args[3]);
            assert_eq!(rc, v, "value {v} must fall through to `default` (slot {slot})");
        }
    }
    // Pairwise, to catch any interaction with the fall-through arms.
    for &v in &vals {
        for &l in &CASE_LABELS {
            let rc = diff_cleanup(&mut cap, v, l, 0, 0);
            assert_eq!(rc, model_cleanup(v, l, 0, 0));
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 — `case 10:` has no break and falls into `case 20:`
// ---------------------------------------------------------------------------

fn row06_fallthrough_case_10() {
    let mut cap = Capture::new("c06");
    assert_eq!(diff_cleanup(&mut cap, 10, 0, 0, 0), 30, "10 => +10 then +20");
    assert_ne!(diff_cleanup(&mut cap, 10, 0, 0, 0), 10, "must NOT be a plain +10");
    assert_eq!(diff_cleanup(&mut cap, 10, 10, 0, 0), 60);
    assert_eq!(diff_cleanup(&mut cap, 10, 10, 10, 0), 90);
    assert_eq!(diff_cleanup(&mut cap, 10, 10, 10, 10), 120);
    // 10 next to its neighbours 9 and 11 (which take `default`).
    assert_eq!(diff_cleanup(&mut cap, 9, 10, 11, 0), 9 + 30 + 11);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 7 — `case 30:` has no break and falls into `case 40:`
// ---------------------------------------------------------------------------

fn row07_fallthrough_case_30() {
    let mut cap = Capture::new("c07");
    assert_eq!(diff_cleanup(&mut cap, 30, 0, 0, 0), 70, "30 => +30 then +40");
    assert_ne!(diff_cleanup(&mut cap, 30, 0, 0, 0), 30, "must NOT be a plain +30");
    assert_eq!(diff_cleanup(&mut cap, 30, 30, 0, 0), 140);
    assert_eq!(diff_cleanup(&mut cap, 30, 30, 30, 0), 210);
    assert_eq!(diff_cleanup(&mut cap, 30, 30, 30, 30), 280);
    assert_eq!(diff_cleanup(&mut cap, 29, 30, 31, 0), 29 + 70 + 31);
    // `case 20:` and `case 40:` do break — they must not accumulate further.
    assert_eq!(diff_cleanup(&mut cap, 20, 0, 0, 0), 20);
    assert_eq!(diff_cleanup(&mut cap, 40, 0, 0, 0), 40);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 8 — signed overflow of the accumulator
// ---------------------------------------------------------------------------

fn row08_result_overflow_wraps() {
    let mut cap = Capture::new("c08");
    let cases: [(i32, i32, i32, i32, i32); 10] = [
        (i32::MAX, i32::MAX, 0, 0, -2),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX, -4),
        (i32::MIN, i32::MIN, 0, 0, 0),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN, 0),
        (i32::MAX, 1, 0, 0, i32::MIN),
        (i32::MIN, -1, 0, 0, i32::MAX),
        (i32::MAX, 10, 0, 0, i32::MIN + 29),
        (i32::MAX, 30, 0, 0, i32::MIN + 69),
        (i32::MIN, 40, 0, 0, i32::MIN + 40),
        (i32::MAX, i32::MIN, 0, 0, -1),
    ];
    for (a, b, c0, d, expect) in cases {
        let rc = diff_cleanup(&mut cap, a, b, c0, d);
        assert_eq!(rc, expect, "overflow shape ({a},{b},{c0},{d})");
    }
    // Randomised overflow-heavy sweep.
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..3000 {
        let pick = |r: &mut Rng| match r.below(3) {
            0 => i32::MAX - r.range_i32(0, 64),
            1 => i32::MIN + r.range_i32(0, 64),
            _ => r.next_i32(),
        };
        let (a, b, c0, d) = (pick(&mut rng), pick(&mut rng), pick(&mut rng), pick(&mut rng));
        let rc = diff_cleanup(&mut cap, a, b, c0, d);
        assert_eq!(rc, model_cleanup(a, b, c0, d), "({a},{b},{c0},{d})");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 9 — the snprintf bound of 50 and the stringised argument
// ---------------------------------------------------------------------------

fn row09_snprintf_bound() {
    let mut cap = Capture::new("c09");
    // "Processed numbers: numbers" is 26 bytes + NUL == 27 <= 50, so nothing is
    // ever truncated, for any input, because the text is input independent.
    assert_eq!(EXPECTED_CLEANUP_STDOUT.len(), 27);
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..1500 {
        let (a, b, c0, d) = if i < 4 {
            (CASE_LABELS[i], CASE_LABELS[i], CASE_LABELS[i], CASE_LABELS[i])
        } else {
            (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32())
        };
        let (_, out) = diff_cleanup_out(&mut cap, a, b, c0, d);
        assert_eq!(
            out,
            EXPECTED_CLEANUP_STDOUT,
            "stdout must be input-independent, got \"{}\" for ({a},{b},{c0},{d})",
            show(&out)
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 10 — print_result(NULL, n): no null check in the C
// ---------------------------------------------------------------------------

fn row10_print_result_null_label() {
    let mut cap = Capture::new("c10");
    for &n in &[0i32, 1, -1, i32::MAX, i32::MIN, 12345] {
        let out = diff_print_result(&mut cap, std::ptr::null(), n, "NULL");
        assert_eq!(
            out,
            format!("(null): {n}\n").into_bytes(),
            "glibc printf renders a NULL %s as (null)"
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 11 — int boundaries for the %d argument
// ---------------------------------------------------------------------------

fn row11_print_result_int_bounds() {
    let mut cap = Capture::new("c11");
    let l = nul("n");
    for &n in &[
        0i32,
        -1,
        1,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        -2147483647,
    ] {
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, n, "n");
        assert_eq!(out, format!("n: {n}\n").into_bytes());
    }
    // And the values `cleanup` itself can return, fed straight through.
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..500 {
        let n = model_cleanup(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, n, "n");
        assert_eq!(out, format!("n: {n}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 12 — conversion specifiers inside the label
// ---------------------------------------------------------------------------

fn row12_print_result_percent_in_label() {
    let mut cap = Capture::new("c12");
    for s in [
        "%s", "%d", "%n", "%%", "%", "%%%%", "%1000000d", "%99999999s", "%p", "%hhn",
        "%s: %d\n", "AAAA%n%n%n%n",
    ] {
        let l = nul(s);
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, 5, s);
        assert_eq!(
            out,
            format!("{s}: 5\n").into_bytes(),
            "the label is an argument, never a format string"
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 13 — zero-length label
// ---------------------------------------------------------------------------

fn row13_print_result_empty_label() {
    let mut cap = Capture::new("c13");
    let l = nul("");
    for &n in &[0i32, -1, i32::MAX, i32::MIN] {
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, n, "\"\"");
        assert_eq!(out, format!(": {n}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 14 — oversized and non-UTF-8 labels
// ---------------------------------------------------------------------------

fn row14_print_result_oversized_and_non_utf8() {
    let mut cap = Capture::new("c14");
    for len in [65536usize, 1 << 20] {
        let mut l = vec![b'Z'; len];
        l.push(0);
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, -1, "<oversized>");
        assert_eq!(out.len(), len + 5);
        assert_eq!(&out[len..], b": -1\n");
    }
    // Non-UTF-8 payload, including a long run of high bytes.
    let mut l: Vec<u8> = (0..4096).map(|i| 0x80u8.wrapping_add((i % 128) as u8) | 0x80).collect();
    l.push(0);
    let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, 0, "<non-utf8>");
    assert_eq!(out.len(), 4096 + 4);
    // Embedded newlines interact with stdio buffering; bytes must still match.
    let mut l2: Vec<u8> = Vec::new();
    for _ in 0..1000 {
        l2.extend_from_slice(b"line\n");
    }
    l2.push(0);
    let out2 = diff_print_result(&mut cap, l2.as_ptr() as *const c_char, 1, "<newlines>");
    assert_eq!(out2.len(), 5000 + 4);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 15 — exactly the four arguments contribute, each once
// ---------------------------------------------------------------------------

fn row15_exactly_four_args_contribute() {
    let mut cap = Capture::new("c15");
    // Distinct powers of two: the returned sum identifies exactly which slots
    // contributed and how many times.
    let rc = diff_cleanup(&mut cap, 1, 2, 4, 8);
    assert_eq!(rc, 15, "each of the four arguments contributes exactly once");
    let rc = diff_cleanup(&mut cap, 1 << 4, 1 << 8, 1 << 12, 1 << 16);
    assert_eq!(rc, (1 << 4) + (1 << 8) + (1 << 12) + (1 << 16));
    // A fifth value cannot exist; permuting the four must not change the sum.
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..1000 {
        let a = rng.range_i32(-1000, 1000);
        let b = rng.range_i32(-1000, 1000);
        let c0 = rng.range_i32(-1000, 1000);
        let d = rng.range_i32(-1000, 1000);
        let base = diff_cleanup(&mut cap, a, b, c0, d);
        assert_eq!(diff_cleanup(&mut cap, d, c0, b, a), base, "order independence");
        assert_eq!(diff_cleanup(&mut cap, b, a, d, c0), base, "order independence");
    }
}

// ---------------------------------------------------------------------------

fn main() {
    common::run_tests(&[
        ("row01_strncmp_validation_failure", row01_strncmp_validation_failure),
        ("child_row01_strncmp_fail", child_row01_strncmp_fail),
        ("row02_malloc_failure", row02_malloc_failure),
        ("child_row02_malloc_fail", child_row02_malloc_fail),
        ("row03_cleanup_resources_null", row03_cleanup_resources_null),
        ("row04_cleanup_resources_frees", row04_cleanup_resources_frees),
        ("row0304_free_accounting", row0304_free_accounting),
        ("child_row0304_free_accounting", child_row0304_free_accounting),
        ("row05_default_arm_non_case_values", row05_default_arm_non_case_values),
        ("row06_fallthrough_case_10", row06_fallthrough_case_10),
        ("row07_fallthrough_case_30", row07_fallthrough_case_30),
        ("row08_result_overflow_wraps", row08_result_overflow_wraps),
        ("row09_snprintf_bound", row09_snprintf_bound),
        ("row10_print_result_null_label", row10_print_result_null_label),
        ("row11_print_result_int_bounds", row11_print_result_int_bounds),
        ("row12_print_result_percent_in_label", row12_print_result_percent_in_label),
        ("row13_print_result_empty_label", row13_print_result_empty_label),
        (
            "row14_print_result_oversized_and_non_utf8",
            row14_print_result_oversized_and_non_utf8,
        ),
        ("row15_exactly_four_args_contribute", row15_exactly_four_args_contribute),
    ]);
}
