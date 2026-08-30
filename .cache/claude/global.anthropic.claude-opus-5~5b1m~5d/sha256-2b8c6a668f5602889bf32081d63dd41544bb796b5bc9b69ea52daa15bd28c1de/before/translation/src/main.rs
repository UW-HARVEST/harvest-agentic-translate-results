// Rust translation of c_src/src/main.c
//
// Original C:
//     int main() {
//         int x = 1, y = 1;
//         scanf("%d %d", &x, &y);
//         div_t result = div(x, y);
//         printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
//         return 0;
//     }
//
// Behaviour that must be reproduced exactly:
//   * scanf("%d %d") skips arbitrary leading whitespace (including newlines)
//     before each conversion, and leaves a variable untouched when its
//     conversion does not happen (matching failure / input failure), so the
//     initial value of 1 survives.
//   * glibc converts %d through a `long`, so out-of-range input saturates at
//     LONG_MAX / LONG_MIN and is then truncated to `int`
//     (e.g. "99999999999999999999999" -> -1, "4294967296" -> 0).
//   * div(x, 0) and div(INT_MIN, -1) are undefined behaviour in C; on x86-64
//     the hardware `idiv` instruction raises SIGFPE, which is what the
//     original binary does (exit status 128+8).

use std::io::Read;
use std::io::Write;

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates a single `%d` conversion of glibc's scanf.
///
/// Returns `Some(value)` when the conversion succeeded, `None` on input
/// failure (EOF) or matching failure (no digits).  `pos` is advanced past the
/// consumed characters.
fn scan_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Directive whitespace / the leading whitespace skip of %d.
    while *pos < input.len() && is_c_space(input[*pos]) {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None; // input failure
    }

    let start = *pos;
    let mut negative = false;
    if input[*pos] == b'+' || input[*pos] == b'-' {
        negative = input[*pos] == b'-';
        *pos += 1;
    }

    if *pos >= input.len() || !input[*pos].is_ascii_digit() {
        // Matching failure: push back what we read (the sign, if any).
        *pos = start;
        return None;
    }

    // Accumulate as a C `long` (64-bit) with strtol-style saturation.
    let mut acc: i64 = 0;
    let mut saturated = false;
    while *pos < input.len() && input[*pos].is_ascii_digit() {
        let digit = (input[*pos] - b'0') as i64;
        *pos += 1;
        if saturated {
            continue;
        }
        match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) => acc = v,
            None => saturated = true,
        }
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    // Storing a `long` into an `int`: implementation-defined truncation.
    Some(value as u64 as u32 as i32)
}

/// `div(num, den)` as executed by the original binary: a raw hardware signed
/// division, so division by zero and INT_MIN / -1 raise SIGFPE just like the C
/// program instead of producing a Rust panic.
#[cfg(target_arch = "x86_64")]
fn c_div(num: i32, den: i32) -> (i32, i32) {
    let quot: i32;
    let rem: i32;
    unsafe {
        std::arch::asm!(
            "cdq",
            "idiv {den:e}",
            den = in(reg) den,
            inout("eax") num => quot,
            out("edx") rem,
            options(nostack),
        );
    }
    (quot, rem)
}

#[cfg(not(target_arch = "x86_64"))]
fn c_div(num: i32, den: i32) -> (i32, i32) {
    match (num.checked_div(den), num.checked_rem(den)) {
        (Some(q), Some(r)) => (q, r),
        // Mirror the fatal arithmetic fault of the original program.
        _ => std::process::abort(),
    }
}

fn main() {
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);

    let mut pos = 0usize;
    if let Some(v) = scan_int(&input, &mut pos) {
        x = v;
        if let Some(v) = scan_int(&input, &mut pos) {
            y = v;
        }
    }

    let (quot, rem) = c_div(x, y);
    print!("quotient: {}, remainder: {}\n", quot, rem);
    let _ = std::io::stdout().flush();
}
