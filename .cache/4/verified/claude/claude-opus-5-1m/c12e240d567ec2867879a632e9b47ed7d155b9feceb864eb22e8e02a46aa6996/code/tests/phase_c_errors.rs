//! Phase C: one differential test per row of `ERRORS.md` (rows E1..E30 plus the
//! generic FFI boundary rows G1..G5).  Rows E31..E43 live in `driver_cli.rs`
//! because they are `scanf` rejections of the `main` driver.

mod common;

use common::{classify_nonpow2, Diff, Nonpow2Class, Rng, SPECIAL_F32, SPECIAL_INTS, SPECIAL_WRAPS};

/// The exact bit pattern of `NAN` from `<math.h>` on this target.
const C_NAN_BITS: u32 = 0x7fc0_0000;

/// E1..E4: `inner` rejects an out-of-range `which` with `NAN`.
#[test]
fn e1_e4_inner_which_out_of_range() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E1-E4 inner which out of range");
    for which in [-1, 6, 7, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, -6, 100] {
        let cv = unsafe { (c.inner)(which, 0.5, 0.5, 0.5, 0, 0, 0, 0, 2.0, 0.5, 1.0, 6) };
        let rv = unsafe { (r.inner)(which, 0.5, 0.5, 0.5, 0, 0, 0, 0, 2.0, 0.5, 1.0, 6) };
        d.check(format_args!("which={which}"), cv, rv);
        assert_eq!(
            cv.to_bits(),
            C_NAN_BITS,
            "C must return the positive quiet NAN for which={which}"
        );
        assert_eq!(
            rv.to_bits(),
            C_NAN_BITS,
            "Rust must return the same NAN sentinel for which={which}"
        );
    }
    d.finish();
}

/// E5 / G2: any `int` value with no matching `case` must yield the same
/// sentinel -- C enums accept every `int`, so this sweeps the whole range.
#[test]
fn e5_inner_which_random_out_of_range() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E5 inner random out-of-range which");
    let mut rng = Rng::new(0xE5);
    for _ in 0..20000 {
        let which = loop {
            let w = rng.next_i32();
            if !(0..=5).contains(&w) {
                break w;
            }
        };
        let (x, y, z) = (rng.coord(8), rng.coord(8), rng.coord(8));
        let cv = unsafe { (c.inner)(which, x, y, z, 4, 8, 16, 3, 2.0, 0.5, 1.0, 4) };
        let rv = unsafe { (r.inner)(which, x, y, z, 4, 8, 16, 3, 2.0, 0.5, 1.0, 4) };
        d.check(format_args!("which={which}"), cv, rv);
        assert_eq!(cv.to_bits(), C_NAN_BITS, "which={which}");
    }
    d.finish();
}

/// E6..E11: `octaves <= 0` returns the `sum` initialiser (+0.0) from all three
/// fractal functions.
#[test]
fn e6_e11_octaves_non_positive() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E6-E11 octaves <= 0");
    let mut rng = Rng::new(0xE6);
    for octaves in [0, -1, -2, -100, i32::MIN, i32::MIN + 1, -65536] {
        for _ in 0..200 {
            let (x, y, z) = (rng.coord(16), rng.coord(16), rng.coord(16));
            let (lac, gain, offset) = (rng.lac_gain(), rng.lac_gain(), rng.lac_gain());
            let ridge_c = unsafe { (c.ridge)(x, y, z, lac, gain, offset, octaves) };
            let ridge_r = unsafe { (r.ridge)(x, y, z, lac, gain, offset, octaves) };
            let fbm_c = unsafe { (c.fbm)(x, y, z, lac, gain, octaves) };
            let fbm_r = unsafe { (r.fbm)(x, y, z, lac, gain, octaves) };
            let turb_c = unsafe { (c.turbulence)(x, y, z, lac, gain, octaves) };
            let turb_r = unsafe { (r.turbulence)(x, y, z, lac, gain, octaves) };
            d.check(format_args!("ridge octaves={octaves}"), ridge_c, ridge_r);
            d.check(format_args!("fbm octaves={octaves}"), fbm_c, fbm_r);
            d.check(format_args!("turbulence octaves={octaves}"), turb_c, turb_r);
            for (label, v) in [("ridge", ridge_c), ("fbm", fbm_c), ("turb", turb_c)] {
                assert_eq!(v.to_bits(), 0, "{label}: C must return +0.0 for octaves={octaves}");
            }
            for (label, v) in [("ridge", ridge_r), ("fbm", fbm_r), ("turb", turb_r)] {
                assert_eq!(v.to_bits(), 0, "{label}: Rust must return +0.0 for octaves={octaves}");
            }
        }
    }
    d.finish();
}

