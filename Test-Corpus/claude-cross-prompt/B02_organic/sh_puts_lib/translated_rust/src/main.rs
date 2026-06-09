// Rust translation of c_src/src/lib.c
//
// The original C is a stb_ds.h-style hash table/array library with a single
// public function `sh_puts(int num)`. That function:
//   1. Performs some scratch arena allocations (no observable output).
//   2. Creates a string-keyed hash map and inserts a single entry
//      with key "a" and value `num`.
//   3. Iterates the hash map (which contains exactly one entry) and prints
//      "%s %d\n" for each entry.
//
// Therefore the only observable output for input `num` is the line
//     a <num>\n
//
// This Rust executable reads an integer from stdin (matching scanf("%d", ...)
// behavior — whitespace including newlines is skipped, and the integer parse
// stops at the first non-digit character) and reproduces the same output.

use std::io::{self, Read, Write};

mod stbds;

fn main() {
    // Read all of stdin, then parse a single int scanf-style.
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let num = match scanf_int(&input) {
        Some(n) => n,
        // If scanf fails to read an integer, the C variable `num` would
        // remain uninitialized in `main`. In our wrapper we have no such
        // main; we simply do nothing, matching "no input no output".
        None => return,
    };

    sh_puts(num);
}

/// Mimic C's `scanf("%d", &n)` behavior: skip leading whitespace
/// (including newlines), then parse an optional sign followed by digits.
fn scanf_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Skip whitespace
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let mut neg = false;
    if bytes[i] == b'+' {
        i += 1;
    } else if bytes[i] == b'-' {
        neg = true;
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return None;
    }
    if neg {
        value = value.wrapping_neg();
    }
    Some(value as i32)
}

/// Translation of the C `sh_puts(int num)` function.
fn sh_puts(num: i32) {
    // (1) Scratch arena allocations and reset — no observable output.
    let mut sa = stbds::StringArena::new();
    for i in 0..num {
        let k = strkey(i);
        sa.stralloc(&k);
    }
    sa.reset();

    // (2) Build the string-keyed hash map and insert {"a", num}.
    let mut strmap: stbds::StringHashMap<i32> = stbds::StringHashMap::new_arena();
    strmap.put("a", num);

    // Internal asserts in the C: keys are duplicated into the arena, value
    // matches. We don't need to replicate the asserts; they don't change
    // observable output.

    // (3) Iterate and print each entry.
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for (key, value) in strmap.iter() {
        // C: printf("%s %d\n", strmap[z], strmap[z].value);
        // Note: the C code passes `strmap[z]` (the struct) where %s expects a
        // `char *`. Because the struct's first field is `char *key`, this
        // happens to print the key. We replicate that observable behavior.
        let _ = writeln!(out, "{} {}", key, value);
    }
}

fn strkey(n: i32) -> String {
    // Matches C: sprintf(buffer, "test_%d", n);
    format!("test_{}", n)
}
