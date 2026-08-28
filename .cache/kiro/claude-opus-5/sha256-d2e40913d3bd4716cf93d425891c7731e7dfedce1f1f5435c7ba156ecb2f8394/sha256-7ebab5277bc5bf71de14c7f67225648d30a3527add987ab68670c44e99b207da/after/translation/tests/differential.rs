//! Differential tests: every call goes through the `.so` FFI boundary for
//! both the C reference and the Rust translation.

mod common;

use common::Libs;

/// Format helper for failure messages.
fn describe(bits: u32) -> String {
    format!("bits=0x{:08x} (f32={:e})", bits, f32::from_bits(bits))
}

/// Hand-picked and structurally interesting inputs, exercising every distinct
/// region of the `m__base` / `m__shift` tables: zeros, subnormals, the
/// subnormal->normal half boundary, normals, overflow to infinity, infinities
/// and NaNs, for both signs.
fn interesting_bit_patterns() -> Vec<u32> {
    let mut v: Vec<u32> = Vec::new();

    // Every possible table index j (top 9 bits), combined with a spread of
    // mantissa patterns. This covers all 512 table entries.
    let mantissas: [u32; 12] = [
        0x000000, 0x000001, 0x000002, 0x00000f, 0x001000, 0x0fffff, 0x400000,
        0x555555, 0x7fffff, 0x7ffffe, 0x123456, 0x2aaaaa,
    ];
    for j in 0u32..512 {
        for m in mantissas {
            v.push((j << 23) | m);
        }
    }

    // Named values.
    let named: [f32; 26] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        1.0 / 3.0,
        65504.0,   // largest finite half
        -65504.0,
        65520.0,   // rounds/truncates at the half overflow boundary
        65536.0,
        6.1035156e-5,  // smallest normal half
        6.0975552e-5,  // largest subnormal half
        5.9604645e-8,  // smallest subnormal half
        2.9802322e-8,  // half of the smallest subnormal half
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::EPSILON,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        core::f32::consts::PI,
    ];
    for f in named {
        v.push(f.to_bits());
    }

    // Subnormal floats and other extremes by bit pattern.
    for extra in [
        0x0000_0001u32,
        0x0000_00ff,
        0x007f_ffff,
        0x0080_0000,
        0x8000_0001,
        0x807f_ffff,
        0x8080_0000,
        0x7f7f_ffff,
        0x7f80_0000,
        0x7f80_0001,
        0x7fc0_0000,
        0x7fff_ffff,
        0xff80_0000,
        0xffc0_0000,
        0xffff_ffff,
    ] {
        v.push(extra);
    }

    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn float2half_matches_on_interesting_inputs() {
    let libs = Libs::load();
    let c = libs.c_float2half();
    let r = libs.rust_float2half();

    for bits in interesting_bit_patterns() {
        let x = f32::from_bits(bits);
        let cv = unsafe { c(x) };
        let rv = unsafe { r(x) };
        assert_eq!(cv, rv, "mismatch for {}: C=0x{:04x} Rust=0x{:04x}", describe(bits), cv, rv);
    }
}

/// Sweep every mantissa value for a selection of exponents/signs, which
/// pins down the per-index shift amount exactly.
#[test]
fn float2half_matches_full_mantissa_sweeps() {
    let libs = Libs::load();
    let c = libs.c_float2half();
    let r = libs.rust_float2half();

    // Indices covering: all-zero base, the subnormal ramp (shift 0x17..0x0e),
    // the normal range (shift 0x0d), and the saturated/inf region, for both
    // signs.
    let mut indices: Vec<u32> = Vec::new();
    for j in 96u32..=160 {
        indices.push(j);
        indices.push(j + 256);
    }
    for j in [0u32, 1, 200, 255, 256, 257, 456, 511] {
        indices.push(j);
    }

    for j in indices {
        for m in 0u32..0x0080_0000 {
            let bits = (j << 23) | m;
            let x = f32::from_bits(bits);
            let cv = unsafe { c(x) };
            let rv = unsafe { r(x) };
            if cv != rv {
                panic!(
                    "mismatch for {}: C=0x{:04x} Rust=0x{:04x}",
                    describe(bits),
                    cv,
                    rv
                );
            }
        }
    }
}

/// Exhaustive sweep over the entire 2^32 input space. Enabled by default;
/// set `SKIP_EXHAUSTIVE=1` to skip it.
#[test]
fn float2half_matches_exhaustively() {
    if std::env::var_os("SKIP_EXHAUSTIVE").is_some() {
        eprintln!("SKIP_EXHAUSTIVE set; skipping exhaustive sweep");
        return;
    }

    let libs = Libs::load();
    let c = libs.c_float2half();
    let r = libs.rust_float2half();
    let cf: common::Float2Half = *c;
    let rf: common::Float2Half = *r;

    let mut bits: u32 = 0;
    loop {
        let x = f32::from_bits(bits);
        let cv = unsafe { cf(x) };
        let rv = unsafe { rf(x) };
        if cv != rv {
            panic!(
                "mismatch for {}: C=0x{:04x} Rust=0x{:04x}",
                describe(bits),
                cv,
                rv
            );
        }
        if bits == u32::MAX {
            break;
        }
        bits += 1;
    }
}
