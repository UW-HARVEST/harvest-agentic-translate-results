// Translated from c_src/src/main.c
use std::ffi::CString;
use std::io::{self, Read};
use std::os::raw::{c_char, c_double, c_int};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

// Read all of stdin and split into whitespace-separated tokens, mirroring
// scanf("%d%f...") which skips whitespace (including newlines).
struct TokenStream {
    data: String,
    pos: usize,
}

impl TokenStream {
    fn new() -> Self {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).unwrap_or(0);
        TokenStream { data: s, pos: 0 }
    }

    fn next_token(&mut self) -> Option<&str> {
        let bytes = self.data.as_bytes();
        // skip whitespace
        while self.pos < bytes.len() && bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.pos >= bytes.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < bytes.len() && !bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        Some(&self.data[start..self.pos])
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

fn main() {
    let mut ts = TokenStream::new();

    let which = ts.next_i32();
    let x = ts.next_f32();
    let y = ts.next_f32();
    let z = ts.next_f32();
    let x_wrap = ts.next_i32();
    let y_wrap = ts.next_i32();
    let z_wrap = ts.next_i32();
    let seed = ts.next_i32();
    let lacunarity = ts.next_f32();
    let gain = ts.next_f32();
    let offset = ts.next_f32();
    let octaves = ts.next_i32();

    let res = driver::inner_rs(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    // C: printf("%.9g\n", res); res is float promoted to double per C variadic rules.
    let fmt = CString::new("%.9g\n").unwrap();
    unsafe {
        printf(fmt.as_ptr(), res as c_double);
    }
}