/// E12..E16: coordinates that `(int)` cannot represent (`NaN`, infinities,
/// magnitudes past 2^31) go through the `cvttss2si` indefinite value.
#[test]
fn e12_e16_fastfloor_out_of_int_range() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E12-E16 fastfloor out of int range");
    let bad = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::from_bits(0xffbf_ffff),
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e30,
        -1e30,
        2147483648.0,
        -2147483904.0,
        4294967296.0,
        f32::MAX,
        f32::MIN,
    ];
    let good = [0.5f32, -1.25, 7.0, 0.0];
    let mut rng = Rng::new(0xE12);
    for &b in &bad {
        for axis in 0..3 {
            for _ in 0..40 {
                let mut xyz = [
                    *rng.pick(&good),
                    *rng.pick(&good),
                    *rng.pick(&good),
                ];
                xyz[axis] = b;
                let [x, y, z] = xyz;
                let (xw, yw, zw) = (
                    *rng.pick(SPECIAL_WRAPS),
                    *rng.pick(SPECIAL_WRAPS),
                    *rng.pick(SPECIAL_WRAPS),
                );
                let s = rng.seed_u8();
                let ctx = format_args!(
                    "axis={axis} bits={:#010x} wraps=({xw},{yw},{zw}) seed={s}",
                    b.to_bits()
                );
                d.check(ctx, unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) }, unsafe {
                    (r.noise3_internal)(x, y, z, xw, yw, zw, s)
                });
                d.check(ctx, unsafe { (c.noise3)(x, y, z, xw, yw, zw) }, unsafe {
                    (r.noise3)(x, y, z, xw, yw, zw)
                });
                d.check(ctx, unsafe { (c.noise3_seed)(x, y, z, xw, yw, zw, 12345) }, unsafe {
                    (r.noise3_seed)(x, y, z, xw, yw, zw, 12345)
                });
                d.check(ctx, unsafe { (c.ridge)(x, y, z, 2.0, 0.5, 1.0, 4) }, unsafe {
                    (r.ridge)(x, y, z, 2.0, 0.5, 1.0, 4)
                });
                d.check(ctx, unsafe { (c.fbm)(x, y, z, 2.0, 0.5, 4) }, unsafe {
                    (r.fbm)(x, y, z, 2.0, 0.5, 4)
                });
                d.check(ctx, unsafe { (c.turbulence)(x, y, z, 2.0, 0.5, 4) }, unsafe {
                    (r.turbulence)(x, y, z, 2.0, 0.5, 4)
                });
                // The non-pow2 variant only when the indices stay in-window.
                if classify_nonpow2(x, y, z, 3, 5, 7, s) == Nonpow2Class::Reproducible {
                    d.check(ctx, unsafe { (c.wrap_nonpow2)(x, y, z, 3, 5, 7, s) }, unsafe {
                        (r.wrap_nonpow2)(x, y, z, 3, 5, 7, s)
                    });
                }
            }
        }
    }
    d.finish();
}

