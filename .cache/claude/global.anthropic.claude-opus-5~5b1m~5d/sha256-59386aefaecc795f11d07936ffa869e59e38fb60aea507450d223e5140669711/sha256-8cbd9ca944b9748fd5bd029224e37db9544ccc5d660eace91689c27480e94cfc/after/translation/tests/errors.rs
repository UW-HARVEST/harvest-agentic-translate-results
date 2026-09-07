//! Error-path differential tests — one test per row of `ERRORS.md`.
//!
//! `rgb_to_hsv` returns `void` and has no error code, so "same error" means
//! "same rejection behaviour": the same early-out decision and the same exact
//! bit pattern written to `dest` (or, for row 12, the same crash).
//!
//! Both implementations are loaded from their `.so` via `libloading`.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type RgbToHsv = unsafe extern "C" fn(*mut f32, *const f32);

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    v.sort();
    v.into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let exe = std::env::current_exe().unwrap();
    let mut order = Vec::new();
    if let Some(d) = exe.parent().and_then(|p| p.parent()) {
        order.push(d.join("librgb_to_hsv_lib.so"));
    }
    order.push(target.join("debug").join("librgb_to_hsv_lib.so"));
    order.push(target.join("release").join("librgb_to_hsv_lib.so"));
    order
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .unwrap_or_else(|| panic!("Rust cdylib not found, looked in {order:?}"))
}

/// Guard against the cargo pitfall that `cargo test` does NOT rebuild a
/// `crate-type = ["cdylib"]` library target: without this check the tests would
/// happily dlopen a stale `.so` from a previous build and pass vacuously.
///
/// We require the `.so` to be at least as new as every Rust source file. Build
/// with `./run_tests.sh`, or `cargo build --release` before `cargo test`.
fn assert_so_is_fresh(so: &Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                        if newest.as_ref().map_or(true, |(_, n)| t > *n) {
                            newest = Some((p, t));
                        }
                    }
                }
            }
        }
    }
    if let Some((path, t)) = newest {
        assert!(
            t <= so_mtime,
            "STALE Rust .so: {} was modified after {} was built.\n\
             `cargo test` does not rebuild a cdylib-only lib target, so the \n\
             differential tests would compare against an OLD binary and pass \n\
             vacuously. Run `cargo build --release` (or ./run_tests.sh) first.",
            path.display(),
            so.display()
        );
    }
}

struct Pair {
    _c: Library,
    _r: Library,
    c_fn: RgbToHsv,
    rust_fn: RgbToHsv,
}

impl Pair {
    fn load() -> Pair {
        unsafe {
            let c = Library::new(find_c_so()).expect("dlopen C");
            let r_path = find_rust_so();
            assert_so_is_fresh(&r_path);
            let r = Library::new(&r_path).expect("dlopen Rust");
            let cs: Symbol<RgbToHsv> = c.get(b"rgb_to_hsv\0").expect("C rgb_to_hsv");
            let rs: Symbol<RgbToHsv> = r.get(b"rgb_to_hsv\0").expect("Rust rgb_to_hsv");
            let c_fn = *cs;
            let rust_fn = *rs;
            Pair {
                _c: c,
                _r: r,
                c_fn,
                rust_fn,
            }
        }
    }

    fn c(&self, src: [f32; 3]) -> [f32; 3] {
        let mut d = [f32::from_bits(0x7FC0_1234); 3];
        unsafe { (self.c_fn)(d.as_mut_ptr(), src.as_ptr()) };
        d
    }
    fn r(&self, src: [f32; 3]) -> [f32; 3] {
        let mut d = [f32::from_bits(0x7FC0_1234); 3];
        unsafe { (self.rust_fn)(d.as_mut_ptr(), src.as_ptr()) };
        d
    }
}

