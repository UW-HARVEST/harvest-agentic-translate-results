//! Exhaustive hue sweep.
//!
//! `h` is the *only* input that reaches `fmodf`, and `fmodf` is the one place
//! where the two shared objects genuinely execute different machine code: the C
//! calls glibc's `fmodf@GLIBC_2.2.5` while the Rust `.so` binds to the `fmodf`
//! that `compiler_builtins` statically provides (see `SYMBOLS.md`). Random
//! sampling can only ever cover a vanishing fraction of the 2^32 hue patterns,
//! so this file walks them exhaustively.
//!
//! The sweep is split into strided chunks so it can run inside the time budget:
//! `HSL_SWEEP_STRIDE` (default 4096, which keeps the in-suite run to a few
//! seconds) and `HSL_SWEEP_OFFSET` (default 0) select the
//! residue class `bits % stride == offset`. `HSL_SWEEP_STRIDE=1` walks all
//! 4 294 967 296 hues (see `sweep.sh`, which drives every residue class).

mod common;

use common::*;

/// When `HSL_SWEEP_SINGLE=1`, the outer loops over hue (and saturation) collapse
/// to their first entry, which is what makes a complete stride-1 sweep of the
/// remaining component affordable.
fn single_config() -> bool {
    std::env::var("HSL_SWEEP_SINGLE").map(|v| v == "1").unwrap_or(false)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Tight differential loop: no heap traffic per call, so the sweep is bounded by
/// the libraries themselves rather than by the harness.
fn sweep(s: f32, l: f32, stride: u64, offset: u64) -> u64 {
    let c = c_lib();
    let rust = rust_libs();
    let mut src = [0.0f32, s, l];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    let mut n = 0u64;

    let mut bits = offset;
    while bits <= u32::MAX as u64 {
        let h = f32::from_bits(bits as u32);
        src[0] = h;
        unsafe { (c.f)(c_out.as_mut_ptr(), src.as_ptr()) };
        for r in rust {
            unsafe { (r.f)(r_out.as_mut_ptr(), src.as_ptr()) };
            if c_out[0].to_bits() != r_out[0].to_bits()
                || c_out[1].to_bits() != r_out[1].to_bits()
                || c_out[2].to_bits() != r_out[2].to_bits()
            {
                panic!(
                    "DIVERGENCE in exhaustive hue sweep ({} vs c)\n  \
                     h = {:#010x}  s = {:#010x}  l = {:#010x}\n  \
                     C   : {:#010x} {:#010x} {:#010x}\n  \
                     Rust: {:#010x} {:#010x} {:#010x}",
                    r.name,
                    bits as u32,
                    s.to_bits(),
                    l.to_bits(),
                    c_out[0].to_bits(),
                    c_out[1].to_bits(),
                    c_out[2].to_bits(),
                    r_out[0].to_bits(),
                    r_out[1].to_bits(),
                    r_out[2].to_bits(),
                );
            }
        }
        n += 1;
        bits += stride;
    }
    n
}

/// Default configuration: `s = 1`, `l = 0.5` makes `c = 1` and `m = 0`, so the
/// three stored words are a direct read-out of the selected branch and of `x`
/// — i.e. of `fmodf`'s result — with nothing to mask a difference.
#[test]
fn exhaustive_hue_sweep_c_is_one_m_is_zero() {
    let stride = env_u64("HSL_SWEEP_STRIDE", 4096).max(1);
    let offset = env_u64("HSL_SWEEP_OFFSET", 0) % stride;
    let n = sweep(1.0, 0.5, stride, offset);
    eprintln!("swept {n} hue bit patterns (stride {stride}, offset {offset})");
    assert!(n > 0);
}

/// A second pass with a saturation/lightness pair that makes `c` a non-trivial
/// value (so `x = c * term` exercises the rounding of the product, not just the
/// term itself) and `m` non-zero (so `x + m` rounds too).
#[test]
fn exhaustive_hue_sweep_awkward_c_and_m() {
    let stride = env_u64("HSL_SWEEP_STRIDE", 4096).max(1);
    let offset = env_u64("HSL_SWEEP_OFFSET", 0) % stride;
    // 1 - |2*0.3 - 1| = 0.4 (inexact in binary), s = 0.7 (inexact) => c inexact.
    let n = sweep(0.7, 0.3, stride, offset);
    eprintln!("swept {n} hue bit patterns (stride {stride}, offset {offset})");
    assert!(n > 0);
}

/// And one with an infinite saturation, so `c`/`m`/`x` are NaN and the sweep
/// checks NaN propagation for every possible hue rather than only the value path.
#[test]
fn exhaustive_hue_sweep_nan_chroma() {
    let stride = env_u64("HSL_SWEEP_STRIDE", 4096).max(1);
    let offset = env_u64("HSL_SWEEP_OFFSET", 0) % stride;
    let n = sweep(f32::INFINITY, 0.0, stride, offset);
    eprintln!("swept {n} hue bit patterns (stride {stride}, offset {offset})");
    assert!(n > 0);
}

/// Exhaustive over *saturation* and *lightness* instead: all 2^32 patterns of
/// `s` with a fixed hue and lightness, and likewise for `l`. These do not touch
/// `fmodf` but they do cover every possible chroma/midpoint bit pattern.
#[test]
fn exhaustive_saturation_sweep() {
    let stride = env_u64("HSL_SWEEP_STRIDE", 4096).max(1);
    let offset = env_u64("HSL_SWEEP_OFFSET", 0) % stride;
    let c = c_lib();
    let rust = rust_libs();
    let mut n = 0u64;
    // One hue per branch of the dispatch chain, so the sweep covers all seven
    // outcomes rather than only sector B1.
    let hues: &[f32] = &[30.0, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0, -30.0, f32::NAN];
    let hues = if single_config() { &hues[..1] } else { hues };
    for &h in hues {
        let mut src = [h, 0.0, 0.375f32];
        let mut c_out = [0.0f32; 3];
        let mut r_out = [0.0f32; 3];
        let mut bits = offset;
        while bits <= u32::MAX as u64 {
            src[1] = f32::from_bits(bits as u32);
            unsafe { (c.f)(c_out.as_mut_ptr(), src.as_ptr()) };
            for r in rust {
                unsafe { (r.f)(r_out.as_mut_ptr(), src.as_ptr()) };
                assert!(
                    c_out[0].to_bits() == r_out[0].to_bits()
                        && c_out[1].to_bits() == r_out[1].to_bits()
                        && c_out[2].to_bits() == r_out[2].to_bits(),
                    "DIVERGENCE in exhaustive saturation sweep ({} vs c): h={:#010x} s={:#010x} l={:#010x}\n  C   : {:#010x} {:#010x} {:#010x}\n  Rust: {:#010x} {:#010x} {:#010x}",
                    r.name,
                    h.to_bits(),
                    bits as u32,
                    src[2].to_bits(),
                    c_out[0].to_bits(),
                    c_out[1].to_bits(),
                    c_out[2].to_bits(),
                    r_out[0].to_bits(),
                    r_out[1].to_bits(),
                    r_out[2].to_bits(),
                );
            }
            n += 1;
            bits += stride;
        }
    }
    eprintln!("swept {n} saturation bit patterns (stride {stride}, offset {offset})");
}

#[test]
fn exhaustive_lightness_sweep() {
    let stride = env_u64("HSL_SWEEP_STRIDE", 4096).max(1);
    let offset = env_u64("HSL_SWEEP_OFFSET", 0) % stride;
    let c = c_lib();
    let rust = rust_libs();
    let mut n = 0u64;
    let hues: &[f32] = &[30.0, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0, -30.0, f32::NAN];
    let sats: &[f32] = &[0.75, -0.75, f32::INFINITY];
    let hues = if single_config() { &hues[..1] } else { hues };
    let sats = if single_config() { &sats[..1] } else { sats };
    for &h in hues {
        for &s in sats {
            let mut src = [h, s, 0.0];
            let mut c_out = [0.0f32; 3];
            let mut r_out = [0.0f32; 3];
            let mut bits = offset;
            while bits <= u32::MAX as u64 {
                src[2] = f32::from_bits(bits as u32);
                unsafe { (c.f)(c_out.as_mut_ptr(), src.as_ptr()) };
                for r in rust {
                    unsafe { (r.f)(r_out.as_mut_ptr(), src.as_ptr()) };
                    assert!(
                        c_out[0].to_bits() == r_out[0].to_bits()
                            && c_out[1].to_bits() == r_out[1].to_bits()
                            && c_out[2].to_bits() == r_out[2].to_bits(),
                        "DIVERGENCE in exhaustive lightness sweep ({} vs c): h={:#010x} s={:#010x} l={:#010x}\n  C   : {:#010x} {:#010x} {:#010x}\n  Rust: {:#010x} {:#010x} {:#010x}",
                        r.name,
                        h.to_bits(),
                        s.to_bits(),
                        bits as u32,
                        c_out[0].to_bits(),
                        c_out[1].to_bits(),
                        c_out[2].to_bits(),
                        r_out[0].to_bits(),
                        r_out[1].to_bits(),
                        r_out[2].to_bits(),
                    );
                }
                n += 1;
                bits += stride;
            }
        }
    }
    eprintln!("swept {n} lightness bit patterns (stride {stride}, offset {offset})");
}
