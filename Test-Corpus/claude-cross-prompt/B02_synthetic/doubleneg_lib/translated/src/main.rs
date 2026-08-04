// Rust translation of c_src/src/lib.c (doubleneg)
// Aims to produce byte-identical output to the C version.

use std::io::{self, BufWriter, Read, Write};

/// Mimic C's `(int)value` cast for f64 -> i32 on x86_64 (cvttsd2si):
/// truncate toward zero; out-of-range or NaN yields INT_MIN.
fn convert_double_to_int(value: f64) -> i32 {
    if value.is_nan() || value >= 2147483648.0 || value < -2147483648.0 {
        i32::MIN
    } else {
        value as i32
    }
}

/// Search `buffer` for the first occurrence of `search_val` (interpreted
/// as a byte, mirroring C's `(char)search_val` followed by memchr's
/// unsigned-char comparison).
fn find_value_in_buffer(buffer: &[u8], search_val: i32) -> i32 {
    let target = search_val as u8;
    match buffer.iter().position(|&b| b == target) {
        Some(pos) => pos as i32,
        None => -1,
    }
}

/// Fill `buffer` (length `size`) with `(seed + i*7) % 256`, matching
/// C's signed-int wrap-around and char truncation semantics.
fn create_numeric_buffer(buffer: &mut [u8], size: usize, seed: i32) {
    for i in 0..size {
        let idx = i as i32;
        // Use wrapping arithmetic to mirror C's (technically UB) signed
        // overflow behavior, which on common x86_64 toolchains wraps.
        let val = seed
            .wrapping_add(idx.wrapping_mul(7))
            .wrapping_rem(256);
        buffer[i] = val as u8;
    }
}

fn calculate_with_doubles(a: i32, b: i32, c: i32) -> f64 {
    let mut result: f64 = 0.0;
    if b != 0 {
        result = (a as f64) / (b as f64);
    }
    // C: pow(10.0, c % 10). C's % truncates toward zero, matching Rust's %.
    result *= 10.0_f64.powi(c % 10);
    result
}

/// Format a f64 in C's `%e` style (default precision 6).
fn format_e(x: f64) -> String {
    if x.is_nan() {
        if x.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        }
    } else if x.is_infinite() {
        if x < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        }
    } else {
        // Rust's {:.6e} produces e.g. "-1.099512e12" or "0.000000e0".
        // Convert to C's "-1.099512e+12" / "0.000000e+00" form.
        let s = format!("{:.6e}", x);
        let (mantissa, exp_str) = s.split_once('e').unwrap();
        let exp: i32 = exp_str.parse().unwrap();
        let exp_sign = if exp < 0 { '-' } else { '+' };
        let exp_abs = exp.unsigned_abs();
        // Ensure mantissa has a decimal point with 6 fractional digits.
        let mantissa_str = if mantissa.contains('.') {
            mantissa.to_string()
        } else {
            format!("{}.000000", mantissa)
        };
        format!("{}e{}{:02}", mantissa_str, exp_sign, exp_abs)
    }
}

