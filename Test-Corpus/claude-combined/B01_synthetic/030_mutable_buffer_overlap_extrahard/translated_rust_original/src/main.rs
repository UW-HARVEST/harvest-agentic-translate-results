use std::io::{self, Read, Write, BufWriter};

/// Reads bytes from stdin and parses one signed decimal integer matching scanf("%d").
/// Skips leading whitespace, optionally consumes a sign, then consumes digits.
/// Returns Some(value) on successful match, None on EOF/no match.
struct ScanfReader {
    buf: Vec<u8>,
    pos: usize,
}

impl ScanfReader {
    fn new() -> io::Result<Self> {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        Ok(ScanfReader { buf, pos: 0 })
    }

    fn peek(&self) -> Option<u8> {
        if self.pos < self.buf.len() {
            Some(self.buf[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            // C isspace() recognizes ' ', '\t', '\n', '\v', '\f', '\r'
            if matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Match scanf("%d", ...) -> returns Some(i32) if a value was successfully scanned.
    fn read_int(&mut self) -> Option<i32> {
        self.skip_whitespace();
        let start = self.pos;
        let mut sign: i64 = 1;
        if let Some(c) = self.peek() {
            if c == b'+' {
                self.advance();
            } else if c == b'-' {
                sign = -1;
                self.advance();
            }
        }
        let digits_start = self.pos;
        let mut value: i64 = 0;
        let mut any_digit = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                any_digit = true;
                let d = (c - b'0') as i64;
                // C scanf with %d into int silently truncates / has undefined behavior
                // on overflow; we mimic by wrapping at i32 width using i64 accumulation
                // and casting at the end.
                value = value.wrapping_mul(10).wrapping_add(d);
                self.advance();
            } else {
                break;
            }
        }
        if !any_digit {
            // No conversion happened; rewind position to before sign so future calls
            // see the same input. scanf actually leaves the offending char in the
            // stream, but pushes back nothing extra; the sign char however is
            // typically considered consumed only if a digit follows (i.e., the match
            // failure leaves stream pointing at the sign char). Restore position.
            self.pos = start;
            // Also signal no read occurred.
            // But if we hit EOF after skipping whitespace, return None.
            if digits_start >= self.buf.len() {
                return None;
            }
            return None;
        }
        let signed = sign.wrapping_mul(value);
        Some(signed as i32)
    }
}

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn driver<W: Write>(out: &mut [i32], len: usize, writer: &mut W) -> io::Result<()> {
    // Snapshot the inputs to mirror C semantics where mul1/mul2/add alias `out`.
    // In the C code, fma_array is called with the same pointer for all four
    // arguments. The loop reads out[i] (for mul1, mul2, add) before writing
    // out[i], so each element is computed as orig * orig + orig.
    let snapshot = out[..len].to_vec();
    fma_array(out, &snapshot, &snapshot, &snapshot, len);
    for i in 0..len {
        writeln!(writer, "{}", out[i])?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut reader = ScanfReader::new()?;
    let mut data: [i32; 100] = [0; 100];
    let mut i: usize = 0;
    while i < 100 {
        match reader.read_int() {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut data, i, &mut out)?;
    out.flush()?;
    Ok(())
}
