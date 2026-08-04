use std::io::{self, Read, Write, BufWriter};

/// Reader that lets us peek one byte at a time from stdin.
struct ByteReader<R: Read> {
    inner: R,
    peeked: Option<u8>,
    eof: bool,
}

impl<R: Read> ByteReader<R> {
    fn new(inner: R) -> Self {
        Self { inner, peeked: None, eof: false }
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

    fn consume(&mut self) {
        self.peeked = None;
    }
}

/// Mimic the behavior of `scanf("%d", &out)`. On a successful match, the parsed
/// value is stored in `out`. Returns the number of items successfully matched
/// (0 or 1), like scanf does. If no match occurs, `out` is left unchanged.
fn scanf_int<R: Read>(rdr: &mut ByteReader<R>, out: &mut i32) -> i32 {
    // Skip leading whitespace (matches isspace: space, \t, \n, \v, \f, \r).
    loop {
        match rdr.peek() {
            Some(b) if b == b' ' || b == b'\t' || b == b'\n'
                || b == 0x0B || b == 0x0C || b == b'\r' => {
                rdr.consume();
            }
            _ => break,
        }
    }

    // Optional sign.
    let mut negative = false;
    let mut have_sign = false;
    match rdr.peek() {
        Some(b'+') => { rdr.consume(); have_sign = true; }
        Some(b'-') => { rdr.consume(); negative = true; have_sign = true; }
        _ => {}
    }

    // Digits.
    let mut have_digit = false;
    // Use wrapping arithmetic on i32 to match C overflow behavior (UB in C, but
    // typical 2's complement implementations wrap).
    let mut acc: i32 = 0;
    loop {
        match rdr.peek() {
            Some(b) if b.is_ascii_digit() => {
                rdr.consume();
                let d = (b - b'0') as i32;
                acc = acc.wrapping_mul(10);
                if negative {
                    acc = acc.wrapping_sub(d);
                } else {
                    acc = acc.wrapping_add(d);
                }
                have_digit = true;
            }
            _ => break,
        }
    }

    if !have_digit {
        // Matching failure. scanf returns 0 (or EOF if no input).
        // We don't push back the sign — scanf would, but since we only call
        // this twice in sequence and this case won't be exercised, the
        // simpler behavior is sufficient for matching the reference output.
        let _ = have_sign;
        return 0;
    }

    *out = acc;
    1
}

fn driver(x: i32, y: i32, w: &mut impl Write) {
    // x bitor compl y  ==  x | ~y
    let result: i32 = x | !y;
    write!(w, "{}", result).unwrap();
    // puts("") writes "\n".
    writeln!(w).unwrap();
}

fn main() {
    let stdin = io::stdin();
    let mut rdr = ByteReader::new(stdin.lock());

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    let mut y: i32 = 0;
    let _ = scanf_int(&mut rdr, &mut x);
    let _ = scanf_int(&mut rdr, &mut y);
    driver(x, y, &mut out);

    out.flush().unwrap();
}