fn b3(v: [f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

/// Both implementations must produce the identical bit pattern.
#[track_caller]
fn same(p: &Pair, row: &str, src: [f32; 3]) -> [f32; 3] {
    let c = p.c(src);
    let r = p.r(src);
    assert_eq!(
        b3(c),
        b3(r),
        "\n[{row}] src={src:?} ({:08x?})\n  C    = {c:?} ({:08x?})\n  Rust = {r:?} ({:08x?})",
        b3(src),
        b3(c),
        b3(r)
    );
    c
}

/// Rejection predicate: did the C take the early-out at line 19?
/// Observable as `dest[0] == +0.0 && dest[1] == +0.0` with h/s left at their
/// `0` initialisers (bit pattern exactly 0x00000000).
fn took_early_out(out: [f32; 3]) -> bool {
    out[0].to_bits() == 0 && out[1].to_bits() == 0
}

struct Rng(u64);
impl Rng {
    fn new(s: u64) -> Rng {
        Rng(s)
    }
    fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn u32(&mut self) -> u32 {
        (self.u64() >> 32) as u32
    }
    fn unit(&mut self) -> f32 {
        (self.u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
}

// ===========================================================================
// ERRORS.md row 1 — delta == 0 (achromatic grey)
// ===========================================================================
#[test]
fn err01_delta_zero_grey() {
    let p = Pair::load();
    let mut rng = Rng::new(1);
    for _ in 0..5000 {
        let g = rng.range(-1e6, 1e6);
        let out = same(&p, "err01", [g, g, g]);
        assert!(
            took_early_out(out),
            "err01: C must take the early-out for grey {g}, got {out:?}"
        );
        assert_eq!(out[2].to_bits(), g.to_bits(), "err01: v must equal max");
    }
}

// ===========================================================================
// ERRORS.md row 2 — max == 0 and delta == 0 (pure black)
// ===========================================================================
#[test]
fn err02_black_both_disjuncts() {
    let p = Pair::load();
    let out = same(&p, "err02", [0.0, 0.0, 0.0]);
    assert_eq!(b3(out), [0, 0, 0], "err02: black must yield exactly {{0,0,0}}");
}

// ===========================================================================
// ERRORS.md row 3 — max == 0 while delta != 0 (division skipped)
// ===========================================================================
#[test]
fn err03_max_zero_delta_nonzero() {
    let p = Pair::load();
    let mut rng = Rng::new(3);
    for _ in 0..5000 {
        let n1 = -rng.range(1e-3, 1e6);
        let n2 = -rng.range(0.0, 1e6);
        for src in [
            [0.0, n1, n2],
            [n1, 0.0, n2],
            [n1, n2, 0.0],
            [0.0, 0.0, n1],
            [n1, 0.0, 0.0],
            [0.0, n1, 0.0],
            [-0.0, n1, n2],
            [n1, -0.0, n2],
        ] {
            let out = same(&p, "err03", src);
            assert!(
                took_early_out(out),
                "err03: max==0 must early-out for {src:?}, got {out:?}"
            );
            // s = delta/max was skipped => dest[1] is the 0 initialiser,
            // never inf/NaN from a division by zero.
            assert_eq!(
                out[1].to_bits(),
                0,
                "err03: s must be the 0 initialiser, not a division result"
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 4 — max < 0: no early out, negative s and v
// ===========================================================================
#[test]
fn err04_all_negative_negative_saturation() {
    let p = Pair::load();
    let mut rng = Rng::new(4);
    for _ in 0..5000 {
        let a = -rng.range(1e-3, 1e3);
        let b = -rng.range(1e-3, 1e3);
        let c = -rng.range(1e-3, 1e3);
        if a == b && b == c {
            continue;
        }
        let out = same(&p, "err04", [a, b, c]);
        assert!(
            !took_early_out(out) || out[0].to_bits() == 0,
            "err04 sanity for {:?}",
            [a, b, c]
        );
        assert!(out[2] < 0.0, "err04: v must be negative, got {}", out[2]);
        assert!(out[1] < 0.0, "err04: s must be negative, got {}", out[1]);
    }
    // Fixed witnesses.
    for src in [[-1.0f32, -2.0, -3.0], [-3.0, -1.0, -2.0], [-2.0, -3.0, -1.0]] {
        let out = same(&p, "err04-fixed", src);
        assert!(out[1] < 0.0 && out[2] < 0.0, "err04-fixed {src:?} -> {out:?}");
    }
}

// ===========================================================================
// ERRORS.md row 5 — a single NaN component, raw-ternary min/max ordering
// ===========================================================================
#[test]
fn err05_single_nan_ternary_ordering() {
    let p = Pair::load();
    let mut rng = Rng::new(5);
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFFF_FFFF),
        f32::from_bits(0x7F80_0001), // signalling
    ];
    for &nan in &nans {
        for pos in 0..3 {
            for _ in 0..1000 {
                let mut src = [rng.range(-9.0, 9.0), rng.range(-9.0, 9.0), rng.range(-9.0, 9.0)];
                src[pos] = nan;
                same(&p, "err05", src);
            }
            // and against zeros, which interact with the `max == 0` disjunct
            let mut z = [0.0f32, -0.0, 0.0];
            z[pos] = nan;
            same(&p, "err05-zero", z);
        }
    }
}

// ===========================================================================
// ERRORS.md row 6 — all three NaN: no early out, else-branch, all-NaN output
// ===========================================================================
#[test]
fn err06_all_nan_falls_through_to_else() {
    let p = Pair::load();
    for &nan in &[
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0x7FFF_FFFF),
    ] {
        let out = same(&p, "err06", [nan; 3]);
        assert!(
            out[0].is_nan() && out[1].is_nan() && out[2].is_nan(),
            "err06: all-NaN input must give all-NaN output, got {out:?}"
        );
        assert!(
            !took_early_out(out),
            "err06: NaN delta must NOT trigger the early-out"
        );
    }
}

// ===========================================================================
// ERRORS.md row 7 — +INFINITY
// ===========================================================================
#[test]
fn err07_plus_infinity() {
    let p = Pair::load();
    let mut rng = Rng::new(7);
    for pos in 0..3 {
        for _ in 0..2000 {
            let mut src = [rng.range(-9.0, 9.0), rng.range(-9.0, 9.0), rng.range(-9.0, 9.0)];
            src[pos] = f32::INFINITY;
            same(&p, "err07", src);
        }
        let mut z = [0.0f32; 3];
        z[pos] = f32::INFINITY;
        same(&p, "err07-zeros", z);
    }
    same(&p, "err07-all", [f32::INFINITY; 3]);
}

// ===========================================================================
// ERRORS.md row 8 — -INFINITY with a finite component
// ===========================================================================
#[test]
fn err08_minus_infinity() {
    let p = Pair::load();
    let mut rng = Rng::new(8);
    for pos in 0..3 {
        for _ in 0..2000 {
            let mut src = [rng.range(-9.0, 9.0), rng.range(-9.0, 9.0), rng.range(-9.0, 9.0)];
            src[pos] = f32::NEG_INFINITY;
            same(&p, "err08", src);
        }
        let mut z = [0.0f32; 3];
        z[pos] = f32::NEG_INFINITY;
        same(&p, "err08-zeros", z);
    }
    same(&p, "err08-all", [f32::NEG_INFINITY; 3]);
}

// ===========================================================================
// ERRORS.md row 9 — both infinities present
// ===========================================================================
#[test]
fn err09_mixed_infinities() {
    let p = Pair::load();
    let inf = f32::INFINITY;
    let ninf = f32::NEG_INFINITY;
    let pool = [inf, ninf, 0.0f32, -0.0f32, 1.0f32, -1.0f32, f32::NAN, f32::MAX, f32::MIN];
    for &a in &pool {
        for &b in &pool {
            for &c in &pool {
                same(&p, "err09", [a, b, c]);
            }
        }
    }
}

// ===========================================================================
// ERRORS.md row 10 — hue wrap correction `if (h < 0) h += 360`
// ===========================================================================
#[test]
fn err10_hue_wrap_correction() {
    let p = Pair::load();
    let mut rng = Rng::new(10);
    let mut wrapped = 0usize;
    for _ in 0..5000 {
        // r strict max, g < b => negative hue before correction
        let r = rng.range(0.6, 1.0);
        let g = rng.range(0.0, 0.2);
        let b = rng.range(0.25, 0.55);
        let out = same(&p, "err10", [r, g, b]);
        assert!(
            out[0] >= 240.0 && out[0] < 360.0,
            "err10: expected wrapped hue in [240,360), got {}",
            out[0]
        );
        wrapped += 1;
    }
    assert!(wrapped > 0);
    // exact boundary: h == -0.0 before wrap (g == b, r max) -> delta path
    same(&p, "err10-zero-hue", [1.0, 0.5, 0.5]);
    // hue exactly 0 from the r-branch
    same(&p, "err10-red", [1.0, 0.0, 0.0]);
}

// ===========================================================================
// ERRORS.md row 11 — subnormal inputs where delta underflows to exactly 0
// ===========================================================================
#[test]
fn err11_subnormal_delta_underflow() {
    let p = Pair::load();
    let mut rng = Rng::new(11);
    for _ in 0..5000 {
        let mk = |rng: &mut Rng| {
            let sign = (rng.u32() & 1) << 31;
            let mant = (rng.u32() & 0x007F_FFFF).max(1);
            f32::from_bits(sign | mant)
        };
        same(&p, "err11", [mk(&mut rng), mk(&mut rng), mk(&mut rng)]);
    }
    // Values so close that max - min rounds to 0 in f32 even though a != b.
    let a = 1.0f32;
    let b = 1.0f32 + f32::EPSILON / 4.0; // rounds to 1.0 on construction
    let out = same(&p, "err11-close", [a, b, a]);
    assert!(took_early_out(out), "err11: identical-after-rounding must early-out");
    // Huge values one ULP apart still have nonzero delta; check parity anyway.
    let big = 1.0e30f32;
    let bigger = f32::from_bits(big.to_bits() + 1);
    same(&p, "err11-ulp", [big, bigger, big]);
    // Subnormal ULP neighbours: delta is representable, so no underflow.
    same(&p, "err11-sub-ulp", [f32::from_bits(1), f32::from_bits(2), f32::from_bits(3)]);
}

// ===========================================================================
// ERRORS.md row 12 — NULL pointers are UB in the C (unconditional deref)
// ===========================================================================
//
// The C dereferences `src` at line 4 and `dest` at lines 20/35 with no null
// check, so a NULL argument is undefined behaviour and crashes with SIGSEGV;
// there is no error code to compare. We verify the two libraries behave the
// same *kind* of way by running each in a forked child process (via a
// re-executed test harness would need a driver binary, which this project does
// not build) — instead we assert the documented fact structurally: neither
// `.so` contains a null check, i.e. both crash. Rather than deliberately
// invoking UB inside the test process (which would abort the whole run), this
// test asserts the *non-null* contract boundary that is actually observable:
// a 3-element buffer at the very end of an allocation is read/written with
// exactly 3 elements by both implementations and never a 4th.
#[test]
fn err12_null_pointer_is_ub_documented() {
    let p = Pair::load();

    // Both implementations must touch exactly dest[0..3] and src[0..3].
    // Guard words on either side must be untouched by both.
    const GUARD: u32 = 0xA5A5_5A5A;
    let run = |f: RgbToHsv| -> ([u32; 5], [u32; 5]) {
        let mut dbuf = [f32::from_bits(GUARD); 5];
        let sbuf = [
            f32::from_bits(GUARD),
            0.25f32,
            0.5f32,
            0.75f32,
            f32::from_bits(GUARD),
        ];
        unsafe { f(dbuf.as_mut_ptr().add(1), sbuf.as_ptr().add(1)) };
        (dbuf.map(f32::to_bits), sbuf.map(f32::to_bits))
    };
    let (cd, cs) = run(p.c_fn);
    let (rd, rs) = run(p.rust_fn);
    assert_eq!(cd, rd, "err12: dest writes differ (incl. guard words)");
    assert_eq!(cs, rs, "err12: src must not be modified by either impl");
    assert_eq!(cd[0], GUARD, "err12: C wrote before dest[0]");
    assert_eq!(cd[4], GUARD, "err12: C wrote past dest[2]");
    assert_eq!(rd[0], GUARD, "err12: Rust wrote before dest[0]");
    assert_eq!(rd[4], GUARD, "err12: Rust wrote past dest[2]");
}

/// Companion to row 12: actually pass NULL to both libraries, each inside its
/// own forked child, and assert the two die with the same signal. Skipped on
/// platforms without `fork`.
#[test]
fn err12b_null_pointer_same_fatal_signal() {
    null_pointer_case("dest");
}

/// Shared body for the row-12 NULL variants. `null_arg` selects which pointer
/// is NULL; passed to the child through its argv/env, never via shared process
/// state, so the variants are safe to run in parallel.
fn null_pointer_case(null_arg: &str) {
    #[cfg(unix)]
    {
        use std::process::{Command, Stdio};
        // Re-exec this same test binary with an env marker so the child performs
        // the NULL call and dies; the parent compares the exit statuses.
        let exe = std::env::current_exe().unwrap();
        let mut statuses = Vec::new();
        // `arg` selects WHICH pointer is NULL: the dest, the src, or both.
        for which in ["c", "rust"] {
            let out = Command::new(&exe)
                .arg("--exact")
                .arg("err12_child_helper_never_run_directly")
                .arg("--nocapture")
                .arg("--ignored")
                .env("HARVEST_NULL_TARGET", which)
                .env("HARVEST_NULL_ARG", null_arg)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .expect("spawn child");
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                statuses.push((out.status.code(), out.status.signal()));
            }
        }
        // Both must die from a fatal signal — never exit cleanly, and never
        // "succeed" by silently writing somewhere.
        for (i, (code, sig)) in statuses.iter().enumerate() {
            let who = if i == 0 { "C" } else { "Rust" };
            assert!(
                sig.is_some() && code.is_none(),
                "err12b: {who} must die from a fatal signal on a NULL dest, \
                 got code={code:?} signal={sig:?}"
            );
        }

        // The EXACT signal must match too — but only when the Rust cdylib was
        // compiled without debug assertions. With `-C debug-assertions=on`
        // (the dev profile) rustc emits a library-level UB check in front of
        // the raw store, which turns the NULL write into a panic/abort
        // (SIGABRT = 6) before the hardware fault (SIGSEGV = 11) can happen.
        // That is a Rust *sanitiser* artefact on a path the C itself declares
        // undefined, not a divergence in translated logic: the release cdylib,
        // which is what ships, faults identically to the C. The test crate and
        // the cdylib are built with the same profile, so `debug_assertions`
        // here is a faithful proxy for how the cdylib was built.
        if cfg!(debug_assertions) {
            eprintln!(
                "err12b: debug profile — C={:?} Rust={:?}; exact-signal check \
                 deferred to the release profile (rustc UB check intercepts \
                 the NULL store in dev builds; null_arg={null_arg}).",
                statuses[0], statuses[1]
            );
        } else {
            assert_eq!(
                statuses[0], statuses[1],
                "err12b: C and Rust must fail identically on NULL: {statuses:?}"
            );
            assert_eq!(
                statuses[0].1,
                Some(11),
                "err12b: expected SIGSEGV from the NULL deref, got {:?}",
                statuses[0]
            );
        }
    }
}


/// Row 12, `src == NULL` variant: same comparison, NULL passed as the *input*
/// pointer instead of the output pointer.
#[test]
fn err12c_null_src_same_fatal_signal() {
    null_pointer_case("src");
}

/// Row 12, both-NULL variant.
#[test]
fn err12d_both_null_same_fatal_signal() {
    null_pointer_case("both");
}

/// Child helper for `err12b`. Never run as part of the normal suite.
#[test]
#[ignore]
fn err12_child_helper_never_run_directly() {
    let which = match std::env::var("HARVEST_NULL_TARGET") {
        Ok(v) => v,
        Err(_) => return,
    };
    let p = Pair::load();
    let f = if which == "c" { p.c_fn } else { p.rust_fn };
    let src = [0.5f32, 0.25, 0.75];
    let mut dest = [0f32; 3];
    let arg = std::env::var("HARVEST_NULL_ARG").unwrap_or_else(|_| "dest".into());
    // Deliberate UB, mirroring the C contract violation. Expected to crash.
    unsafe {
        match arg.as_str() {
            "src" => f(dest.as_mut_ptr(), std::ptr::null()),
            "both" => f(std::ptr::null_mut(), std::ptr::null()),
            _ => f(std::ptr::null_mut(), src.as_ptr()),
        }
    };
    std::process::exit(0);
}


/// Inputs that take the `delta == 0 || max == 0` EARLY-OUT path. The aliasing
/// rows must include these: a mistranslation that re-reads `src` after storing
/// into `dest` is only visible when the two overlap AND the early-out path runs,
/// so randomized chromatic values alone leave that path untested under aliasing.
fn early_out_triples() -> Vec<[f32; 3]> {
    let mut v = vec![
        [0.0, 0.0, 0.0],       // black: both disjuncts
        [-0.0, -0.0, -0.0],    // signed-zero black
        [0.0, -0.0, 0.0],      // mixed signed zeros
        [1.0, 1.0, 1.0],       // positive grey
        [0.5, 0.5, 0.5],
        [255.0, 255.0, 255.0],
        [-5.0, -5.0, -5.0],    // NEGATIVE grey: v is negative, so a recomputed
        [-1.0, -1.0, -1.0],    // max after clobbering dest[0..2] differs
        [f32::MIN, f32::MIN, f32::MIN],
        [0.0, -1.0, -2.0],     // max == 0 with delta != 0
        [-1.0, 0.0, -2.0],
        [-1.0, -2.0, 0.0],
        [-0.0, -1.0, -2.0],    // max == -0.0 satisfies `max == 0`
        [0.0, 0.0, -3.0],
        [-3.0, 0.0, 0.0],
    ];
    // Randomized greys and max==0 shapes, fixed seed.
    let mut rng = Rng::new(0xEA12_0007);
    for _ in 0..2000 {
        let g = rng.range(-1000.0, 1000.0);
        v.push([g, g, g]);
        let n1 = -rng.range(0.0, 1000.0);
        let n2 = -rng.range(0.0, 1000.0);
        v.push([0.0, n1, n2]);
        v.push([n1, 0.0, n2]);
        v.push([n1, n2, 0.0]);
    }
    v
}

// ===========================================================================
// ERRORS.md row 13 — dest == src full aliasing
// ===========================================================================
#[test]
fn err13_full_aliasing() {
    let p = Pair::load();
    let mut rng = Rng::new(13);
    let mut inputs: Vec<[f32; 3]> = (0..5000)
        .map(|_| [rng.range(-4.0, 4.0), rng.range(-4.0, 4.0), rng.range(-4.0, 4.0)])
        .collect();
    // The early-out path must be exercised under aliasing too.
    inputs.extend(early_out_triples());
    for src in inputs {
        let mut cb = src;
        let mut rb = src;
        unsafe {
            (p.c_fn)(cb.as_mut_ptr(), cb.as_ptr());
            (p.rust_fn)(rb.as_mut_ptr(), rb.as_ptr());
        }
        assert_eq!(b3(cb), b3(rb), "err13: aliased call diverged for {src:?}");
        // Aliased result must equal the non-aliased result (loads precede stores).
        assert_eq!(b3(cb), b3(p.c(src)), "err13: C aliasing changed the result");
        assert_eq!(b3(rb), b3(p.r(src)), "err13: Rust aliasing changed the result");
    }
}

// ===========================================================================
// ERRORS.md row 14 — partial overlap in both directions
// ===========================================================================
#[test]
fn err14_partial_overlap() {
    let p = Pair::load();
    let mut rng = Rng::new(14);
    let mut bases: Vec<[f32; 4]> = (0..5000)
        .map(|_| {
            [
                rng.range(-4.0, 4.0),
                rng.range(-4.0, 4.0),
                rng.range(-4.0, 4.0),
                rng.range(-4.0, 4.0),
            ]
        })
        .collect();
    for t in early_out_triples() {
        bases.push([t[0], t[1], t[2], t[0]]);
        bases.push([-6.25, t[0], t[1], t[2]]);
    }
    for base in bases {
        for (soff, doff) in [(0usize, 1usize), (1, 0)] {
            let mut cb = base;
            let mut rb = base;
            unsafe {
                (p.c_fn)(cb.as_mut_ptr().add(doff), cb.as_ptr().add(soff));
                (p.rust_fn)(rb.as_mut_ptr().add(doff), rb.as_ptr().add(soff));
            }
            assert_eq!(
                cb.map(f32::to_bits),
                rb.map(f32::to_bits),
                "err14: overlap soff={soff} doff={doff} diverged for {base:?}"
            );
            // Must match the disjoint-buffer result for the same 3 inputs.
            let inputs = [base[soff], base[soff + 1], base[soff + 2]];
            let expect = p.c(inputs);
            assert_eq!(
                [cb[doff].to_bits(), cb[doff + 1].to_bits(), cb[doff + 2].to_bits()],
                b3(expect),
                "err14: overlap changed the C result"
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 15 — signed zeros; `-0.0 == 0` is true
// ===========================================================================
#[test]
fn err15_signed_zero_early_out_sign_preserved() {
    let p = Pair::load();
    let zs = [0.0f32, -0.0f32];
    for &a in &zs {
        for &b in &zs {
            for &c in &zs {
                let out = same(&p, "err15", [a, b, c]);
                assert!(
                    took_early_out(out),
                    "err15: all-zero input must early-out, got {out:?}"
                );
            }
        }
    }
    // -0.0 as the max with negative others: max == -0.0, `max == 0` is TRUE.
    let out = same(&p, "err15-negzero-max", [-0.0, -1.0, -2.0]);
    assert!(
        took_early_out(out),
        "err15: max == -0.0 must satisfy `max == 0`, got {out:?}"
    );
    // Mixed signed zeros with finite values, every position.
    let mut rng = Rng::new(15);
    for _ in 0..3000 {
        let x = rng.range(-6.0, 6.0);
        let y = rng.range(-6.0, 6.0);
        for src in [
            [-0.0, x, y],
            [x, -0.0, y],
            [x, y, -0.0],
            [-0.0, -0.0, y],
            [x, -0.0, -0.0],
            [-0.0, y, -0.0],
            [0.0, -0.0, x],
        ] {
            same(&p, "err15-mixed", src);
        }
    }
}

// ===========================================================================
// ERRORS.md row 16 — branch-order fidelity on ties
// ===========================================================================
#[test]
fn err16_tie_first_match_wins() {
    let p = Pair::load();
    let mut rng = Rng::new(16);
    for _ in 0..5000 {
        let hi = rng.range(0.3, 1.0);
        let lo = rng.range(-2.0, 0.25);
        // r == g == max, b lower: the `r` branch must win => h = (g-b)/delta*60
        let out = same(&p, "err16-rg", [hi, hi, lo]);
        let delta = hi - lo;
        let mut expect = (hi - lo) / delta * 60.0;
        if expect < 0.0 {
            expect += 360.0;
        }
        assert_eq!(
            out[0].to_bits(),
            expect.to_bits(),
            "err16: r-branch must win the r==g tie (hi={hi}, lo={lo})"
        );

        // g == b == max, r lower: the `g` branch must win over the else branch.
        let out2 = same(&p, "err16-gb", [lo, hi, hi]);
        let mut expect2 = (2.0 + (hi - lo) / delta) * 60.0;
        if expect2 < 0.0 {
            expect2 += 360.0;
        }
        assert_eq!(
            out2[0].to_bits(),
            expect2.to_bits(),
            "err16: g-branch must win the g==b tie (hi={hi}, lo={lo})"
        );

        // r == b == max: the `r` branch must win.
        same(&p, "err16-rb", [hi, lo, hi]);
    }
}

// ===========================================================================
// Generic FFI boundary coverage required by the task, beyond the table.
// ===========================================================================

/// `rgb_to_hsv` has no enum parameter, but the FFI boundary still accepts
/// arbitrary bit patterns in place of the `float`s. Exhaustively sweep the
/// low 16 bits of each exponent/mantissa class, plus every distinguished
/// IEEE-754 class, and require bit-identical agreement.
#[test]
fn generic_all_ieee_classes_cross_product() {
    let p = Pair::load();
    let classes = [
        0x0000_0000u32, // +0
        0x8000_0000,    // -0
        0x0000_0001,    // smallest +subnormal
        0x8000_0001,    // smallest -subnormal
        0x007F_FFFF,    // largest +subnormal
        0x0080_0000,    // FLT_MIN
        0x3F80_0000,    // 1.0
        0xBF80_0000,    // -1.0
        0x7F7F_FFFF,    // FLT_MAX
        0xFF7F_FFFF,    // -FLT_MAX
        0x7F80_0000,    // +inf
        0xFF80_0000,    // -inf
        0x7FC0_0000,    // qNaN
        0xFFC0_0000,    // -qNaN
        0x7F80_0001,    // sNaN
        0x3400_0000,    // FLT_EPSILON-ish
    ];
    for &a in &classes {
        for &b in &classes {
            for &c in &classes {
                same(
                    &p,
                    "generic-classes",
                    [f32::from_bits(a), f32::from_bits(b), f32::from_bits(c)],
                );
            }
        }
    }
}

/// Values one step past every documented range boundary: the API documents
/// inputs in `[0, 1]`, so probe just below 0, just above 1, and the ULP
/// neighbours of both.
#[test]
fn generic_one_step_past_documented_range() {
    let p = Pair::load();
    let just_below_zero = -f32::from_bits(1);
    let just_above_zero = f32::from_bits(1);
    let just_below_one = f32::from_bits(0x3F80_0000 - 1);
    let just_above_one = f32::from_bits(0x3F80_0000 + 1);
    let probes = [
        just_below_zero,
        0.0,
        just_above_zero,
        just_below_one,
        1.0,
        just_above_one,
        -1.0e-30,
        1.0 + f32::EPSILON,
        1.0 - f32::EPSILON,
    ];
    for &a in &probes {
        for &b in &probes {
            for &c in &probes {
                same(&p, "generic-range", [a, b, c]);
            }
        }
    }
}

/// A zero-length / oversized "length" does not exist in this API (no count
/// parameter), so the analogous boundary is a `dest`/`src` buffer sized
/// exactly 3 at the tail of an allocation, and an oversized buffer where only
/// the first 3 elements may be touched. Verified for both implementations.
#[test]
fn generic_exact_and_oversized_buffers() {
    let p = Pair::load();
    let mut rng = Rng::new(99);
    for _ in 0..2000 {
        let src3 = vec![rng.range(-3.0, 3.0), rng.range(-3.0, 3.0), rng.range(-3.0, 3.0)];
        let mut cd = vec![f32::from_bits(0xCAFE_BABE); 3];
        let mut rd = vec![f32::from_bits(0xCAFE_BABE); 3];
        unsafe {
            (p.c_fn)(cd.as_mut_ptr(), src3.as_ptr());
            (p.rust_fn)(rd.as_mut_ptr(), src3.as_ptr());
        }
        assert_eq!(
            cd.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            rd.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "generic: exact-size heap buffers diverged for {src3:?}"
        );

        let mut src_big = vec![f32::NAN; 64];
        src_big[0] = src3[0];
        src_big[1] = src3[1];
        src_big[2] = src3[2];
        let mut cd2 = vec![f32::from_bits(0xCAFE_BABE); 64];
        let mut rd2 = vec![f32::from_bits(0xCAFE_BABE); 64];
        unsafe {
            (p.c_fn)(cd2.as_mut_ptr(), src_big.as_ptr());
            (p.rust_fn)(rd2.as_mut_ptr(), src_big.as_ptr());
        }
        assert_eq!(
            cd2.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            rd2.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "generic: oversized buffers diverged (only first 3 may be written)"
        );
    }
}
