// Translation of c_src/src/lib.c to Rust.
// The C source defines a library function `memchra2(a, b, c, d)` returning an int.
// Since the task requires an executable, this binary reads 4 integers from
// stdin (scanf-style: whitespace-separated, including newlines) and prints
// the result of memchra2 followed by a newline (matching `printf("%d\n", x)`).

use std::io::{self, Read, Write};

// memchra: counts the number of bytes equal to (c as i8) within the first
// `n` bytes of `str`. The C version compares str[i] == (char)c.
fn memchra(s: &[u8], c: i32, n: usize) -> i32 {
    let target = c as i8 as u8;
    let mut count: i32 = 0;
    let bound = if n <= s.len() { n } else { s.len() };
    for i in 0..bound {
        if s[i] == target {
            count = count.wrapping_add(1);
        }
    }
    count
}

// process_buffer: returns the sum of bytes (interpreted as signed char then
// promoted to int) until '\0' or the length is reached. Returns -1 if the
// buffer is empty.
fn process_buffer(buffer: &[u8], len: usize) -> i32 {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }

    let mut result: i32 = 0;
    let bound = if len <= buffer.len() { len } else { buffer.len() };
    for i in 0..bound {
        let b = buffer[i];
        if b == 0 {
            break;
        }
        // C: result += (int)(*i) where *i is `char` (signed on Linux x86_64).
        result = result.wrapping_add(b as i8 as i32);
    }
    result
}

// int_to_float_bits: reinterpret the bits of an int as a float (union-based
// type-pun in C).
fn int_to_float_bits(value: i32) -> f32 {
    f32::from_bits(value as u32)
}

// process_strings: counts how many of the first `count` strings begin with
// `target`. Treats null pointers and empty strings as skipped.
fn process_strings(strings: &[&str], count: i32, target: &str) -> i32 {
    if count <= 0 {
        return 0;
    }

    let target_bytes = target.as_bytes();
    let target_len = target_bytes.len();
    let mut matches: i32 = 0;
    let n = count as usize;
    let bound = if n <= strings.len() { n } else { strings.len() };

    for s in &strings[..bound] {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            continue;
        }
        // strncmp(*i, target, strlen(target)) == 0
        if bytes.len() >= target_len && &bytes[..target_len] == target_bytes {
            matches = matches.wrapping_add(1);
        }
    }

    matches
}

// safe_sum_array: sums an array of ints with wrapping (matching the C
// behavior on common compilers).
fn safe_sum_array(arr: &[i32]) -> i32 {
    if arr.is_empty() {
        return 0;
    }
    let mut sum: i32 = 0;
    for v in arr {
        sum = sum.wrapping_add(*v);
    }
    sum
}

// interpret_as_int: in C this casts `unsigned char *` to `int *` and
// dereferences, which on little-endian x86_64 reads the bytes in
// little-endian order.
fn interpret_as_int(bytes: &[u8], len: usize) -> i32 {
    if bytes.is_empty() || len < 4 {
        return 0;
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[..4]);
    i32::from_le_bytes(buf)
}

// count_occurrences: counts how many times `ch` appears in `text`. Returns 0
// for empty / null input.
fn count_occurrences(text: &[u8], ch: u8) -> i32 {
    if text.is_empty() || text[0] == 0 {
        return 0;
    }
    // strlen analogue: bytes up to first NUL.
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    memchra(text, ch as i32, len)
}

// complex_iteration: XOR-fold the low byte of each i32 in `data`.
fn complex_iteration(data: &[i32]) -> i32 {
    if data.is_empty() {
        return -1;
    }
    let mut result: i32 = 0;
    for v in data {
        let u = *v as u32;
        result ^= (u & 0xFF) as i32;
    }
    result
}

fn memchra2(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let mut result: i32 = 0;

    // snprintf(buffer, 64, "test%d-%d-%d-%d", a, b, c, d);
    // Format: %d for i32 in C uses the standard signed decimal representation.
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    // Truncate to 63 bytes to match snprintf with size=64 (which writes at
    // most 63 characters plus a NUL terminator).
    let mut buffer: Vec<u8> = Vec::with_capacity(64);
    let bytes = formatted.as_bytes();
    let take = bytes.len().min(63);
    buffer.extend_from_slice(&bytes[..take]);
    buffer.push(0); // NUL terminator
    // Pad up to 64 bytes (uninitialized in C, but the rest of the code only
    // looks at the NUL-terminated prefix).
    while buffer.len() < 64 {
        buffer.push(0);
    }

    let dash_count = count_occurrences(&buffer, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values);
    result = result.wrapping_add(sum);

    let test_strings = ["test1", "test2", "testing", "other"];
    let matches = process_strings(&test_strings, 4, "test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        // C cast (int)f truncates toward zero. Since f is in (0, 1000), this
        // fits in i32.
        result = result.wrapping_add(f as i32);
    }

    // strlen(buffer) — bytes up to NUL.
    let buf_strlen = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(&buffer, buf_strlen);
    if buf_sum > 0 {
        // C: buf_sum % 256 with positive buf_sum is positive. Use rem_euclid to
        // mirror this even if signed arithmetic in Rust differs for negatives
        // (we already gated on buf_sum > 0).
        result = result.wrapping_add(buf_sum % 256);
    }

    let mut bytes_arr = [0u8; 4];
    bytes_arr[0] = (b & 0xFF) as u8;
    bytes_arr[1] = (c & 0xFF) as u8;
    bytes_arr[2] = (d & 0xFF) as u8;
    bytes_arr[3] = 0;

    let interpreted = interpret_as_int(&bytes_arr, 4);
    result ^= interpreted;

    let complex_result = complex_iteration(&values);
    result = result.wrapping_add(complex_result);

    result
}

fn read_int_scanf<I: Iterator<Item = u8>>(iter: &mut std::iter::Peekable<I>) -> Option<i32> {
    // Skip leading whitespace (matches scanf %d behavior).
    while let Some(&b) = iter.peek() {
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0B || b == 0x0C {
            iter.next();
        } else {
            break;
        }
    }

    let mut sign: i64 = 1;
    if let Some(&b) = iter.peek() {
        if b == b'-' {
            sign = -1;
            iter.next();
        } else if b == b'+' {
            iter.next();
        }
    }

    let mut have_digit = false;
    let mut value: i64 = 0;
    while let Some(&b) = iter.peek() {
        if b.is_ascii_digit() {
            have_digit = true;
            value = value.wrapping_mul(10).wrapping_add((b - b'0') as i64);
            iter.next();
        } else {
            break;
        }
    }

    if !have_digit {
        return None;
    }

    Some((sign * value) as i32)
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return;
    }

    let mut iter = input.into_iter().peekable();
    let a = match read_int_scanf(&mut iter) { Some(v) => v, None => return };
    let b = match read_int_scanf(&mut iter) { Some(v) => v, None => return };
    let c = match read_int_scanf(&mut iter) { Some(v) => v, None => return };
    let d = match read_int_scanf(&mut iter) { Some(v) => v, None => return };

    let result = memchra2(a, b, c, d);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", result);
}
