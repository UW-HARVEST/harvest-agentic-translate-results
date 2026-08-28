//! Differential test: loads BOTH the C `.so` and the Rust `cdylib` via
//! `libloading` and compares `normalize` outputs bit-for-bit.
//!
//! Neither implementation is called directly; both go through the dynamic
//! symbol `normalize`, so the `#[unsafe(no_mangle)] extern "C"` wrapper is
//! exercised exactly as an external C caller would exercise it.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `void normalize(float *dest, const float *src, int size)`
type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, std::ffi::c_int);

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    // Allow pointing at an alternative C build (e.g. an optimized one) without
    // touching c_src/.
    if let Some(p) = std::env::var_os("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so walk up to the
    // profile directory and pick up the cdylib built alongside it.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>/deps/<test>");
    for name in ["libnormalize_lib.so", "normalize_lib.so"] {
        let p = profile_dir.join(name);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found in {}. Run `cargo build` for the same profile.",
        profile_dir.display()
    );
}

/// `cargo test` does not rebuild a `crate-type = ["cdylib"]` target, so the
/// `.so` under test can silently lag behind `src/`. Without this guard a stale
/// library would be tested and every case would pass regardless of the source.
/// Run `cargo build` (matching profile) before `cargo test`.
fn assert_so_is_current(rust_so: &Path) {
    let so_mtime = std::fs::metadata(rust_so)
        .and_then(|m| m.modified())
        .expect("stat Rust .so");

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            if let Ok(t) = entry.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().is_none_or(|(_, best)| t > *best) {
                    newest = Some((path, t));
                }
            }
        }
    }

    if let Some((path, src_mtime)) = newest {
        assert!(
            so_mtime >= src_mtime,
            "stale cdylib: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib -- run `cargo build` \
             (same profile and features) first, otherwise these tests would \
             validate an out-of-date library.",
            rust_so.display(),
            path.display(),
        );
    }
}

struct Impls {
    _c_lib: Library,
    _rs_lib: Library,
    c: NormalizeFn,
    rs: NormalizeFn,
}

impl Impls {
    fn load() -> Self {
        let rust_so = find_rust_so();
        assert_so_is_current(&rust_so);
        unsafe {
            let c_lib = Library::new(find_c_so()).expect("load C .so");
            let rs_lib = Library::new(rust_so).expect("load Rust .so");
            let c: Symbol<NormalizeFn> = c_lib.get(b"normalize\0").expect("C normalize");
            let rs: Symbol<NormalizeFn> = rs_lib.get(b"normalize\0").expect("Rust normalize");
            let (c, rs) = (*c, *rs);
            Impls { _c_lib: c_lib, _rs_lib: rs_lib, c, rs }
        }
    }
}

/// Bit pattern comparison so that `-0.0` vs `0.0` and differing NaN payloads
/// are treated as mismatches.
fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|f| f.to_bits()).collect()
}

fn describe(v: &[f32]) -> String {
    let shown: Vec<String> = v
        .iter()
        .take(12)
        .map(|f| format!("{f:e}/{:#010x}", f.to_bits()))
        .collect();
    let mut s = shown.join(", ");
    if v.len() > 12 {
        s.push_str(", ...");
    }
    s
}

/// Separate `dest` and `src` buffers. `dest` is pre-filled with a sentinel so
/// that "left untouched" is distinguishable from "zeroed".
fn check_out_of_place(im: &Impls, label: &str, src: &[f32], size: std::ffi::c_int) {
    const SENTINEL: f32 = -1.234_567_8e-3;
    let mut dest_c = vec![SENTINEL; src.len()];
    let mut dest_rs = vec![SENTINEL; src.len()];
    let src_c = src.to_vec();
    let src_rs = src.to_vec();

    unsafe {
        (im.c)(dest_c.as_mut_ptr(), src_c.as_ptr(), size);
        (im.rs)(dest_rs.as_mut_ptr(), src_rs.as_ptr(), size);
    }

    assert_eq!(
        bits(&dest_c),
        bits(&dest_rs),
        "[{label}] out-of-place dest mismatch\n  size = {size}\n  src  = [{}]\n  C    = [{}]\n  Rust = [{}]",
        describe(src),
        describe(&dest_c),
        describe(&dest_rs),
    );
    // `src` is const in C; confirm neither side scribbled on it.
    assert_eq!(bits(&src_c), bits(&src_rs), "[{label}] src buffer diverged");
    assert_eq!(bits(&src_c), bits(src), "[{label}] src buffer was modified");
}

