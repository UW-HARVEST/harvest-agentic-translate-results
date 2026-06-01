// Rust port of C stb_perlin.h driver. Output must be byte-identical to the C version.

use std::io::{self, Read};

mod perlin;
mod scanf_util;
mod printf_util;

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
        0 => perlin::stb_perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => perlin::stb_perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => perlin::stb_perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => perlin::stb_perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => perlin::stb_perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => perlin::stb_perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        _ => f32::NAN,
    }
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let mut sc = scanf_util::Scanner::new(&input);

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

    // Replicate scanf("%d%f%f%f%d%d%d%d%f%f%f%d", ...) - stop on first failure.
    // Variables keep their initial value (0) if not parsed.
    'parse: {
        match sc.read_i32() { Some(v) => which = v, None => break 'parse }
        match sc.read_f32() { Some(v) => x = v, None => break 'parse }
        match sc.read_f32() { Some(v) => y = v, None => break 'parse }
        match sc.read_f32() { Some(v) => z = v, None => break 'parse }
        match sc.read_i32() { Some(v) => x_wrap = v, None => break 'parse }
        match sc.read_i32() { Some(v) => y_wrap = v, None => break 'parse }
        match sc.read_i32() { Some(v) => z_wrap = v, None => break 'parse }
        match sc.read_i32() { Some(v) => seed = v, None => break 'parse }
        match sc.read_f32() { Some(v) => lacunarity = v, None => break 'parse }
        match sc.read_f32() { Some(v) => gain = v, None => break 'parse }
        match sc.read_f32() { Some(v) => offset = v, None => break 'parse }
        match sc.read_i32() { Some(v) => octaves = v, None => break 'parse }
    }

    let res = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    // C: printf("%.9g\n", res);  -- res is float, promoted to double.
    println!("{}", printf_util::format_g(res as f64, 9));
}
