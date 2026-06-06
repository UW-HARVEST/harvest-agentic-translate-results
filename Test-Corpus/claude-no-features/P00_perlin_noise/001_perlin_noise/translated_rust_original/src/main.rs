use std::io::{self, Read, Write};

mod perlin;
mod printf_g;

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

struct Tokens {
    data: Vec<String>,
    idx: usize,
}

impl Tokens {
    fn from_stdin() -> Self {
        let mut s = String::new();
        let _ = io::stdin().read_to_string(&mut s);
        let data = s.split_ascii_whitespace().map(|t| t.to_string()).collect();
        Tokens { data, idx: 0 }
    }

    fn next_int(&mut self) -> Option<i32> {
        if self.idx >= self.data.len() {
            return None;
        }
        let t = &self.data[self.idx];
        self.idx += 1;
        // mimic scanf %d: parse signed integer
        t.parse::<i32>().ok()
    }

    fn next_float(&mut self) -> Option<f32> {
        if self.idx >= self.data.len() {
            return None;
        }
        let t = &self.data[self.idx];
        self.idx += 1;
        t.parse::<f32>().ok()
    }
}

fn main() {
    let mut tokens = Tokens::from_stdin();

    let which = tokens.next_int().unwrap_or(0);
    let x = tokens.next_float().unwrap_or(0.0);
    let y = tokens.next_float().unwrap_or(0.0);
    let z = tokens.next_float().unwrap_or(0.0);
    let x_wrap = tokens.next_int().unwrap_or(0);
    let y_wrap = tokens.next_int().unwrap_or(0);
    let z_wrap = tokens.next_int().unwrap_or(0);
    let seed = tokens.next_int().unwrap_or(0);
    let lacunarity = tokens.next_float().unwrap_or(0.0);
    let gain = tokens.next_float().unwrap_or(0.0);
    let offset = tokens.next_float().unwrap_or(0.0);
    let octaves = tokens.next_int().unwrap_or(0);

    let res = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    let formatted = printf_g::format_g(res as f64, 9);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", formatted);
}