/// `dest == src`: the in-place path, which also selects the `dest != src`
/// branch's *false* arm when the sum of squares is not positive.
fn check_in_place(im: &Impls, label: &str, src: &[f32], size: std::ffi::c_int) {
    let mut buf_c = src.to_vec();
    let mut buf_rs = src.to_vec();

    unsafe {
        let p_c = buf_c.as_mut_ptr();
        (im.c)(p_c, p_c, size);
        let p_rs = buf_rs.as_mut_ptr();
        (im.rs)(p_rs, p_rs, size);
    }

    assert_eq!(
        bits(&buf_c),
        bits(&buf_rs),
        "[{label}] in-place mismatch\n  size = {size}\n  src  = [{}]\n  C    = [{}]\n  Rust = [{}]",
        describe(src),
        describe(&buf_c),
        describe(&buf_rs),
    );
}

/// Overlapping-but-not-equal buffers: `dest` trails `src` inside one
/// allocation, so the forward element-wise loop reads values it has already
/// overwritten. Confirms the Rust port copies element-by-element rather than
/// via a slice/`copy_nonoverlapping` shortcut.
fn check_overlapping(im: &Impls, label: &str, data: &[f32], offset: usize, size: std::ffi::c_int) {
    assert!(offset > 0 && offset + (size.max(0) as usize) <= data.len());
    let mut buf_c = data.to_vec();
    let mut buf_rs = data.to_vec();

    unsafe {
        (im.c)(buf_c.as_mut_ptr(), buf_c.as_ptr().add(offset), size);
        (im.rs)(buf_rs.as_mut_ptr(), buf_rs.as_ptr().add(offset), size);
    }

    assert_eq!(
        bits(&buf_c),
        bits(&buf_rs),
        "[{label}] overlapping mismatch\n  size = {size}, offset = {offset}\n  in   = [{}]\n  C    = [{}]\n  Rust = [{}]",
        describe(data),
        describe(&buf_c),
        describe(&buf_rs),
    );
}

