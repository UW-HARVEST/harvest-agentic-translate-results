// Phase B -- valid-path differential tests for `complexmode` (the one-shot
// wrapper) and for composed pipelines. CONFIGS.md rows C25..C35.

mod common;
use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

fn cm(mode: i32, v1: i32, v2: i32, v3: i32, what: &str) -> (c_int, Vec<u8>) {
    let (c, r) = each::<FnComplexmode, _>("complexmode", |f| {
        capture(|| unsafe { f(mode, v1, v2, v3) })
    });
    let out = c.clone();
    same(what, c, r);
    out
}

// ===========================================================================
// C25..C26 -- mode 1 (addition; permissions 0644 satisfies READ|WRITE = 0600)
// ===========================================================================

#[test]
fn c25_complexmode_mode1_random() {
    let mut rng = Rng::with_seed(25);
    for _ in 0..ITERS * 4 {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        cm(1, a, b, c, &format!("C25 complexmode(1,{a},{b},{c})"));
    }
    // non-vacuous: the C library really does emit the 3-line mode-1 transcript
    let (rv, out) = cm(1, 20, 22, 999, "C25 sanity");
    assert_eq!(rv, 42);
    assert_eq!(
        out,
        b"Mode 1: Addition\nResult: 42\nOperation performed: addition\n"
    );
}

#[test]
fn c26_complexmode_mode1_boundaries() {
    for &a in BOUNDS {
        for &b in BOUNDS {
            cm(1, a, b, 0, &format!("C26 complexmode(1,{a},{b},0)"));
        }
    }
    for &(a, b) in &[
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX - 1, 2),
    ] {
        cm(1, a, b, i32::MIN, &format!("C26 complexmode(1,{a},{b},MIN)"));
    }
}

// ===========================================================================
// C27..C28 -- mode 2 (multiplication + malloc'ed log string)
// ===========================================================================

#[test]
fn c27_complexmode_mode2_random() {
    let mut rng = Rng::with_seed(27);
    for _ in 0..ITERS * 4 {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        cm(2, a, b, c, &format!("C27 complexmode(2,{a},{b},{c})"));
    }
    let (rv, out) = cm(2, 6, 7, -1, "C27 sanity");
    assert_eq!(rv, 42);
    assert_eq!(
        out,
        b"Mode 2: Operation: multiply, Value: 42\nOperation performed: multiplication\n"
    );
}

#[test]
fn c28_complexmode_mode2_boundaries() {
    for &a in BOUNDS {
        for &b in BOUNDS {
            cm(2, a, b, 0, &format!("C28 complexmode(2,{a},{b},0)"));
        }
    }
    // products needing the full width of %d, including INT_MIN (11 chars)
    for &(a, b) in &[
        (i32::MIN, 1),
        (1, i32::MIN),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MAX, 1),
        (46341, 46341),
        (-46341, 46341),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (65536, 32768),
    ] {
        cm(2, a, b, 0, &format!("C28 complexmode(2,{a},{b},0)"));
    }
}

// ===========================================================================
// C29..C30 -- mode 3 (array sum through copy_and_sum(values, 3))
// ===========================================================================

#[test]
fn c29_complexmode_mode3_random() {
    let mut rng = Rng::with_seed(29);
    for _ in 0..ITERS * 4 {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        cm(3, a, b, c, &format!("C29 complexmode(3,{a},{b},{c})"));
    }
    let (rv, out) = cm(3, 10, 15, 17, "C29 sanity");
    assert_eq!(rv, 42);
    assert_eq!(
        out,
        b"Mode 3: Array Sum\nResult: 42\nOperation performed: array_sum\n"
    );
}

#[test]
fn c30_complexmode_mode3_boundaries() {
    let ext = [i32::MAX, i32::MIN, 0, -1, 1, i32::MAX - 1, i32::MIN + 1];
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                cm(3, a, b, c, &format!("C30 complexmode(3,{a},{b},{c})"));
            }
        }
    }
    for &a in BOUNDS {
        cm(3, a, a, a, &format!("C30 complexmode(3,{a},{a},{a})"));
    }
    // A sum of exactly -1 makes mode 3 return the same value as the
    // copy_and_sum error sentinel -- confirm both libraries agree there too.
    cm(3, -1, 0, 0, "C30 sum == -1 (aliases error sentinel)");
    cm(3, i32::MAX, i32::MIN, 0, "C30 sum == -1 via wraparound");
}

// ===========================================================================
// C31..C32 -- mode 4 (permissions 0644 & 0100 == 0, so the ELSE branch)
// ===========================================================================

