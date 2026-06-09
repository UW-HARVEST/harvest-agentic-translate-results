use std::io::{self, Read, Write};

fn encode_quant(uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32, lsbit: i32) -> i32 {
    let mut uni = uni;
    let mut uni1: i32 = uni.wrapping_add(1);
    let mut uni2: i32 = uni.wrapping_sub(1);

    if (uni ^ uni1) & (!7i32) != 0 {
        uni1 = uni;
    }
    if (uni ^ uni2) & (!7i32) != 0 {
        uni2 = uni;
    }

    if lsbit != 0 {
        if lsbit == 4 {
            uni &= !1i32;
            uni1 &= !1i32;
            uni2 &= !1i32;
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if (lsbit & 1) != 0 {
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            uni &= !1i32;
            uni1 &= !1i32;
            uni2 &= !1i32;
        }
    }

    let mut diff: i32 = (2i32.wrapping_mul(uni & 7).wrapping_add(1)).wrapping_mul(step) / 8;
    if (uni & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    let p0: i32 = pred.wrapping_add(diff);
    let mut d0: i32 = tgt.wrapping_sub(p0);
    d0 = d0 ^ (d0 >> 31);

    diff = (2i32.wrapping_mul(uni1 & 7).wrapping_add(1)).wrapping_mul(step) / 8;
    if (uni1 & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    let p1: i32 = pred.wrapping_add(diff);
    let mut d1: i32 = tgt.wrapping_sub(p1);
    d1 = d1 ^ (d1 >> 31);

    diff = (2i32.wrapping_mul(uni2 & 7).wrapping_add(1)).wrapping_mul(step) / 8;
    if (uni2 & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    let p2: i32 = pred.wrapping_add(diff);
    let mut d2: i32 = tgt.wrapping_sub(p2);
    d2 = d2 ^ (d2 >> 31);

    let mut d3: i32 = tgt2.wrapping_sub(p0);
    d3 = d3 ^ (d3 >> 31);
    d0 = d0.wrapping_add(d3 >> 5);

    d3 = tgt2.wrapping_sub(p1);
    d3 = d3 ^ (d3 >> 31);
    d1 = d1.wrapping_add(d3 >> 5);

    d3 = tgt2.wrapping_sub(p2);
    d3 = d3 ^ (d3 >> 31);
    d2 = d2.wrapping_add(d3 >> 5);

    if d1 < d0 {
        uni = uni1;
    }
    if d2 < d0 {
        uni = uni2;
    }
    uni
}

/// Reads whitespace-separated integers from stdin similar to scanf("%d", ...).
struct ScanfIntReader {
    buf: Vec<u8>,
    pos: usize,
}

impl ScanfIntReader {
    fn new() -> Self {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).ok();
        ScanfIntReader { buf, pos: 0 }
    }

    fn next_int(&mut self) -> Option<i32> {
        // Skip whitespace (matches isspace in C: space, tab, newline, vertical tab, form feed, carriage return)
        while self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        // Optional sign
        if self.buf[self.pos] == b'+' || self.buf[self.pos] == b'-' {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while self.pos < self.buf.len() && self.buf[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == digits_start {
            // No digits read; matching failure
            self.pos = start;
            return None;
        }
        let s = std::str::from_utf8(&self.buf[start..self.pos]).ok()?;
        // C's scanf wraps on overflow in undefined ways; use wrapping parse.
        match s.parse::<i64>() {
            Ok(v) => Some(v as i32),
            Err(_) => None,
        }
    }
}

fn main() {
    let mut reader = ScanfIntReader::new();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let uni = match reader.next_int() {
        Some(v) => v,
        None => return,
    };
    let step = match reader.next_int() {
        Some(v) => v,
        None => return,
    };
    let pred = match reader.next_int() {
        Some(v) => v,
        None => return,
    };
    let tgt = match reader.next_int() {
        Some(v) => v,
        None => return,
    };
    let tgt2 = match reader.next_int() {
        Some(v) => v,
        None => return,
    };
    let lsbit = match reader.next_int() {
        Some(v) => v,
        None => return,
    };

    let result = encode_quant(uni, step, pred, tgt, tgt2, lsbit);
    let _ = writeln!(out, "{}", result);
}
