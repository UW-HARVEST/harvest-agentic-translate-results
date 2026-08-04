use std::io::{self, BufRead};

mod lib;
use lib::*;

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
        0 => perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        _ => f32::NAN,
    }
}

fn main() {
    let stdin = io::stdin();
    let line = stdin.lock().lines().next().unwrap().unwrap();
    let parts: Vec<&str> = line.split_whitespace().collect();
    
    let which: i32 = parts[0].parse().unwrap();
    let x: f32 = parts[1].parse().unwrap();
    let y: f32 = parts[2].parse().unwrap();
    let z: f32 = parts[3].parse().unwrap();
    let x_wrap: i32 = parts[4].parse().unwrap();
    let y_wrap: i32 = parts[5].parse().unwrap();
    let z_wrap: i32 = parts[6].parse().unwrap();
    let seed: i32 = parts[7].parse().unwrap();
    let lacunarity: f32 = parts[8].parse().unwrap();
    let gain: f32 = parts[9].parse().unwrap();
    let offset: f32 = parts[10].parse().unwrap();
    let octaves: i32 = parts[11].parse().unwrap();
    
    let res = inner(
        which,
        x,
        y,
        z,
        x_wrap,
        y_wrap,
        z_wrap,
        seed,
        lacunarity,
        gain,
        offset,
        octaves,
    );
    println!("{:.9}", res);
}