/// Every safe calling convention for one input vector.
fn check_all(im: &Impls, label: &str, src: &[f32]) {
    let size = src.len() as std::ffi::c_int;
    check_out_of_place(im, label, src, size);
    check_in_place(im, label, src, size);
    if src.len() >= 2 {
        check_overlapping(im, label, src, 1, size - 1);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn exports_normalize_symbol() {
    // Loading is itself the assertion: a missing `#[no_mangle]` export fails here.
    let _ = Impls::load();
}

#[test]
fn zero_and_tiny_sizes() {
    let im = Impls::load();
    // size == 0: loop never runs, sum stays 0, so the `dest != src` arm
    // memsets 0 bytes -> dest keeps its sentinel.
    check_out_of_place(&im, "size0-empty", &[], 0);
    check_out_of_place(&im, "size0-nonempty-buf", &[3.0, 4.0], 0);
    check_in_place(&im, "size0-in-place", &[3.0, 4.0], 0);

    check_all(&im, "single-positive", &[5.0]);
    check_all(&im, "single-negative", &[-5.0]);
    check_all(&im, "single-zero", &[0.0]);
    check_all(&im, "single-neg-zero", &[-0.0]);
}

#[test]
fn negative_size_in_place() {
    // A negative `size` skips the loops; with dest == src the C code takes the
    // `else if (dest != src)` false arm and touches nothing. (Out-of-place with
    // a negative size sign-extends into a huge `memset` length and crashes in
    // C too, so it is not exercised.)
    let im = Impls::load();
    for size in [-1, -7, i32::MIN] {
        check_in_place(&im, &format!("negsize{size}"), &[1.0, -2.0, 3.5], size);
    }
}

#[test]
fn all_zero_inputs_take_the_memset_path() {
    let im = Impls::load();
    for len in [1usize, 2, 3, 8, 17, 64] {
        check_all(&im, &format!("zeros{len}"), &vec![0.0f32; len]);
    }
    // Mixed signed zeros still sum to +0.0, which is not > 0.
    check_all(&im, "signed-zeros", &[0.0, -0.0, 0.0, -0.0, -0.0]);
}

#[test]
fn ordinary_vectors() {
    let im = Impls::load();
    check_all(&im, "3-4", &[3.0, 4.0]);
    check_all(&im, "unit-x", &[1.0, 0.0, 0.0]);
    check_all(&im, "ones8", &[1.0; 8]);
    check_all(&im, "mixed", &[1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0]);
    check_all(&im, "fractions", &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]);
    check_all(&im, "already-unit", &[0.6, 0.8]);
    check_all(&im, "tiny-and-huge", &[1e-20, 1e20, 1.0]);
    check_all(&im, "descending", &[9.75, -8.5, 7.25, -6.0, 4.875]);
}

#[test]
fn special_values() {
    let im = Impls::load();
    let inf = f32::INFINITY;
    let nan = f32::NAN;

    // sum -> inf: 1/sqrt(inf) == 0, so finite entries become +/-0 and
    // infinite entries become NaN (inf * 0).
    check_all(&im, "inf-single", &[inf]);
    check_all(&im, "inf-neg", &[-inf]);
    check_all(&im, "inf-mixed", &[inf, 1.0, -2.0]);
    check_all(&im, "inf-both", &[inf, -inf]);

    // sum -> NaN: `NaN > 0.0f` is false, so the memset arm is taken instead.
    check_all(&im, "nan-single", &[nan]);
    check_all(&im, "nan-mixed", &[1.0, nan, 3.0]);
    check_all(&im, "nan-and-inf", &[inf, nan]);

    // Overflow during accumulation from finite inputs.
    check_all(&im, "overflow", &[f32::MAX, f32::MAX]);
    check_all(&im, "near-max", &[f32::MAX]);
    check_all(&im, "sqrt-underflow", &[f32::MIN_POSITIVE, f32::MIN_POSITIVE]);

    // Subnormals: squares flush to zero, so the sum is +0.0 and the memset
    // arm runs even though the input is non-zero.
    let denorm = f32::from_bits(1);
    check_all(&im, "subnormal-min", &[denorm, denorm, denorm]);
    check_all(&im, "subnormal-max", &[f32::from_bits(0x007f_ffff); 4]);
    check_all(&im, "subnormal-mixed", &[denorm, 1.0]);

    // Non-canonical NaN payloads, incl. a signalling pattern, to confirm the
    // exact bits that survive the multiply.
    for (label, b) in [
        ("qnan-payload", 0x7fc1_2345u32),
        ("snan-payload", 0x7f80_0001u32),
        ("neg-qnan", 0xffc0_0000u32),
    ] {
        check_all(&im, label, &[f32::from_bits(b), 2.0, -3.0]);
    }
}

#[test]
fn accumulation_order_is_observable() {
    // f32 addition is non-associative; these inputs give a different `sum`
    // if the accumulation is reordered or widened to f64, which would shift
    // the final quotient by an ULP or more.
    let im = Impls::load();
    let mut v = Vec::new();
    v.push(1.0f32);
    for _ in 0..2048 {
        v.push(1e-4);
    }
    check_all(&im, "asymmetric-accum", &v);

    let cancelling: Vec<f32> = (0..512)
        .map(|i| if i % 2 == 0 { 1.6777216e7 } else { 1.0 })
        .collect();
    check_all(&im, "cancelling-accum", &cancelling);
}

#[test]
fn pseudorandom_sweep() {
    let im = Impls::load();
    // xorshift64* for reproducibility without extra dependencies.
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    };

    for len in 1usize..=48 {
        for round in 0..24 {
            // Alternate generators to cover magnitudes, sign patterns, and raw
            // bit patterns (which include NaNs/infs/subnormals).
            let src: Vec<f32> = (0..len)
                .map(|_| {
                    let r = next();
                    match round % 4 {
                        0 => (r >> 40) as f32 / 1024.0 - 1.0,
                        1 => f32::from_bits((r >> 32) as u32) * 1e-30,
                        2 => {
                            let f = f32::from_bits((r >> 32) as u32);
                            if f.is_finite() { f } else { 0.0 }
                        }
                        _ => f32::from_bits((r >> 32) as u32),
                    }
                })
                .collect();
            check_all(&im, &format!("rand-len{len}-r{round}"), &src);
        }
    }
}

#[test]
fn partial_size_smaller_than_buffer() {
    // `size` shorter than the allocation: confirms nothing past `size` is
    // written on either the scaling path or the memset path.
    let im = Impls::load();
    let src = [3.0f32, 4.0, 100.0, 200.0, 300.0];
    check_out_of_place(&im, "partial-scale", &src, 2);
    check_in_place(&im, "partial-scale-inplace", &src, 2);

    let zeros_then_data = [0.0f32, 0.0, 7.0, 8.0];
    check_out_of_place(&im, "partial-memset", &zeros_then_data, 2);
    check_in_place(&im, "partial-memset-inplace", &zeros_then_data, 2);
}

#[test]
fn large_buffers() {
    let im = Impls::load();
    let n = 100_000usize;
    let ramp: Vec<f32> = (0..n).map(|i| (i as f32) * 1e-3 - 50.0).collect();
    check_all(&im, "large-ramp", &ramp);
    check_all(&im, "large-zeros", &vec![0.0f32; n]);
}