#[test]
fn c31_complexmode_mode4_random() {
    let mut rng = Rng::with_seed(31);
    for _ in 0..ITERS * 4 {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        cm(4, a, b, c, &format!("C31 complexmode(4,{a},{b},{c})"));
    }
    // The C takes the else-branch (v1+v2+v3), NOT (v1*v2)+v3: 2+3+4 = 9, not 10.
    let (rv, out) = cm(4, 2, 3, 4, "C31 sanity: else-branch");
    assert_eq!(rv, 9, "mode 4 must use v1+v2+v3 (EXEC_PERM is not set in 0644)");
    assert_eq!(
        out,
        b"Mode 4: Complex Calculation\nResult: 9\nOperation performed: complex\n"
    );
}

#[test]
fn c32_complexmode_mode4_boundaries() {
    let ext = [i32::MAX, i32::MIN, 0, -1, 1, i32::MAX - 1, i32::MIN + 1];
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                cm(4, a, b, c, &format!("C32 complexmode(4,{a},{b},{c})"));
            }
        }
    }
    for &a in BOUNDS {
        cm(4, a, a, a, &format!("C32 complexmode(4,{a},{a},{a})"));
    }
}

// ===========================================================================
// C33 -- default arm (also an ERRORS.md row, kept here as the valid-input
//        "unknown mode" configuration)
// ===========================================================================