/// E17..E19: the mask `(wrap-1) & 255` at its edges -- `INT_MIN` (signed
/// overflow), non-powers of two and `wrap == 1`.
#[test]
fn e17_e19_wrap_mask_edges() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E17-E19 wrap mask edges");
    let mut rng = Rng::new(0xE17);
    for _ in 0..4000 {
        let (x, y, z) = (rng.coord(600), rng.coord(600), rng.coord(600));
        let s = rng.seed_u8();
        // E17: INT_MIN wraps -- `INT_MIN-1` overflows to INT_MAX, mask 255.
        let cv = unsafe { (c.noise3_internal)(x, y, z, i32::MIN, i32::MIN, i32::MIN, s) };
        let rv = unsafe { (r.noise3_internal)(x, y, z, i32::MIN, i32::MIN, i32::MIN, s) };
        d.check(format_args!("INT_MIN wraps x={x:e} y={y:e} z={z:e} seed={s}"), cv, rv);
        // ... and that is the same mask as `wrap = 0` / `wrap = 256`.
        let zero = unsafe { (c.noise3_internal)(x, y, z, 0, 0, 0, s) };
        assert_eq!(
            cv.to_bits(),
            zero.to_bits(),
            "INT_MIN must behave like mask 255 (x={x:e} y={y:e} z={z:e} seed={s})"
        );
        // E18: non-powers of two are accepted silently.
        let (xw, yw, zw) = (rng.range(1, 1000), rng.range(1, 1000), rng.range(1, 1000));
        d.check(
            format_args!("non-pow2 wraps=({xw},{yw},{zw})"),
            unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
        );
        // E19: wrap == 1 collapses every index to 0.
        d.check(
            format_args!("wrap==1 x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, 1, 1, 1, s) },
            unsafe { (r.noise3_internal)(x, y, z, 1, 1, 1, s) },
        );
    }
    d.finish();
}

/// E20/E21/G3: the `int` seed is truncated to `unsigned char`.
#[test]
fn e20_seed_truncation() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E20/E21 seed truncation");
    let mut rng = Rng::new(0xE20);
    for _ in 0..4000 {
        let (x, y, z) = (rng.coord(32), rng.coord(32), rng.coord(32));
        let seed = rng.next_i32();
        let (xw, yw, zw) = (
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
        );
        let cv = unsafe { (c.noise3_seed)(x, y, z, xw, yw, zw, seed) };
        let rv = unsafe { (r.noise3_seed)(x, y, z, xw, yw, zw, seed) };
        d.check(format_args!("seed={seed}"), cv, rv);
        // Truncation: the same low byte must give the same result.
        let same_byte = seed.wrapping_add(256 * rng.range(-1000, 1000));
        let cv2 = unsafe { (c.noise3_seed)(x, y, z, xw, yw, zw, same_byte) };
        d.check(format_args!("seed={seed} vs {same_byte}"), cv, cv2);
        // ... and it must equal the `unsigned char` entry point.
        d.check(
            format_args!("seed={seed} vs internal({})", seed as u8),
            cv,
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, seed as u8) },
        );
        // E21: the same truncation through `inner`'s case 5.
        let (xw2, yw2, zw2) = (rng.range(1, 256), rng.range(1, 256), rng.range(1, 256));
        if classify_nonpow2(x, y, z, xw2, yw2, zw2, seed as u8) == Nonpow2Class::Reproducible {
            d.check(
                format_args!("inner(5) seed={seed}"),
                unsafe { (c.inner)(5, x, y, z, xw2, yw2, zw2, seed, 0.0, 0.0, 0.0, 0) },
                unsafe { (r.wrap_nonpow2)(x, y, z, xw2, yw2, zw2, seed as u8) },
            );
        }
    }
    d.finish();
}

