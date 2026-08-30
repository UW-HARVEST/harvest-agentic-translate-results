mod cformat;
mod cscan;
mod sse;
mod stb_perlin;
mod tables;

use std::io::{Read, Write};

use cscan::Scanner;
use stb_perlin::*;

/// Kills this process with `sig`, the way the C program dies for the
/// corresponding undefined behaviour.
fn die_with_signal(sig: i32) -> ! {
    extern "C" {
        fn raise(sig: i32) -> i32;
        fn signal(sig: i32, handler: usize) -> usize;
    }
    // SAFETY: restoring the default disposition (SIG_DFL == 0) and then
    // signalling this process; both signals are fatal by default, so the call
    // to `raise` does not return. The Rust runtime installs a SIGSEGV handler
    // for stack-overflow reporting, which has to be removed first.
    unsafe {
        signal(sig, 0);
        raise(sig);
    }
    std::process::abort();
}

/// The C program dies from SIGFPE when its `%` operator overflows
/// (`INT_MIN % -1` traps on x86-64).
pub fn raise_sigfpe() -> ! {
    die_with_signal(8)
}

/// The C program dies from SIGSEGV when an out-of-bounds table index reaches
/// past the pages holding its static data.
pub fn raise_sigsegv() -> ! {
    die_with_signal(11)
}

#[allow(clippy::too_many_arguments)]
fn inner(
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
        5 => stb_perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        _ => f32::NAN,
    }
}

fn main() {
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

    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    let mut sc = Scanner::new(input);

    // scanf("%d%f%f%f%d%d%d%d%f%f%f%d", ...): conversions stop at the first
    // matching/input failure, leaving the remaining variables untouched.
    'scan: {
        match sc.scan_int() {
            Some(v) => which = v,
            None => break 'scan,
        }
        match sc.scan_f32() {
            Some(v) => x = v,
            None => break 'scan,
        }
        match sc.scan_f32() {
            Some(v) => y = v,
            None => break 'scan,
        }
        match sc.scan_f32() {
            Some(v) => z = v,
            None => break 'scan,
        }
        match sc.scan_int() {
            Some(v) => x_wrap = v,
            None => break 'scan,
        }
        match sc.scan_int() {
            Some(v) => y_wrap = v,
            None => break 'scan,
        }
        match sc.scan_int() {
            Some(v) => z_wrap = v,
            None => break 'scan,
        }
        match sc.scan_int() {
            Some(v) => seed = v,
            None => break 'scan,
        }
        match sc.scan_f32() {
            Some(v) => lacunarity = v,
            None => break 'scan,
        }
        match sc.scan_f32() {
            Some(v) => gain = v,
            None => break 'scan,
        }
        match sc.scan_f32() {
            Some(v) => offset = v,
            None => break 'scan,
        }
        match sc.scan_int() {
            Some(v) => octaves = v,
            None => break 'scan,
        }
    }

    let res = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    // printf("%.9g\n", res) -- res is promoted to double.
    let out = cformat::format_g(res as f64, 9);
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "{}", out);
    let _ = lock.flush();
}
