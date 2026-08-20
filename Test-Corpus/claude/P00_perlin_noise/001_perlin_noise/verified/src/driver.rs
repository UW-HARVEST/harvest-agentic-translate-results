//! Translation of the driver in `c_src/src/main.c` (`inner` and `main`).
//!
//! The logic lives in its own module so both the `driver` binary and the
//! `cdylib` (which re-exports `inner`/`main` with C linkage, exactly like the
//! shared object built from `main.c`) share one implementation.

use std::io::Write;

use crate::cfmt;
use crate::cscan::Scanner;
use crate::stb_perlin::{
    stb_perlin_fbm_noise3, stb_perlin_noise3, stb_perlin_noise3_seed,
    stb_perlin_noise3_wrap_nonpow2, stb_perlin_ridge_noise3, stb_perlin_turbulence_noise3,
};

/// `float inner(int which, float x, float y, float z, int x_wrap, int y_wrap,
///              int z_wrap, int seed, float lacunarity, float gain,
///              float offset, int octaves)`
#[allow(clippy::too_many_arguments)]
pub fn inner(
    which: i32,
    x: f32,
    y: f32,
    z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: i32,
    lacunarity: f32,
    gain: f32,
    offset: f32,
    octaves: i32,
) -> f32 {
    match which {
        0 => stb_perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => stb_perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => stb_perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => stb_perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => stb_perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        // The prototype takes an `unsigned char` seed, so the `int` is truncated.
        5 => stb_perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        // `NAN` from <math.h>: a positive quiet NaN.
        _ => f32::NAN,
    }
}

/// Runs the body of C's `main` against the given reader/writer.
pub fn run<R: std::io::Read, W: Write>(reader: R, mut writer: W) -> i32 {
    let mut which: i32 = 0;
    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;
    let mut z: f32 = 0.0;
    let mut x_wrap: i32 = 0;
    let mut y_wrap: i32 = 0;
    let mut z_wrap: i32 = 0;
    let mut seed: i32 = 0;
    let mut lacunarity: f32 = 0.0;
    let mut gain: f32 = 0.0;
    let mut offset: f32 = 0.0;
    let mut octaves: i32 = 0;

    let mut scanner = Scanner::new(reader);

    // scanf("%d%f%f%f%d%d%d%d%f%f%f%d", ...): conversions are applied in order
    // and a matching failure leaves every remaining variable untouched.
    'scan: {
        match scanner.scan_int() {
            Some(v) => which = v,
            None => break 'scan,
        }
        match scanner.scan_float() {
            Some(v) => x = v,
            None => break 'scan,
        }
        match scanner.scan_float() {
            Some(v) => y = v,
            None => break 'scan,
        }
        match scanner.scan_float() {
            Some(v) => z = v,
            None => break 'scan,
        }
        match scanner.scan_int() {
            Some(v) => x_wrap = v,
            None => break 'scan,
        }
        match scanner.scan_int() {
            Some(v) => y_wrap = v,
            None => break 'scan,
        }
        match scanner.scan_int() {
            Some(v) => z_wrap = v,
            None => break 'scan,
        }
        match scanner.scan_int() {
            Some(v) => seed = v,
            None => break 'scan,
        }
        match scanner.scan_float() {
            Some(v) => lacunarity = v,
            None => break 'scan,
        }
        match scanner.scan_float() {
            Some(v) => gain = v,
            None => break 'scan,
        }
        match scanner.scan_float() {
            Some(v) => offset = v,
            None => break 'scan,
        }
        if let Some(v) = scanner.scan_int() {
            octaves = v;
        }
    }

    let res = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    // printf("%.9g\n", res) -- the float argument is promoted to double.
    let _ = writeln!(writer, "{}", cfmt::format_g(f64::from(res), 9));
    let _ = writer.flush();

    0
}

/// C's `int main(void)`.
pub fn c_main() -> i32 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run(stdin.lock(), stdout.lock())
}