/// E22: a zero wrap means 256 in the non-pow2 variant.
#[test]
fn e22_nonpow2_zero_wrap_is_256() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E22 nonpow2 wrap 0 == 256");
    let mut rng = Rng::new(0xE22);
    for _ in 0..3000 {
        let (x, y, z) = (rng.coord(600), rng.coord(600), rng.coord(600));
        let s = rng.seed_u8();
        let zero = unsafe { (c.wrap_nonpow2)(x, y, z, 0, 0, 0, s) };
        let two56 = unsafe { (c.wrap_nonpow2)(x, y, z, 256, 256, 256, s) };
        assert_eq!(
            zero.to_bits(),
            two56.to_bits(),
            "C: wrap 0 must behave like 256 (x={x:e} y={y:e} z={z:e} seed={s})"
        );
        d.check(format_args!("wrap 0, x={x:e} y={y:e} z={z:e} seed={s}"), zero, unsafe {
            (r.wrap_nonpow2)(x, y, z, 0, 0, 0, s)
        });
        d.check(format_args!("wrap 256, x={x:e} y={y:e} z={z:e} seed={s}"), two56, unsafe {
            (r.wrap_nonpow2)(x, y, z, 256, 256, 256, s)
        });
        // Mixed: only one axis defaulted.
        let w = rng.range(1, 256);
        d.check(
            format_args!("mixed zero wrap w={w}"),
            unsafe { (c.wrap_nonpow2)(x, y, z, 0, w, 0, s) },
            unsafe { (r.wrap_nonpow2)(x, y, z, 0, w, 0, s) },
        );
    }
    d.finish();
}

/// E23: negative `px` is corrected by `x0 += x_wrap2`.
#[test]
fn e23_nonpow2_negative_px() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E23 nonpow2 negative px correction");
    let mut rng = Rng::new(0xE23);
    for _ in 0..8000 {
        // Strictly negative coordinates.
        let f = |rng: &mut Rng| -((rng.range(0, 5000) as f32) + 0.5);
        let (x, y, z) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let (xw, yw, zw) = (rng.range(1, 256), rng.range(1, 256), rng.range(1, 256));
        let s = rng.seed_u8();
        assert_eq!(
            classify_nonpow2(x, y, z, xw, yw, zw, s),
            Nonpow2Class::Reproducible
        );
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.wrap_nonpow2)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.wrap_nonpow2)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

/// E24: negative wrap with a non-negative `px` (C's `%` truncates towards 0).
#[test]
fn e24_nonpow2_negative_wrap_positive_px() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E24 nonpow2 negative wrap, px >= 0");
    let mut rng = Rng::new(0xE24);
    for _ in 0..8000 {
        let f = |rng: &mut Rng| (rng.range(0, 5000) as f32) + 0.25;
        let (x, y, z) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let (xw, yw, zw) = (-rng.range(1, 256), -rng.range(1, 256), -rng.range(1, 256));
        let s = rng.seed_u8();
        if classify_nonpow2(x, y, z, xw, yw, zw, s) != Nonpow2Class::Reproducible {
            continue;
        }
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.wrap_nonpow2)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.wrap_nonpow2)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

/// E25: an index in `512..1024` reads the gradient table that follows
/// `randtab` in `.data` -- deterministic in both C builds and modelled by the
/// Rust translation.
#[test]
fn e25_nonpow2_index_into_grad_table() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E25 nonpow2 index into the gradient table");
    let mut rng = Rng::new(0xE25);
    let mut in_grad_band = 0usize;
    for _ in 0..60000 {
        let wrap = rng.range(600, 1024);
        let px = rng.range(512, 1023);
        let x = px as f32 + 0.5;
        let (y, z) = (rng.coord(200) , rng.coord(200));
        let (yw, zw) = (rng.range(1, 256), rng.range(1, 256));
        let s = rng.seed_u8();
        if classify_nonpow2(x, y, z, wrap, yw, zw, s) != Nonpow2Class::Reproducible {
            continue;
        }
        // px in 512..1024 with a wrap above it means `randtab[x0]` reads the
        // gradient table.
        in_grad_band += 1;
        d.check(
            format_args!("wrap={wrap} px={px} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.wrap_nonpow2)(x, y, z, wrap, yw, zw, s) },
            unsafe { (r.wrap_nonpow2)(x, y, z, wrap, yw, zw, s) },
        );
    }
    assert!(
        in_grad_band > 100,
        "expected many reads from the gradient band, got {in_grad_band}"
    );
    d.finish();
}

