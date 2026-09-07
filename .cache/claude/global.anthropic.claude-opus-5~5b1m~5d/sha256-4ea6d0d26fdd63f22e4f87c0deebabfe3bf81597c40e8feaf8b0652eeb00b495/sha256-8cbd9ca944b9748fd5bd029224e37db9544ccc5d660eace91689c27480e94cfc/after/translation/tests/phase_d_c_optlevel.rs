//! Diagnostic: is the C library's own output stable across optimisation levels?
//!
//! Set `HSL_C_SO` and `HSL_C_SO_ALT` to two builds of `c_src` (e.g. the default
//! `-O0` build that `CMakeLists.txt` produces and a `-DCMAKE_BUILD_TYPE=Release`
//! `-O2` build) and this test reports every input on which the two C builds
//! disagree. If they disagree, no single Rust implementation can be
//! bit-identical to both, and the correct target is the build the project's own
//! `CMakeLists.txt` produces.
//!
//! Skipped (passes trivially) when `HSL_C_SO_ALT` is unset.

mod common;

use common::Rng;
use libloading::{Library, Symbol};

type HslToRgb = unsafe extern "C" fn(*mut f32, *const f32);

fn load(path: &str) -> (Library, HslToRgb) {
    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path}: {e}"));
        let f = {
            let s: Symbol<HslToRgb> = lib.get(b"hsl_to_rgb\0").expect("hsl_to_rgb");
            *s
        };
        (lib, f)
    }
}

#[test]
fn c_builds_agree_across_opt_levels() {
    let (a_path, b_path) = match (std::env::var("HSL_C_SO"), std::env::var("HSL_C_SO_ALT")) {
        (Ok(a), Ok(b)) => (a, b),
        _ => {
            eprintln!("skipped: set HSL_C_SO and HSL_C_SO_ALT to compare two C builds");
            return;
        }
    };
    let (_la, fa) = load(&a_path);
    let (_lb, fb) = load(&b_path);

    let mut r = Rng::new(0xC0FFEE);
    let mut diffs: Vec<String> = Vec::new();
    let mut n = 0usize;
    for _ in 0..200_000 {
        let src = [r.any_f32(), r.any_f32(), r.any_f32()];
        let mut oa = [0.0f32; 3];
        let mut ob = [0.0f32; 3];
        unsafe {
            fa(oa.as_mut_ptr(), src.as_ptr());
            fb(ob.as_mut_ptr(), src.as_ptr());
        }
        n += 1;
        if oa.map(f32::to_bits) != ob.map(f32::to_bits) && diffs.len() < 5 {
            diffs.push(format!(
                "  src={}\n    A({a_path})={}\n    B({b_path})={}",
                common::show(&src),
                common::show(&oa),
                common::show(&ob)
            ));
        }
    }
    // NaN-heavy, where operand-order differences surface.
    for _ in 0..200_000 {
        let src = [r.any_nan(), r.any_nan(), r.any_nan()];
        let mut oa = [0.0f32; 3];
        let mut ob = [0.0f32; 3];
        unsafe {
            fa(oa.as_mut_ptr(), src.as_ptr());
            fb(ob.as_mut_ptr(), src.as_ptr());
        }
        n += 1;
        if oa.map(f32::to_bits) != ob.map(f32::to_bits) && diffs.len() < 10 {
            diffs.push(format!(
                "  src={}\n    A={}\n    B={}",
                common::show(&src),
                common::show(&oa),
                common::show(&ob)
            ));
        }
    }

    if !diffs.is_empty() {
        panic!(
            "the two C builds disagree with EACH OTHER on {}/{n} sampled inputs \
             (showing up to 10):\n{}",
            diffs.len(),
            diffs.join("\n")
        );
    }
    eprintln!("the two C builds agree on all {n} sampled inputs");
}
