// Translation of c_src/src/lib.c to Rust.
//
// The original C is a shared library that exposes a single function:
//     void sh_geti(int num);
//
// `sh_geti` exercises a string-keyed hash map (stb_ds.h style). Its ONLY
// observable output (via stdout) comes from a single line:
//
//     printf("%s %d\n", strmap[z], strmap[z].value);
//
// where `strmap[z]` is a `struct { char *key; int value; }` passed by value.
// On the x86_64 System V ABI, that 16-byte struct is passed in two registers
// (the key pointer and the value, padded). So `%s` reads the key pointer and
// `%d` reads the value field — the trailing `strmap[z].value` argument is
// consumed by nothing in the format string.
//
// The `printf` loop runs BEFORE any deletes, so the entries are iterated in
// insertion order. Inside `sh_geti`, the inserts are:
//     for (i = 0; i < num; i += 2) shput(strmap, strkey(i), i * 3);
// so the visible output for each of the two j-iterations is:
//     test_0 0
//     test_2 6
//     test_4 12
//     ...
// for every even `i` in [0, num).
//
// The task wants byte-identical stdout for the same inputs. Since the C is a
// library with no main, we provide one that reads a single integer from stdin
// in `scanf("%d", &num)` style — leading whitespace is skipped and parsing
// crosses newlines.

use std::io::{self, Read, Write};

fn main() {
    // Read all of stdin and parse the first whitespace-separated integer,
    // matching C's `scanf("%d", &num)` semantics (skip leading whitespace,
    // read across newlines).
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        // If stdin can't be read, treat as no input (num remains default 0).
    }
    let num: i32 = parse_first_int(&input).unwrap_or(0);

    sh_geti(num);
}

/// Parses the first whitespace-separated signed integer from `s`, mirroring
/// scanf("%d") behavior (skip leading whitespace, optional sign, then digits).
fn parse_first_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    std::str::from_utf8(&bytes[start..i]).ok()?.parse::<i32>().ok()
}

/// Translation of the C `sh_geti(int num)` function. Only stdout output is
/// preserved; internal hash-map operations have no externally visible effect
/// beyond the single printf call below, which we reproduce.
fn sh_geti(num: i32) {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    // The C code runs an outer `for (j = 0; j < 2; ++j)` loop and within each
    // iteration prints every entry (in insertion order) of the string hash
    // map. Inserts happen for every even i in [0, num), with value = i * 3.
    for _j in 0..2 {
        let mut i: i32 = 0;
        while i < num {
            // Match C's `printf("%s %d\n", ..., i*3)` exactly. Use wrapping
            // multiplication to mirror C's typical 32-bit two's-complement
            // overflow behavior (and avoid Rust debug-mode panics).
            let value = i.wrapping_mul(3);
            // sprintf(buffer, "test_%d", n) — the static buffer matches our
            // format here byte-for-byte.
            let _ = writeln!(out, "test_{} {}", i, value);
            i = i.wrapping_add(2);
        }
    }

    let _ = out.flush();
}