/// E26/E27/E30: reads past the modelled window are not reproducible -- the C
/// executable and the C shared object built from the *same* source disagree, so
/// no implementation can match both.  The Rust library must stay memory-safe.
#[test]
fn e26_e27_deep_oob_is_not_reproducible() {
    let rust = common::rust_api();
    // (input for the driver, arguments for the shared-object probe)
    let cases: [(&str, [&str; 7]); 3] = [
        (
            "5 1.5 2.5 300.5 5 7 400 0 0 0 0 0",
            ["1.5", "2.5", "300.5", "5", "7", "400", "0"],
        ),
        (
            "5 1.5 2.5 500.5 5 7 1000 0 0 0 0 0",
            ["1.5", "2.5", "500.5", "5", "7", "1000", "0"],
        ),
        (
            "5 -299.5 2.5 3.5 -300 5 7 0 0 0 0 0",
            ["-299.5", "2.5", "3.5", "-300", "5", "7", "0"],
        ),
    ];
    let mut disagreements = 0;
    for (driver_input, probe_args) in cases {
        let exe = common::run_c_driver(driver_input);
        let args: Vec<String> = probe_args.iter().map(|s| s.to_string()).collect();
        let (status, bits) = common::run_probe(
            &common::c_so(),
            "stb_perlin_noise3_wrap_nonpow2",
            &args,
        );
        let exe_text = String::from_utf8_lossy(&exe.stdout).trim().to_string();
        // Rust stays memory-safe and returns a value instead of reading
        // whatever the linker happened to place behind `.data`.
        let v = unsafe {
            (rust.wrap_nonpow2)(
                probe_args[0].parse().unwrap(),
                probe_args[1].parse().unwrap(),
                probe_args[2].parse().unwrap(),
                probe_args[3].parse().unwrap(),
                probe_args[4].parse().unwrap(),
                probe_args[5].parse().unwrap(),
                probe_args[6].parse().unwrap(),
            )
        };
        // "The same C source, two builds": the executable from
        // c_src/CMakeLists.txt and the shared object.  `%.9g` of the executable
        // is compared with the exact value of the shared object.
        let exe_result = if exe.code == Some(0) && !exe_text.is_empty() {
            Some(exe_text.clone())
        } else {
            None
        };
        let so_result = bits.map(|b| format!("{:.9}", f32::from_bits(b)));
        let differ = match (&exe_result, bits) {
            (None, _) | (_, None) => true, // one of the builds crashed
            (Some(text), Some(b)) => {
                let exe_val: f32 = text.parse().unwrap_or(f32::NAN);
                // `%.9g` is lossless for a float, so unequal text means
                // unequal values.
                exe_val.to_bits() != b && !(exe_val.is_nan() && f32::from_bits(b).is_nan())
            }
        };
        println!(
            "deep OOB {driver_input:?}\n    C exe  -> {exe_result:?} (exit {:?})\n    C .so  -> {so_result:?} (exit {:?})\n    Rust   -> {:#010x}\n    builds differ: {differ}",
            exe.code,
            status.code(),
            v.to_bits(),
        );
        if differ {
            disagreements += 1;
        }
    }
    assert!(
        disagreements >= 2,
        "expected the two C builds to disagree on the deep-out-of-bounds inputs \
         (got {disagreements} of 3); without such a disagreement these rows would \
         have to be compared by value"
    );
}

