use std::io::{self, Read, Write, BufWriter};

fn fma_array(out: &mut [i32], len: usize) {
    // Original C: fma_array(out, out, out, out, len) — all four pointers alias.
    // Each iteration writes only out[i] using mul1[i], mul2[i], add[i] (all out[i]).
    // So this is equivalent to: out[i] = out[i] * out[i] + out[i]
    for i in 0..len {
        let v = out[i];
        out[i] = v.wrapping_mul(v).wrapping_add(v);
    }
}

fn driver<W: Write>(out: &mut [i32], len: usize, w: &mut W) {
    fma_array(out, len);
    for i in 0..len {
        writeln!(w, "{}", out[i]).unwrap();
    }
}

/// Mimic C's scanf("%d", ...) for reading a single integer.
/// Returns Some(value) on success, None on EOF/no match.
/// Advances `pos` past any consumed bytes. On match failure (e.g., a non-numeric
/// character after skipping whitespace), `pos` is left at that character.
fn scanf_int(buf: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace (matches C's isspace for "%d": space, \t, \n, \v, \f, \r).
    while *pos < buf.len() {
        let c = buf[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\x0b' || c == b'\x0c' || c == b'\r' {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= buf.len() {
        return None;
    }

    let start = *pos;
    let mut sign: i64 = 1;
    if buf[*pos] == b'+' {
        *pos += 1;
    } else if buf[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }

    let digits_start = *pos;
    let mut value: i64 = 0;
    let mut overflowed = false;
    while *pos < buf.len() {
        let c = buf[*pos];
        if c.is_ascii_digit() {
            let d = (c - b'0') as i64;
            if !overflowed {
                value = value.saturating_mul(10).saturating_add(d);
                // Detect i32 overflow with sign applied
                let signed = sign * value;
                if signed > i32::MAX as i64 || signed < i32::MIN as i64 {
                    // Continue consuming digits but clamp at i32 wrap.
                    overflowed = true;
                }
            }
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos == digits_start {
        // No digits consumed → match failure. Per scanf, leave the input pointer
        // at the offending character (we keep `*pos` as-is, but rewind any
        // consumed sign).
        *pos = start;
        return None;
    }

    let signed = sign * value;
    // Truncate to i32 like C does (wrap on overflow).
    Some(signed as i32)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let stdout = io::stdout();
    let mut out_w = BufWriter::new(stdout.lock());

    let mut data: [i32; 100] = [0; 100];
    let mut pos = 0usize;
    let mut i: usize = 0;
    while i < 100 {
        match scanf_int(&input, &mut pos) {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    driver(&mut data, i, &mut out_w);
}