fn doubleneg(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut result: i32 = 0;
    let mut buffer = [0u8; 256];

    writeln!(out, "=== Starting foo() execution ===").unwrap();
    writeln!(
        out,
        "Parameters: {}, {}, {}, {}",
        param1, param2, param3, param4
    )
    .unwrap();

    writeln!(out, "\n--- Integer Negation Test ---").unwrap();
    let negation_test = param1;
    let negation_result: i32 = if negation_test != 0 { 1 } else { 0 };
    writeln!(out, "Original value: {}", negation_test).unwrap();
    writeln!(out, "After !!negation: {}", negation_result).unwrap();
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2: i32 = if param2 != 0 { 1 } else { 0 };
    let neg_p3: i32 = if param3 != 0 { 1 } else { 0 };
    let neg_p4: i32 = if param4 != 0 { 1 } else { 0 };
    writeln!(
        out,
        "Double negation results: {}, {}, {}",
        neg_p2, neg_p3, neg_p4
    )
    .unwrap();
    result = result
        .wrapping_add(neg_p2)
        .wrapping_add(neg_p3)
        .wrapping_add(neg_p4);

    writeln!(out, "\n--- Double to Int Conversion Test ---").unwrap();

    let large_double = calculate_with_doubles(param1, param2, param3);
    writeln!(
        out,
        "Calculated double value: {}",
        format_e(large_double)
    )
    .unwrap();

    let converted_int = convert_double_to_int(large_double);
    writeln!(out, "Converted to int (may be UB): {}", converted_int).unwrap();

    let negative_large = -1.0_f64 * 2.0_f64.powi(40);
    writeln!(
        out,
        "Very large negative double: {}",
        format_e(negative_large)
    )
    .unwrap();
    let converted_neg = convert_double_to_int(negative_large);
    writeln!(out, "Converted to int (UB likely): {}", converted_neg).unwrap();

    let added = (converted_int.wrapping_rem(1000))
        .wrapping_add(converted_neg.wrapping_rem(1000));
    result = result.wrapping_add(added);

    writeln!(out, "\n--- Memchr Search Test ---").unwrap();

    create_numeric_buffer(&mut buffer, 256, param1);

    let search_values: [i32; 4] = [
        param2.wrapping_rem(256),
        param3.wrapping_rem(256),
        param4.wrapping_rem(256),
        42,
    ];
    let num_searches = search_values.len();

    writeln!(out, "Searching buffer for values...").unwrap();
    for i in 0..num_searches {
        let pos = find_value_in_buffer(&buffer, search_values[i]);
        if pos >= 0 {
            writeln!(
                out,
                "Found value {} at position {}",
                search_values[i], pos
            )
            .unwrap();
            result = result.wrapping_add(pos);
        } else {
            writeln!(out, "Value {} not found", search_values[i]).unwrap();
        }
    }

    // Direct memchr for byte 100 (printed with %ld for ptrdiff in C).
    if let Some(pos) = buffer.iter().position(|&b| b == 100) {
        writeln!(
            out,
            "Direct memchr found byte 100 at offset: {}",
            pos as i64
        )
        .unwrap();
        result = result.wrapping_add(pos as i32);
    }

    writeln!(out, "\n--- Combined Feature Test ---").unwrap();
    for i in 0..10i32 {
        let search_byte = param1
            .wrapping_add(i.wrapping_mul(param2))
            .wrapping_rem(256);
        let target = search_byte as u8;
        let found_flag: i32 = if buffer.iter().any(|&b| b == target) {
            1
        } else {
            0
        };
        writeln!(
            out,
            "Search {}: byte={}, found={}",
            i, search_byte, found_flag
        )
        .unwrap();
        result = result.wrapping_add(found_flag);
    }

    let infinity_val = f64::INFINITY;
    let nan_val = f64::NAN;

    writeln!(out, "\n--- Special Double Values ---").unwrap();
    write!(out, "Converting INFINITY to int: ").unwrap();
    let inf_as_int = convert_double_to_int(infinity_val);
    writeln!(out, "{} (undefined behavior)", inf_as_int).unwrap();

    write!(out, "Converting NAN to int: ").unwrap();
    let nan_as_int = convert_double_to_int(nan_val);
    writeln!(out, "{} (undefined behavior)", nan_as_int).unwrap();

    writeln!(out, "\n=== Final Result ===").unwrap();
    writeln!(out, "Accumulated result: {}", result).unwrap();

    out.flush().unwrap();
    result
}

fn main() {
    // Match the conventional scanf("%d %d %d %d", ...) wrapper: read all
    // of stdin, then split by ASCII whitespace (which crosses newlines)
    // and parse the first four tokens as i32.
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let mut iter = input.split_ascii_whitespace();
    let parse_next = |it: &mut std::str::SplitAsciiWhitespace| -> i32 {
        it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0)
    };
    let p1 = parse_next(&mut iter);
    let p2 = parse_next(&mut iter);
    let p3 = parse_next(&mut iter);
    let p4 = parse_next(&mut iter);

    doubleneg(p1, p2, p3, p4);
}
