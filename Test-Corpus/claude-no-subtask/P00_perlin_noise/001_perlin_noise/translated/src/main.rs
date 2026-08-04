use std::io::{self, Read, Write};

mod perlin;

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
        0 => perlin::noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => perlin::noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => perlin::ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => perlin::fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => perlin::turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => perlin::noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        _ => f32::NAN,
    }
}

/// A scanf-like token reader: reads whitespace-separated tokens from stdin.
/// Whitespace includes spaces, tabs, newlines, etc. Mirrors C's scanf("%d"/"%f")
/// which skips leading whitespace then reads characters until whitespace.
struct TokenReader {
    buf: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn new() -> io::Result<Self> {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        Ok(Self { buf, pos: 0 })
    }

    fn next_token(&mut self) -> Option<&str> {
        // skip whitespace
        while self.pos < self.buf.len() && (self.buf[self.pos] as char).is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.buf.len() && !(self.buf[self.pos] as char).is_ascii_whitespace() {
            self.pos += 1;
        }
        // Should be valid ASCII for numeric tokens; assume utf8 ok.
        std::str::from_utf8(&self.buf[start..self.pos]).ok()
    }

    fn next_i32(&mut self) -> i32 {
        match self.next_token() {
            Some(t) => t.parse::<i32>().unwrap_or(0),
            None => 0,
        }
    }

    fn next_f32(&mut self) -> f32 {
        match self.next_token() {
            Some(t) => t.parse::<f32>().unwrap_or(0.0),
            None => 0.0,
        }
    }
}

/// Format a float like C's printf("%.9g", v) — delegates to snprintf for
/// byte-identical output (including locale-specific NaN/Inf formatting like
/// "-nan" on glibc).
fn format_g(v: f32, precision: usize) -> String {
    use std::os::raw::{c_char, c_double, c_int};
    extern "C" {
        fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    }

    // "%.<precision>g\0"
    let fmt = format!("%.{}g\0", precision);
    // Buffer size large enough for any %g output.
    let mut buf = vec![0u8; 64];
    // C's printf("%g", float) promotes float to double.
    let n = unsafe {
        snprintf(
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            fmt.as_ptr() as *const c_char,
            v as c_double,
        )
    };
    if n < 0 {
        return String::new();
    }
    let len = (n as usize).min(buf.len() - 1);
    String::from_utf8_lossy(&buf[..len]).into_owned()
}

fn main() {
    let mut reader = match TokenReader::new() {
        Ok(r) => r,
        Err(_) => return,
    };

    // Match C scanf order: %d %f %f %f %d %d %d %d %f %f %f %d
    let which = reader.next_i32();
    let x = reader.next_f32();
    let y = reader.next_f32();
    let z = reader.next_f32();
    let x_wrap = reader.next_i32();
    let y_wrap = reader.next_i32();
    let z_wrap = reader.next_i32();
    let seed = reader.next_i32();
    let lacunarity = reader.next_f32();
    let gain = reader.next_f32();
    let offset = reader.next_f32();
    let octaves = reader.next_i32();

    let res = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", format_g(res, 9));
}
