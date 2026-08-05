//! Deterministic random JSON generator (no external deps) shared by tests.
#![allow(dead_code)]

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    pub fn boolean(&mut self) -> bool {
        self.next() & 1 == 0
    }
}

fn rand_string(rng: &mut Rng) -> String {
    let n = rng.below(8);
    let mut s = String::new();
    let choices = [
        "a", "Z", "0", " ", "\"", "\\", "/", "\n", "\t", "\r", "é", "€", "😀", "\u{7f}",
        "\u{1}", "key", "b",
    ];
    for _ in 0..n {
        s.push_str(choices[rng.below(choices.len() as u64) as usize]);
    }
    s
}

fn json_escape_into(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Generate a random JSON text of bounded depth.
pub fn gen_json(rng: &mut Rng, depth: u32) -> String {
    let kind = if depth == 0 {
        rng.below(5) // scalars only
    } else {
        rng.below(7)
    };
    match kind {
        0 => {
            // integer
            let v = rng.next() as i64;
            format!("{}", v)
        }
        1 => {
            // real
            let mant = (rng.next() % 1_000_000) as f64;
            let exp = (rng.below(20) as i64) - 10;
            format!("{}e{}", mant, exp)
        }
        2 => {
            let mut s = String::new();
            json_escape_into(&mut s, &rand_string(rng));
            s
        }
        3 => "true".into(),
        4 => match rng.below(2) {
            0 => "false".into(),
            _ => "null".into(),
        },
        5 => {
            // array
            let n = rng.below(5);
            let mut s = String::from("[");
            for i in 0..n {
                if i > 0 {
                    s.push(',');
                }
                s.push_str(&gen_json(rng, depth - 1));
            }
            s.push(']');
            s
        }
        _ => {
            // object with unique keys
            let n = rng.below(5);
            let mut s = String::from("{");
            for i in 0..n {
                if i > 0 {
                    s.push(',');
                }
                let mut k = String::new();
                json_escape_into(&mut k, &format!("k{}_{}", i, rng.below(1000)));
                s.push_str(&k);
                s.push(':');
                s.push_str(&gen_json(rng, depth - 1));
            }
            s.push('}');
            s
        }
    }
}
