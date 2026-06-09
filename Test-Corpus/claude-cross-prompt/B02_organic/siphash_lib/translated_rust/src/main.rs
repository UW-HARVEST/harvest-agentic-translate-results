use std::io::{self, Read, Write, BufWriter};

// SipRound macro from C, operating on 64-bit unsigned (size_t on 64-bit systems).
#[inline(always)]
fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

fn stbds_siphash_bytes(p: &[u8], len: usize, seed: u64) -> u64 {
    // The C code uses size_t (u64 on 64-bit). Mimic exactly.
    // Constants:
    //   ((((size_t)0x736f6d65 << 16) << 16) + 0x70736575) = 0x736f6d6570736575
    //   ((((size_t)0x646f7261 << 16) << 16) + 0x6e646f6d) = 0x646f72616e646f6d
    //   ((((size_t)0x6c796765 << 16) << 16) + 0x6e657261) = 0x6c7967656e657261
    //   ((((size_t)0x74656462 << 16) << 16) + 0x79746573) = 0x7465646279746573
    let not_seed = !seed;
    let mut v0: u64 = 0x736f6d6570736575u64 ^ seed;
    let mut v1: u64 = 0x646f72616e646f6du64 ^ not_seed;
    let mut v2: u64 = 0x6c7967656e657261u64 ^ seed;
    let mut v3: u64 = 0x7465646279746573u64 ^ not_seed;
    v0 ^= 0x0706050403020100u64 ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 ^ not_seed;
    v2 ^= 0x0706050403020100u64 ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 ^ not_seed;

    let mut i: usize = 0;
    let size_of_size_t: usize = 8; // sizeof(size_t) on 64-bit
    let data: u64;

    // Main loop: process 8-byte chunks.
    // Reproduce C semantics including the implicit sign-extension when an int
    // expression with the high bit set is converted to size_t.
    while i + size_of_size_t <= len {
        // First 32-bit chunk: int value computed via OR of shifted unsigned chars
        // (each unsigned char promotes to int). If the byte d[3] has its high
        // bit set, (d[3] << 24) is a negative int, and assigning to size_t
        // sign-extends. We replicate that with `as i32 as i64 as u64`.
        let part1_i32: i32 = (p[i] as i32)
            | ((p[i + 1] as i32) << 8)
            | ((p[i + 2] as i32) << 16)
            | ((p[i + 3] as i32) << 24);
        let mut local_data: u64 = part1_i32 as i64 as u64;

        // Second 32-bit chunk: cast to size_t happens first in C, then shift
        // by 16 then 16 again. The cast (size_t)int_val sign-extends.
        let part2_i32: i32 = (p[i + 4] as i32)
            | ((p[i + 5] as i32) << 8)
            | ((p[i + 6] as i32) << 16)
            | ((p[i + 7] as i32) << 24);
        let part2_u64: u64 = part2_i32 as i64 as u64;
        local_data |= part2_u64.wrapping_shl(16).wrapping_shl(16);

        v3 ^= local_data;
        // Two rounds (j=0..2)
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= local_data;

        i += size_of_size_t;
    }

    // Trailing bytes processing.
    // data = len << ((sizeof(size_t)*8) - 8) = len << 56
    let mut tail: u64 = (len as u64).wrapping_shl(56);

    // Switch with fall-through (C semantics).
    let remaining = len - i;
    // We unroll using fall-through behavior.
    // Note: case 4 has `(d[3] << 24)` which is an int expression that becomes
    // negative if d[3] has its high bit set, then sign-extends to size_t.
    // Cases 5,6,7 use `(size_t)d[k] << X << X` so no sign extension issue.
    // Cases 1,2,3 produce non-negative int values, so no sign extension.
    // We model exactly.
    if remaining == 7 {
        tail |= ((p[i + 6] as u64) << 24) << 24;
        tail |= ((p[i + 5] as u64) << 20) << 20;
        tail |= ((p[i + 4] as u64) << 16) << 16;
        // case 4: (d[3] << 24) — int with possible sign extension
        tail |= ((p[i + 3] as i32).wrapping_shl(24)) as i64 as u64;
        tail |= ((p[i + 2] as i32) << 16) as i64 as u64;
        tail |= ((p[i + 1] as i32) << 8) as i64 as u64;
        tail |= p[i] as u64;
    } else if remaining == 6 {
        tail |= ((p[i + 5] as u64) << 20) << 20;
        tail |= ((p[i + 4] as u64) << 16) << 16;
        tail |= ((p[i + 3] as i32).wrapping_shl(24)) as i64 as u64;
        tail |= ((p[i + 2] as i32) << 16) as i64 as u64;
        tail |= ((p[i + 1] as i32) << 8) as i64 as u64;
        tail |= p[i] as u64;
    } else if remaining == 5 {
        tail |= ((p[i + 4] as u64) << 16) << 16;
        tail |= ((p[i + 3] as i32).wrapping_shl(24)) as i64 as u64;
        tail |= ((p[i + 2] as i32) << 16) as i64 as u64;
        tail |= ((p[i + 1] as i32) << 8) as i64 as u64;
        tail |= p[i] as u64;
    } else if remaining == 4 {
        tail |= ((p[i + 3] as i32).wrapping_shl(24)) as i64 as u64;
        tail |= ((p[i + 2] as i32) << 16) as i64 as u64;
        tail |= ((p[i + 1] as i32) << 8) as i64 as u64;
        tail |= p[i] as u64;
    } else if remaining == 3 {
        tail |= ((p[i + 2] as i32) << 16) as i64 as u64;
        tail |= ((p[i + 1] as i32) << 8) as i64 as u64;
        tail |= p[i] as u64;
    } else if remaining == 2 {
        tail |= ((p[i + 1] as i32) << 8) as i64 as u64;
        tail |= p[i] as u64;
    } else if remaining == 1 {
        tail |= p[i] as u64;
    } else {
        // case 0: nothing
    }
    data = tail;

    v3 ^= data;
    for _ in 0..2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

fn stbds_hash_bytes(p: &[u8], len: usize, seed: u64) -> u64 {
    stbds_siphash_bytes(p, len, seed)
}

fn siphash(init: i32, out: &mut impl Write) {
    let mut mem = [0u8; 64];
    let mut z: i32 = init;
    for i in 0..64 {
        // C: mem[i] = z; (assigning int to unsigned char truncates lower 8 bits)
        mem[i] = (z as u32 & 0xff) as u8;
        z = z.wrapping_add(1);
    }
    for i in 0..64 {
        let hash = stbds_hash_bytes(&mem, i, 0);
        write!(out, "  {{ ").unwrap();
        for j in 0..8 {
            let byte = ((hash >> (j * 8)) & 0xff) as u8;
            write!(out, "0x{:02x}, ", byte).unwrap();
        }
        write!(out, " }},\n").unwrap();
    }
}

fn main() {
    // Read entire stdin and parse the first integer (scanf-like behavior:
    // skips leading whitespace including newlines, then reads optional sign
    // and digits).
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok();

    let init = parse_scanf_int(&input).unwrap_or(0);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    siphash(init, &mut out);
    out.flush().ok();
}

// Parse an integer in the manner of scanf("%d", ...): skip leading whitespace,
// optional sign, then digits. Stops at first non-digit.
fn parse_scanf_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
        idx += 1;
    }
    if idx >= bytes.len() {
        return None;
    }
    let mut sign: i64 = 1;
    if bytes[idx] == b'+' {
        idx += 1;
    } else if bytes[idx] == b'-' {
        sign = -1;
        idx += 1;
    }
    let start = idx;
    let mut value: i64 = 0;
    while idx < bytes.len() && (bytes[idx] as char).is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((bytes[idx] - b'0') as i64);
        idx += 1;
    }
    if idx == start {
        return None;
    }
    Some(value.wrapping_mul(sign) as i32)
}
