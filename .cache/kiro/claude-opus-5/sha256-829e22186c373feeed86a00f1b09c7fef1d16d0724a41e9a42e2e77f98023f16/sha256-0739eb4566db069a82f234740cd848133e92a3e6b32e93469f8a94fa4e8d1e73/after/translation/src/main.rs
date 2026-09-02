// Rust translation of c_src/src/main.c
//
// Behavior is reproduced exactly, including:
//   * `house_t house = {0};` zero-initializes the whole object (padding included),
//     then the three fields are assigned.
//   * `memcpy(raw, &house, sizeof(house))` copies the raw object representation,
//     which is then dumped byte-by-byte as lowercase 2-digit hex followed by "\n".
//   * `scanf("%d", &x)` on matching failure / EOF leaves `x` at its initial 0.
//
// No bugs are "fixed": the original prints the raw struct representation
// (including any padding bytes), and that is what this does.

use std::io::{self, Read, Write};

/// Mirror of the C `house_t`.
///
/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
///
/// With `repr(C)` this has the same size, alignment and field offsets as the C
/// struct: `floors` @ 0, `bedrooms` @ 4, `bathrooms` @ 8, total size 16 on the
/// LP64 targets the C is built for (there are no padding bytes there).
#[repr(C)]
#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

const HOUSE_SIZE: usize = std::mem::size_of::<House>();
const FLOORS_OFF: usize = 0;
const BEDROOMS_OFF: usize = 4;
const BATHROOMS_OFF: usize = 8;

impl House {
    /// Zero-initialized object representation, matching `house_t house = {0};`.
    fn zeroed_bytes() -> [u8; HOUSE_SIZE] {
        [0u8; HOUSE_SIZE]
    }

    /// Produce the object representation (what `memcpy` would copy out).
    /// Uses native endianness, exactly as the C memcpy would observe.
    fn to_object_representation(&self) -> [u8; HOUSE_SIZE] {
        let mut raw = Self::zeroed_bytes();
        let f = self.floors.to_ne_bytes();
        let b = self.bedrooms.to_ne_bytes();
        let w = self.bathrooms.to_ne_bytes();
        raw[FLOORS_OFF..FLOORS_OFF + f.len()].copy_from_slice(&f);
        raw[BEDROOMS_OFF..BEDROOMS_OFF + b.len()].copy_from_slice(&b);
        raw[BATHROOMS_OFF..BATHROOMS_OFF + w.len()].copy_from_slice(&w);
        raw
    }
}

/// `static void print_hex(unsigned char *p, int len)`
fn print_hex(out: &mut impl Write, p: &[u8], len: usize) {
    for i in 0..len {
        let _ = write!(out, "{:02x}", p[i]);
    }
    let _ = writeln!(out);
}

/// `void driver(int floors)`
fn driver(out: &mut impl Write, floors: i32) {
    // house_t house = {0};
    let mut house = House {
        floors: 0,
        bedrooms: 0,
        bathrooms: 0.0,
    };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    // char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));
    let raw: [u8; HOUSE_SIZE] = house.to_object_representation();

    // print_hex((unsigned char *)&raw, sizeof(raw));
    print_hex(out, &raw, raw.len());
}

/// Byte-at-a-time reader over stdin so that only the characters consumed by the
/// conversion are taken from the stream, like C's `scanf`.
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

    fn peek(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
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
            Ok(_) => {
                self.peeked = Some(buf[0]);
                Some(buf[0])
            }
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn bump(&mut self) {
        self.peeked = None;
    }

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }

    /// `scanf("%d", &out)`.
    ///
    /// Returns the number of successful assignments (1) or 0 on a matching
    /// failure; `out` is left untouched unless the conversion succeeds.
    /// Mirrors glibc: the digits are accumulated with `strtol` semantics
    /// (saturating at LONG_MIN/LONG_MAX on overflow) and the result is then
    /// stored into an `int`, i.e. truncated.
    fn scan_int(&mut self, out: &mut i32) -> i32 {
        // %d skips leading whitespace, including newlines.
        while let Some(c) = self.peek() {
            if (c as char).is_ascii_whitespace() {
                self.bump();
            } else {
                break;
            }
        }

        let mut negative = false;
        match self.peek() {
            Some(b'+') => self.bump(),
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            Some(_) => {}
            None => return -1, // input failure (EOF before any conversion)
        }

        let mut saw_digit = false;
        let mut acc: i64 = 0;
        let mut overflow = false;
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_digit() => {
                    self.bump();
                    saw_digit = true;
                    let d = i64::from(c - b'0');
                    if !overflow {
                        match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                            Some(v) => acc = v,
                            None => overflow = true,
                        }
                    }
                }
                Some(other) => {
                    // Push the non-matching character back, as scanf does.
                    self.unget(other);
                    break;
                }
                None => break,
            }
        }

        if !saw_digit {
            // Matching failure: the (possible) sign character was consumed by
            // the failed conversion; the destination is not modified.
            return 0;
        }

        let value: i64 = if overflow {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            -acc
        } else {
            acc
        };

        *out = value as i32; // truncation to int, as glibc does
        1
    }
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    let stdout = io::stdout();
    let mut out = stdout.lock();

    // int x = 0; scanf("%d", &x);
    let mut x: i32 = 0;
    let _ = scanner.scan_int(&mut x);

    driver(&mut out, x);

    let _ = out.flush();
}
