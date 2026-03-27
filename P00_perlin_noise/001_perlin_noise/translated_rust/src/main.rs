use std::io::{self, Read};
use perlin_noise::{inner, format_g};

/// Reads whitespace-delimited tokens from stdin (matching C scanf behavior).
struct Scanner {
    tokens: Vec<String>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).unwrap();
        let tokens: Vec<String> = input.split_whitespace().map(String::from).collect();
        Self { tokens, pos: 0 }
    }
    fn next_i32(&mut self) -> i32 {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t.parse().unwrap()
    }
    fn next_f32(&mut self) -> f32 {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t.parse().unwrap()
    }
}

fn main() {
    let mut sc = Scanner::new();
    let which = sc.next_i32();
    let x = sc.next_f32();
    let y = sc.next_f32();
    let z = sc.next_f32();
    let x_wrap = sc.next_i32();
    let y_wrap = sc.next_i32();
    let z_wrap = sc.next_i32();
    let seed = sc.next_i32();
    let lacunarity = sc.next_f32();
    let gain = sc.next_f32();
    let offset = sc.next_f32();
    let octaves = sc.next_i32();
    let res = inner(which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves);
    println!("{}", format_g(res, 9));
}
