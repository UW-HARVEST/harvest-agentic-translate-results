mod stb_perlin;

use std::io::{self, Read};
use stb_perlin::{
    stb_perlin_fbm_noise3, stb_perlin_noise3, stb_perlin_noise3_seed,
    stb_perlin_noise3_wrap_nonpow2, stb_perlin_ridge_noise3, stb_perlin_turbulence_noise3,
};

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
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut it = input.split_whitespace();

    let which: i32 = it.next().unwrap_or("0").parse().unwrap_or(0);
    let x: f32 = it.next().unwrap_or("0").parse().unwrap_or(0.0);
    let y: f32 = it.next().unwrap_or("0").parse().unwrap_or(0.0);
    let z: f32 = it.next().unwrap_or("0").parse().unwrap_or(0.0);
    let x_wrap: i32 = it.next().unwrap_or("0").parse().unwrap_or(0);
    let y_wrap: i32 = it.next().unwrap_or("0").parse().unwrap_or(0);
    let z_wrap: i32 = it.next().unwrap_or("0").parse().unwrap_or(0);
    let seed: i32 = it.next().unwrap_or("0").parse().unwrap_or(0);
    let lacunarity: f32 = it.next().unwrap_or("0").parse().unwrap_or(0.0);
    let gain: f32 = it.next().unwrap_or("0").parse().unwrap_or(0.0);
    let offset: f32 = it.next().unwrap_or("0").parse().unwrap_or(0.0);
    let octaves: i32 = it.next().unwrap_or("0").parse().unwrap_or(0);

    let res = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );
    println!("{:.9}", res);
}
