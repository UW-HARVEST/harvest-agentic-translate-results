use std::io::{self, Read, Write};

#[repr(C)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for b in p {
        buf.push_str(&format!("{:02x}", b));
    }
    buf.push('\n');
    out.write_all(buf.as_bytes()).unwrap();
}

fn driver(floors: i32) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    // memcpy struct bytes into raw buffer
    let size = std::mem::size_of::<House>();
    let mut raw = vec![0u8; size];
    unsafe {
        let src = (&house as *const House) as *const u8;
        std::ptr::copy_nonoverlapping(src, raw.as_mut_ptr(), size);
    }
    print_hex(&raw);
}

/// Parse the first integer found in the input the way C's `scanf("%d", &x)` does.
/// scanf("%d") skips leading whitespace (including newlines), then reads an
/// optional sign and consecutive decimal digits. If parsing fails, the
/// destination is left unchanged (in main, x remains initialized to 0).
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip leading whitespace
    while i < input.len() && (input[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= input.len() {
        return 0;
    }
    let start = i;
    if input[i] == b'+' || input[i] == b'-' {
        i += 1;
    }
    let digits_start = i;
    while i < input.len() && (input[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        // No digits parsed; scanf would fail and not modify x
        return 0;
    }
    let s = std::str::from_utf8(&input[start..i]).unwrap_or("0");
    // C's scanf with %d would wrap on overflow per implementation; use
    // wrapping parse via i64 then truncate to i32 to mimic typical behavior.
    match s.parse::<i64>() {
        Ok(v) => v as i32,
        Err(_) => 0,
    }
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).ok();
    let x = scanf_int(&buf);
    driver(x);
}
