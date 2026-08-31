// Rust translation of c_src/src/main.c
//
// Original C is Copyright 2025 MIT Lincoln Laboratory (MIT license); see
// c_src/src/main.c for the full notice. This translation reproduces the
// behavior of that program exactly, including its reliance on mutable global
// state that persists between the two `run()` invocations.

use std::cell::RefCell;
use std::io::{Read, Write};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

thread_local! {
    /// static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
    static THE_HOUSE: RefCell<House> = const {
        RefCell::new(House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        })
    };
}

fn add_floor(house: &mut House) {
    // house->floors++  (C signed overflow is UB; wrap to stay defined)
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // house->bedrooms += extra_bedrooms
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house() {
    THE_HOUSE.with(|h| add_floor(&mut h.borrow_mut()));
}

fn print_the_house<W: Write>(out: &mut W) {
    let h = THE_HOUSE.with(|h| *h.borrow());
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        h.floors,
        h.bedrooms,
        format_f64_1(h.bathrooms)
    );
}

/// Formats a double the way C's `printf("%.1f", v)` does.
fn format_f64_1(v: f64) -> String {
    if v.is_nan() {
        // glibc prints "nan" / "-nan" (sign bit honored) for %f.
        return if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if v.is_infinite() {
        return if v < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    // Rust's `{:.1}` rounds half away from zero on the exact decimal value of
    // the double, which matches glibc's round-half-away behavior for these
    // values. Negative zero keeps its sign in both.
    format!("{:.1}", v)
}

fn run<W: Write>(out: &mut W, extra_bedrooms: i32) {
    print_the_house(out);
    add_floor_to_the_house();
    print_the_house(out);
    THE_HOUSE.with(|h| h.borrow_mut().bathrooms += 1.0);
    print_the_house(out);
    THE_HOUSE.with(|h| add_bedrooms(&mut h.borrow_mut(), extra_bedrooms));
    print_the_house(out);
}

/// Byte-at-a-time stdin reader with one byte of pushback, so we consume no
/// more input than `scanf` would.
struct Scanner<R: Read> {
    inner: R,
    peeked: Option<u8>,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(inner: R) -> Self {
        Scanner {
            inner,
            peeked: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => Some(buf[0]),
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn push_back(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// Equivalent of `scanf("%d", &out)`. Returns true on a successful
    /// conversion (the C code leaves `x` untouched otherwise). Whitespace,
    /// including newlines, is skipped before the number.
    fn scan_i32(&mut self, out: &mut i32) -> bool {
        // %d skips leading whitespace, crossing newlines.
        let mut c = loop {
            match self.next_byte() {
                None => return false,
                Some(b) if is_space(b) => continue,
                Some(b) => break b,
            }
        };

        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            match self.next_byte() {
                None => return false,
                Some(b) => c = b,
            }
        }

        if !c.is_ascii_digit() {
            // Matching failure: push the offending byte back, leave *out alone.
            self.push_back(c);
            return false;
        }

        // Accumulate in i64 with saturation, then truncate to int, mirroring
        // glibc's wide internal accumulation for out-of-range input.
        let mut acc: i64 = 0;
        loop {
            let digit = (c - b'0') as i64;
            acc = acc
                .saturating_mul(10)
                .saturating_add(if negative { -digit } else { digit });
            match self.next_byte() {
                None => break,
                Some(b) if b.is_ascii_digit() => c = b,
                Some(b) => {
                    self.push_back(b);
                    break;
                }
            }
        }

        *out = acc as i32;
        true
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn main() {
    let stdin = std::io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    let _ = scanner.scan_i32(&mut x);

    run(&mut out, x);
    run(&mut out, x);

    let _ = out.flush();
}
