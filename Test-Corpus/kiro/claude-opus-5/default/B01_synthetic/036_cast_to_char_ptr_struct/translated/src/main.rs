// Rust translation of c_src/src/main.c
//
// The C program fills a `house_t` struct and dumps its raw in-memory bytes as
// lowercase hex. That output therefore depends on the C ABI struct layout and
// on the platform's endianness. The layout reproduced here is the x86-64
// System V one used by the original program:
//
//     typedef struct {
//         int floors;       // offset 0, 4 bytes
//         int bedrooms;     // offset 4, 4 bytes
//         double bathrooms; // offset 8, 8 bytes
//     } house_t;            // sizeof == 16, alignof == 8
//
// `house_t house = {0}` zero-initializes the whole object (including any
// padding), so the byte image is modelled as a zeroed 16-byte buffer into which
// the individual fields are written little-endian.

use std::io::{self, Read, Write};

/// Total size of `house_t` (`sizeof(house_t)` on x86-64 SysV).
const HOUSE_SIZE: usize = 16;
/// Byte offset of `house_t::floors`.
const OFF_FLOORS: usize = 0;
/// Byte offset of `house_t::bedrooms`.
const OFF_BEDROOMS: usize = 4;
/// Byte offset of `house_t::bathrooms`.
const OFF_BATHROOMS: usize = 8;

/// In-memory image of `house_t`, kept as raw bytes so the hex dump below is
/// byte-identical to the C version.
struct House {
    bytes: [u8; HOUSE_SIZE],
}

impl House {
    /// Equivalent of `house_t house = {0};`
    fn zeroed() -> Self {
        House {
            bytes: [0u8; HOUSE_SIZE],
        }
    }

    fn set_floors(&mut self, v: i32) {
        self.bytes[OFF_FLOORS..OFF_FLOORS + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn set_bedrooms(&mut self, v: i32) {
        self.bytes[OFF_BEDROOMS..OFF_BEDROOMS + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn set_bathrooms(&mut self, v: f64) {
        self.bytes[OFF_BATHROOMS..OFF_BATHROOMS + 8].copy_from_slice(&v.to_le_bytes());
    }
}

/// `static void print_hex(unsigned char *p, int len)`
fn print_hex(out: &mut impl Write, p: &[u8], len: usize) {
    for i in 0..len {
        // printf("%02x", p[i]);
        let _ = write!(out, "{:02x}", p[i]);
    }
    // printf("\n");
    let _ = writeln!(out);
}

/// `void driver(int floors)`
fn driver(out: &mut impl Write, floors: i32) {
    let mut house = House::zeroed();
    house.set_floors(floors);
    house.set_bedrooms(3);
    house.set_bathrooms(2.0);
    print_hex(out, &house.bytes, HOUSE_SIZE);
}

/// Byte-at-a-time stdin reader, mirroring how `scanf` consumes the stream
/// (it happily crosses newlines while skipping leading whitespace).
struct ByteReader<R: Read> {
    inner: R,
    buf: [u8; 1],
    eof: bool,
}

impl<R: Read> ByteReader<R> {
    fn new(inner: R) -> Self {
        ByteReader {
            inner,
            buf: [0u8; 1],
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if self.eof {
            return None;
        }
        loop {
            match self.inner.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(self.buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }
}

/// C `isspace` for the default "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)`.
///
/// Returns `Some(value)` when a conversion happened (the C code would then have
/// assigned it), or `None` on EOF/matching failure, in which case the C code
/// leaves `x` untouched.
///
/// On integer overflow glibc's converter saturates at `LONG_MAX`/`LONG_MIN` and
/// then stores the truncated low 32 bits into the `int`; that is reproduced by
/// saturating in `i64` and casting with `as i32`.
fn scanf_i32<R: Read>(r: &mut ByteReader<R>) -> Option<i32> {
    // Skip leading whitespace.
    let mut cur = loop {
        match r.next_byte() {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return None, // EOF before any conversion
        }
    };

    // Optional sign.
    let mut negative = false;
    if cur == b'+' || cur == b'-' {
        negative = cur == b'-';
        cur = match r.next_byte() {
            Some(b) => b,
            None => return None, // matching failure
        };
    }

    if !cur.is_ascii_digit() {
        return None; // matching failure
    }

    // Accumulate decimal digits, saturating like strtol does.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = i64::from(cur - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            }) {
                Some(v) => acc = v,
                None => {
                    saturated = true;
                    acc = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        match r.next_byte() {
            Some(b) if b.is_ascii_digit() => cur = b,
            // Non-digit terminates the conversion. The C library pushes this
            // byte back onto the stream; nothing else is read here, so there is
            // no observable difference.
            _ => break,
        }
    }

    Some(acc as i32)
}

fn main() {
    let stdin = io::stdin();
    let mut reader = ByteReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // int x = 0;
    let mut x: i32 = 0;
    // scanf("%d", &x);
    if let Some(v) = scanf_i32(&mut reader) {
        x = v;
    }
    // driver(x);
    driver(&mut out, x);
    let _ = out.flush();
}
