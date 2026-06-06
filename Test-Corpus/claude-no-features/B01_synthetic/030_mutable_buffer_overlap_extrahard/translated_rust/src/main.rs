use std::io::{self, Read, Write, BufWriter};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn driver(out: &mut [i32], len: usize, w: &mut impl Write) {
    // fma_array(out, out, out, out, len) — all aliasing the same buffer.
    // Replicate by reading current values and writing into out in-place.
    // Compute equivalent: out[i] = out[i]*out[i] + out[i]
    let snapshot: Vec<i32> = out[..len].to_vec();
    fma_array(out, &snapshot, &snapshot, &snapshot, len);
    for i in 0..len {
        writeln!(w, "{}", out[i]).unwrap();
    }
}

/// Mimic C's scanf("%d", &x) behavior:
/// - Skip leading whitespace (including newlines)
/// - Optional sign
/// - One or more decimal digits
/// Returns Some((value, bytes_consumed)) on success, None on failure (no match or EOF).
fn scan_int(buf: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < buf.len() {
        let c = buf[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos >= buf.len() {
        return None;
    }
    let start = *pos;
    let mut p = *pos;
    // Optional sign
    if p < buf.len() && (buf[p] == b'+' || buf[p] == b'-') {
        p += 1;
    }
    let digits_start = p;
    while p < buf.len() && buf[p].is_ascii_digit() {
        p += 1;
    }
    if p == digits_start {
        // No digits — match failure; restore pos to start
        *pos = start;
        return None;
    }
    let s = std::str::from_utf8(&buf[start..p]).ok()?;
    // Mimic C scanf: on overflow, behavior is undefined; we'll use wrapping parse
    // by parsing as i64 then casting (best effort).
    let val: i64 = s.parse().ok()?;
    *pos = p;
    Some(val as i32)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut data: [i32; 100] = [0; 100];
    let mut pos = 0usize;
    let mut i: usize = 0;
    while i < 100 {
        match scan_int(&input, &mut pos) {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    driver(&mut data, i, &mut out);
}
