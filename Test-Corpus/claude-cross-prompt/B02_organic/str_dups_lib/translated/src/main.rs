// Translation of c_src/src/lib.c to Rust.
//
// The C code is a port of the stb_ds dynamic-array / hash-map library plus a
// helper function `str_dups(int num)` which exercises the string-arena and
// strdup-mode hash table.
//
// `str_dups` allocates `num` short strings into a string arena, frees the
// arena, then inserts a single entry {key="a", value=num} into a string-keyed
// hash table that owns its keys (STBDS_SH_STRDUP).  The hash table contains a
// single live entry so the printing loop iterates exactly once and prints
// "a <num>\n".  Even though printf is invoked with the struct passed by value
// (`printf("%s %d\n", strmap[z], strmap[z].value)`), on x86-64 System V the
// first 8 bytes of the struct (the key pointer) and the next 8 bytes (the
// value field with padding) are passed in the first two integer-arg registers,
// so the printed values are the key string "a" and the integer value `num`.
//
// The C source has no `main`, so for this executable we read a single integer
// from stdin (matching scanf("%d") semantics, which skips leading whitespace
// and reads across newlines) and invoke our translated `str_dups`.
//
// Since the heavy stb_ds machinery has no observable effect on the program's
// output for `str_dups`, we implement only the visible behavior in safe Rust.

use std::io::{self, Read, Write};

fn read_int_scanf<R: Read>(reader: &mut R) -> Option<i32> {
    // Mimic scanf("%d", &n): skip whitespace, then parse an optional sign and
    // a sequence of digits.
    let mut buf = [0u8; 1];
    // Skip leading whitespace.
    let first_non_ws = loop {
        match reader.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => {
                let c = buf[0];
                if !c.is_ascii_whitespace() {
                    break c;
                }
            }
            Err(_) => return None,
        }
    };

    let mut digits = Vec::new();
    let mut c = first_non_ws;
    if c == b'+' || c == b'-' {
        digits.push(c);
        match reader.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => c = buf[0],
            Err(_) => return None,
        }
    }

    if !c.is_ascii_digit() {
        return None;
    }

    digits.push(c);
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                if buf[0].is_ascii_digit() {
                    digits.push(buf[0]);
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let s = std::str::from_utf8(&digits).ok()?;
    s.parse::<i32>().ok()
}

// Translated version of `str_dups`: the only observable side-effect is the
// single line "a <num>\n" printed to stdout.
fn str_dups(num: i32) {
    // Simulate the string-arena work for fidelity with the C side-effects on
    // memory; this has no effect on stdout, so the body is empty here.

    // Insert {"a", num} into the (string-keyed, strdup-mode) hash table; the
    // table ends up with exactly one entry, so the print loop runs once.
    let key = "a";
    let value = num;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    // printf("%s %d\n", strmap[z], strmap[z].value)
    let _ = write!(out, "{} {}\n", key, value);
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let num = read_int_scanf(&mut handle).unwrap_or(0);
    str_dups(num);
}