#[test]
fn c33_complexmode_invalid_modes() {
    let mut rng = Rng::with_seed(33);
    let mut modes: Vec<i32> = vec![0, 5, 6, 7, -1, -2, 100, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    for _ in 0..ITERS {
        let m = rng.i32();
        if !(1..=4).contains(&m) {
            modes.push(m);
        }
    }
    for m in modes {
        let (rv, out) = cm(m, 1, 2, 3, &format!("C33 complexmode({m},1,2,3)"));
        assert_eq!(rv, -1, "mode {m} must return -1");
        // operation stays "none", so NO "Operation performed" trailer
        assert_eq!(out, b"Invalid mode\n", "mode {m} transcript");
    }
}

// ===========================================================================
// C34 -- all five arms in sequence in one process, comparing the concatenated
//        transcript (catches ordering / residual-state differences that
//        per-call tests cannot see)
// ===========================================================================

#[test]
fn c34_complexmode_full_sequence() {
    let mut rng = Rng::with_seed(34);
    let mut sets: Vec<(i32, i32, i32)> = vec![
        (2, 3, 4),
        (0, 0, 0),
        (i32::MAX, i32::MIN, 1),
        (-7, 11, -13),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
    ];
    for _ in 0..64 {
        sets.push((rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed()));
    }

    let l = libs();
    let cf: libloading::Symbol<FnComplexmode> = sym(&l.c, "complexmode");
    let rf: libloading::Symbol<FnComplexmode> = sym(&l.r, "complexmode");

    for (v1, v2, v3) in sets {
        let run = |f: &libloading::Symbol<FnComplexmode>| {
            capture(|| {
                let mut rvs = Vec::new();
                for mode in [1, 2, 3, 4, 0, 5, -1, 4, 3, 2, 1] {
                    rvs.push(unsafe { f(mode, v1, v2, v3) });
                }
                rvs
            })
        };
        let c = run(&cf);
        let r = run(&rf);
        same(&format!("C34 sequence({v1},{v2},{v3})"), c, r);
    }
}

// ===========================================================================
// C35 -- composed low-level pipeline: each function's output feeds the next.
// ===========================================================================

struct Api {
    crs: libloading::Symbol<'static, FnCreateResultString>,
    chk: libloading::Symbol<'static, FnCheckPermissions>,
    add: libloading::Symbol<'static, FnSafeAdd>,
    mul: libloading::Symbol<'static, FnMultiplyWithLog>,
    sum: libloading::Symbol<'static, FnCopyAndSum>,
    cmpo: libloading::Symbol<'static, FnCompareOperations>,
    cmode: libloading::Symbol<'static, FnComplexmode>,
}

fn api(lib: &'static libloading::Library) -> Api {
    Api {
        crs: sym(lib, "create_result_string"),
        chk: sym(lib, "check_permissions"),
        add: sym(lib, "safe_add"),
        mul: sym(lib, "multiply_with_log"),
        sum: sym(lib, "copy_and_sum"),
        cmpo: sym(lib, "compare_operations"),
        cmode: sym(lib, "complexmode"),
    }
}

fn libc_free(p: *mut std::os::raw::c_void) {
    unsafe extern "C" {
        fn free(p: *mut std::os::raw::c_void);
    }
    unsafe { free(p) }
}

/// One pipeline run: returns every intermediate observable, so a divergence
/// anywhere in the chain is caught (and stdout is compared for the whole run).
#[allow(clippy::type_complexity)]
fn pipeline(a: &Api, seed: u64) -> (Vec<i32>, Vec<Option<Vec<u8>>>, Vec<u8>) {
    let mut rng = Rng::with_seed(seed);
    capture(|| {
        let mut ints: Vec<i32> = Vec::new();
        let mut strs: Vec<Option<Vec<u8>>> = Vec::new();

        for _ in 0..24 {
            let perms = rng.i32_mixed();
            let required = if rng.next_u64() % 2 == 0 { 0o600 } else { rng.i32_mixed() };

            // 1. permission probe feeds the perms argument of safe_add
            let ok = unsafe { (a.chk)(perms, required) };
            ints.push(ok);

            // 2. safe_add, using a perms value derived from the probe
            let x = rng.i32_mixed();
            let y = rng.i32_mixed();
            let eff_perms = if ok != 0 { perms } else { perms & !0o200 };
            let s = unsafe { (a.add)(x, y, eff_perms) };
            ints.push(s);

            // 3. multiply the running sum by y, capturing the log string
            let mut log: *mut c_char = std::ptr::null_mut();
            let m = unsafe { (a.mul)(s, y, &mut log) };
            ints.push(m);
            let logbytes = unsafe { read_c_buf(log, 64) };
            strs.push(logbytes.clone());

            // 4. build an independent string for the same value and compare the
            //    two with compare_operations (output -> input chaining)
            let tag = CString::new(format!("op{}", m as i64 & 0xFFFF)).unwrap();
            let built = unsafe { (a.crs)(tag.as_ptr(), m) };
            strs.push(unsafe { read_c_buf(built, 64) });

            if !log.is_null() && !built.is_null() {
                ints.push(unsafe { (a.cmpo)(log, built) });
                ints.push(unsafe { (a.cmpo)(built, log) });
                ints.push(unsafe { (a.cmpo)(built, built) });
            }
            if !log.is_null() {
                libc_free(log as *mut _);
            }
            if !built.is_null() {
                libc_free(built as *mut _);
            }

            // 5. feed all the derived ints into copy_and_sum
            let mut buf: Vec<c_int> = vec![ok, s, m, x, y, perms, required];
            let n = buf.len() as c_int;
            ints.push(unsafe { (a.sum)(buf.as_mut_ptr(), n) });
            ints.push(unsafe { (a.sum)(buf.as_mut_ptr(), 0) });
            ints.push(unsafe { (a.sum)(buf.as_mut_ptr(), 1) });

            // 6. finally drive the one-shot wrapper with the derived values
            for mode in [1, 2, 3, 4, (m & 7)] {
                ints.push(unsafe { (a.cmode)(mode, s, m, x) });
            }
        }
        (ints, strs)
    })
    .into_iter_flat()
}

// helper to reshape ((Vec,Vec), Vec<u8>) -> (Vec, Vec, Vec<u8>)
trait Flat {
    #[allow(clippy::type_complexity)]
    fn into_iter_flat(self) -> (Vec<i32>, Vec<Option<Vec<u8>>>, Vec<u8>);
}
impl Flat for ((Vec<i32>, Vec<Option<Vec<u8>>>), Vec<u8>) {
    fn into_iter_flat(self) -> (Vec<i32>, Vec<Option<Vec<u8>>>, Vec<u8>) {
        ((self.0).0, (self.0).1, self.1)
    }
}

#[test]
fn c35_composed_pipeline() {
    let l = libs();
    let ca = api(&l.c);
    let ra = api(&l.r);
    for seed in 1..=16u64 {
        let (ci, cs, co) = pipeline(&ca, seed * 0x9E37_79B9);
        let (ri, rs, ro) = pipeline(&ra, seed * 0x9E37_79B9);
        assert_eq!(ci, ri, "C35 seed={seed}: integer results diverge");
        assert_eq!(cs, rs, "C35 seed={seed}: string results diverge");
        assert_eq!(
            co,
            ro,
            "C35 seed={seed}: stdout diverges\n  C   = \"{}\"\n  Rust= \"{}\"",
            show(&co),
            show(&ro)
        );
        assert!(!co.is_empty(), "C35 seed={seed}: pipeline produced no output");
    }
}