/// E28: `INT_MIN % -1` traps in C (`SIGFPE`); the Rust translation returns.
#[test]
fn e28_int_min_mod_minus_one_traps_in_c() {
    use std::os::unix::process::ExitStatusExt;
    let args: Vec<String> = ["-2147483648.0", "2.5", "3.5", "-1", "5", "7", "0"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (status, bits) =
        common::run_probe(&common::c_so(), "stb_perlin_noise3_wrap_nonpow2", &args);
    assert_eq!(
        status.signal(),
        Some(libc::SIGFPE),
        "the C library must die with SIGFPE (got status {status:?}, bits {bits:?})"
    );
    // The very same call through the Rust .so returns a value.
    let rust = common::rust_api();
    let v = unsafe { (rust.wrap_nonpow2)(-2147483648.0, 2.5, 3.5, -1, 5, 7, 0) };
    println!("Rust .so returns {:#010x} where C traps", v.to_bits());
    // The C driver executable dies the same way.
    let exe = common::run_c_driver("5 -2147483648 2.5 3.5 -1 5 7 0 0 0 0 0");
    assert_eq!(exe.code, None, "the C executable must be killed by the trap");
}

/// E29: `x0 += INT_MIN` overflows; reproducible while the index stays inside
/// the modelled window (`px >= 0`).
#[test]
fn e29_nonpow2_int_min_wrap() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("E29 nonpow2 wrap = INT_MIN");
    let mut rng = Rng::new(0xE29);
    for px in 0..200 {
        let x = px as f32 + 0.5;
        let (y, z) = (rng.coord(100), rng.coord(100));
        let (yw, zw) = (rng.range(1, 256), rng.range(1, 256));
        let s = rng.seed_u8();
        assert_eq!(
            classify_nonpow2(x, y, z, i32::MIN, yw, zw, s),
            Nonpow2Class::Reproducible
        );
        d.check(
            format_args!("px={px} yw={yw} zw={zw} seed={s}"),
            unsafe { (c.wrap_nonpow2)(x, y, z, i32::MIN, yw, zw, s) },
            unsafe { (r.wrap_nonpow2)(x, y, z, i32::MIN, yw, zw, s) },
        );
    }
    // INT_MAX behaves the same way (no overflow, index == px).
    for px in 0..200 {
        let x = px as f32 + 0.5;
        let (y, z) = (rng.coord(100), rng.coord(100));
        let (yw, zw) = (rng.range(1, 256), rng.range(1, 256));
        let s = rng.seed_u8();
        if classify_nonpow2(x, y, z, i32::MAX, yw, zw, s) != Nonpow2Class::Reproducible {
            continue;
        }
        d.check(
            format_args!("INT_MAX wrap px={px} seed={s}"),
            unsafe { (c.wrap_nonpow2)(x, y, z, i32::MAX, yw, zw, s) },
            unsafe { (r.wrap_nonpow2)(x, y, z, i32::MAX, yw, zw, s) },
        );
    }
    d.finish();
}

/// G1: every `int` parameter at its extremes.
#[test]
fn g1_int_extremes() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("G1 int parameter extremes");
    let mut rng = Rng::new(0x6100);
    for &xw in SPECIAL_INTS {
        for &yw in SPECIAL_INTS {
            for &zw in SPECIAL_INTS {
                let (x, y, z) = (rng.coord(64), rng.coord(64), rng.coord(64));
                let seed = *rng.pick(SPECIAL_INTS);
                // `octaves` is an `int` too; huge positive values would loop for
                // hours in both implementations, so the upper end is probed
                // with the largest value that still runs quickly.
                let octaves = *rng.pick(&[0, 1, 2, 5, -1, i32::MIN, 300]);
                let ctx = format_args!("wraps=({xw},{yw},{zw}) seed={seed} octaves={octaves}");
                d.check(ctx, unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, seed as u8) }, unsafe {
                    (r.noise3_internal)(x, y, z, xw, yw, zw, seed as u8)
                });
                d.check(ctx, unsafe { (c.noise3)(x, y, z, xw, yw, zw) }, unsafe {
                    (r.noise3)(x, y, z, xw, yw, zw)
                });
                d.check(ctx, unsafe { (c.noise3_seed)(x, y, z, xw, yw, zw, seed) }, unsafe {
                    (r.noise3_seed)(x, y, z, xw, yw, zw, seed)
                });
                d.check(ctx, unsafe { (c.ridge)(x, y, z, 2.0, 0.5, 1.0, octaves) }, unsafe {
                    (r.ridge)(x, y, z, 2.0, 0.5, 1.0, octaves)
                });
                d.check(ctx, unsafe { (c.fbm)(x, y, z, 2.0, 0.5, octaves) }, unsafe {
                    (r.fbm)(x, y, z, 2.0, 0.5, octaves)
                });
                d.check(ctx, unsafe { (c.turbulence)(x, y, z, 2.0, 0.5, octaves) }, unsafe {
                    (r.turbulence)(x, y, z, 2.0, 0.5, octaves)
                });
            }
        }
    }
    d.finish();
}

