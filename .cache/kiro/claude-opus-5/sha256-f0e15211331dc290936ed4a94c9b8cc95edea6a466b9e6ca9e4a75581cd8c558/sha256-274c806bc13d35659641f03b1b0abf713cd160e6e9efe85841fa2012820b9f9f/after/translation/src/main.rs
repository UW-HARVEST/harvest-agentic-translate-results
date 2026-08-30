//! Translation of `c_src/src/main.c`.

mod cfmt;
mod cscan;
mod sse;
mod stb_perlin;

use std::io::{Read, Write};

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
        0 => stb_perlin::noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => stb_perlin::noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => stb_perlin::ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => stb_perlin::fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => stb_perlin::turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => stb_perlin::noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        _ => f32::NAN,
    }
}

/// The C code ignores `scanf`'s return value, so any variable whose conversion
/// did not happen keeps its initial zero value.
struct Params {
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
}

fn scan_params(sc: &mut cscan::Scanner, p: &mut Params) {
    // "%d%f%f%f%d%d%d%d%f%f%f%d"
    match sc.scan_i32() {
        Some(v) => p.which = v,
        None => return,
    }
    match sc.scan_f32() {
        Some(v) => p.x = v,
        None => return,
    }
    match sc.scan_f32() {
        Some(v) => p.y = v,
        None => return,
    }
    match sc.scan_f32() {
        Some(v) => p.z = v,
        None => return,
    }
    match sc.scan_i32() {
        Some(v) => p.x_wrap = v,
        None => return,
    }
    match sc.scan_i32() {
        Some(v) => p.y_wrap = v,
        None => return,
    }
    match sc.scan_i32() {
        Some(v) => p.z_wrap = v,
        None => return,
    }
    match sc.scan_i32() {
        Some(v) => p.seed = v,
        None => return,
    }
    match sc.scan_f32() {
        Some(v) => p.lacunarity = v,
        None => return,
    }
    match sc.scan_f32() {
        Some(v) => p.gain = v,
        None => return,
    }
    match sc.scan_f32() {
        Some(v) => p.offset = v,
        None => return,
    }
    if let Some(v) = sc.scan_i32() {
        p.octaves = v;
    }
}

fn main() {
    let mut p = Params {
        which: 0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        x_wrap: 0,
        y_wrap: 0,
        z_wrap: 0,
        seed: 0,
        lacunarity: 0.0,
        gain: 0.0,
        offset: 0.0,
        octaves: 0,
    };

    let mut buf = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut buf);
    let mut sc = cscan::Scanner::new(buf);
    scan_params(&mut sc, &mut p);

    let res = inner(
        p.which,
        p.x,
        p.y,
        p.z,
        p.x_wrap,
        p.y_wrap,
        p.z_wrap,
        p.seed,
        p.lacunarity,
        p.gain,
        p.offset,
        p.octaves,
    );

    let out = format!("{}\n", cfmt::format_g9(res as f64));
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}