/// G4: every `float` parameter at its special values.
#[test]
fn g4_float_extremes() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("G4 float parameter extremes");
    let mut rng = Rng::new(0xF4);
    for &a in SPECIAL_F32 {
        for &b in SPECIAL_F32 {
            let cc = *rng.pick(SPECIAL_F32);
            let ctx = format_args!(
                "a={:#010x} b={:#010x} c={:#010x}",
                a.to_bits(),
                b.to_bits(),
                cc.to_bits()
            );
            d.check(ctx, unsafe { (c.noise3_internal)(a, b, cc, 0, 0, 0, 7) }, unsafe {
                (r.noise3_internal)(a, b, cc, 0, 0, 0, 7)
            });
            d.check(ctx, unsafe { (c.ridge)(0.5, 0.25, 0.75, a, b, cc, 4) }, unsafe {
                (r.ridge)(0.5, 0.25, 0.75, a, b, cc, 4)
            });
            d.check(ctx, unsafe { (c.fbm)(0.5, 0.25, 0.75, a, b, 4) }, unsafe {
                (r.fbm)(0.5, 0.25, 0.75, a, b, 4)
            });
            d.check(ctx, unsafe { (c.turbulence)(0.5, 0.25, 0.75, a, b, 4) }, unsafe {
                (r.turbulence)(0.5, 0.25, 0.75, a, b, 4)
            });
            d.check(
                ctx,
                unsafe { (c.inner)(2, a, b, cc, 0, 0, 0, 0, a, b, cc, 3) },
                unsafe { (r.inner)(2, a, b, cc, 0, 0, 0, 0, a, b, cc, 3) },
            );
        }
    }
    d.finish();
}

/// G5: random 32-bit patterns reinterpreted as floats (signalling NaNs,
/// subnormals, everything).
#[test]
fn g5_random_bit_patterns() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("G5 random float bit patterns");
    let mut rng = Rng::new(0xF5);
    for _ in 0..20000 {
        let (x, y, z) = (rng.any_f32(), rng.any_f32(), rng.any_f32());
        let (lac, gain, offset) = (rng.any_f32(), rng.any_f32(), rng.any_f32());
        let (xw, yw, zw) = (
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
        );
        let s = rng.seed_u8();
        let octaves = rng.range(0, 5);
        let ctx = format_args!(
            "x={:#010x} y={:#010x} z={:#010x} lac={:#010x} gain={:#010x} offset={:#010x} wraps=({xw},{yw},{zw}) seed={s} octaves={octaves}",
            x.to_bits(), y.to_bits(), z.to_bits(), lac.to_bits(), gain.to_bits(), offset.to_bits()
        );
        d.check(ctx, unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) }, unsafe {
            (r.noise3_internal)(x, y, z, xw, yw, zw, s)
        });
        d.check(ctx, unsafe { (c.noise3)(x, y, z, xw, yw, zw) }, unsafe {
            (r.noise3)(x, y, z, xw, yw, zw)
        });
        d.check(ctx, unsafe { (c.noise3_seed)(x, y, z, xw, yw, zw, s as i32) }, unsafe {
            (r.noise3_seed)(x, y, z, xw, yw, zw, s as i32)
        });
        d.check(ctx, unsafe { (c.ridge)(x, y, z, lac, gain, offset, octaves) }, unsafe {
            (r.ridge)(x, y, z, lac, gain, offset, octaves)
        });
        d.check(ctx, unsafe { (c.fbm)(x, y, z, lac, gain, octaves) }, unsafe {
            (r.fbm)(x, y, z, lac, gain, octaves)
        });
        d.check(ctx, unsafe { (c.turbulence)(x, y, z, lac, gain, octaves) }, unsafe {
            (r.turbulence)(x, y, z, lac, gain, octaves)
        });
        if classify_nonpow2(x, y, z, xw, yw, zw, s) == Nonpow2Class::Reproducible {
            d.check(ctx, unsafe { (c.wrap_nonpow2)(x, y, z, xw, yw, zw, s) }, unsafe {
                (r.wrap_nonpow2)(x, y, z, xw, yw, zw, s)
            });
        }
    }
    d.finish();
}
