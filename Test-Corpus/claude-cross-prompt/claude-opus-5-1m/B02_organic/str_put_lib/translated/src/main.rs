// Translation of c_src/src/lib.c to Rust.
//
// The original C file is a library that exposes a single function `str_put(int num)`.
// To turn it into an executable that mimics the behavior of the original C code,
// we read a single integer from stdin (as `scanf("%d", &num)` would) and pass it
// to a Rust port of `str_put`.
//
// The C `str_put` builds a tiny stb_ds string-hash map containing a single entry
// `{"a", num}` and then prints it. The original C contains a deliberate bug in its
// printf call:
//
//   printf("%s %d\n", strmap[z], strmap[z].value);
//
// `strmap[z]` is a struct value (not the `.key` field), but on the SysV AMD64 ABI
// the struct is split into two 8-byte argument slots: the first 8 bytes are the
// `char *key` pointer, the next 8 bytes contain the `int value`. printf reads
// `%s` from the first slot (the key pointer "a") and `%d` from the second slot
// (the value). The trailing `strmap[z].value` argument is ignored.
//
// In practice this prints `a {num}\n`, which is what we reproduce here.
//
// We must reproduce this exact output for byte-identical results.

use std::io::{self, Read, Write};

fn read_int_from_stdin() -> Option<i32> {
    // Mimic C's scanf("%d", &num):
    //  - Skip leading whitespace (including newlines)
    //  - Optional sign
    //  - Decimal digits
    //  - Stop at first non-digit
    //  - Returns None if no integer was matched
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return None;
    }
    let bytes = buf.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let mut sign: i64 = 1;
    if bytes[i] == b'+' {
        i += 1;
    } else if bytes[i] == b'-' {
        sign = -1;
        i += 1;
    }
    let start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return None;
    }
    let s = &buf[start..i];
    let v: i64 = s.parse().ok()?;
    Some((sign * v) as i32)
}

// Port of str_put from lib.c.
//
// The original implementation exercises stb_ds:
//   - calls stralloc(num) times into a string arena (then strreset()s it)
//   - inserts a single key/value into a string-keyed hash map and prints it.
//
// The visible side effect (and the only output) is the final printf loop, which
// in practice prints exactly one line: "a <num>\n".
fn str_put(num: i32) {
    // Faithful reproduction of the arena allocations: in C this calls stralloc
    // `num` times with strings of the form "test_<i>". stralloc has no I/O side
    // effects so we simulate the work but discard the results, matching observable
    // output.
    let mut arena: Vec<String> = Vec::new();
    for i in 0..num {
        arena.push(format!("test_{}", i));
    }
    drop(arena); // equivalent to strreset(&sa)

    // Equivalent to:
    //   shputs(strmap, s); where s = {"a", num}
    //   for (z=0; z < shlen(strmap); ++z)
    //       printf("%s %d\n", strmap[z], strmap[z].value);
    //
    // shlen returns the number of entries inserted, which is 1.
    // The printf bug (passing the struct rather than .key) results in the
    // key pointer being printed via %s and the value via %d on x86_64 SysV.
    let key = "a";
    let value = num;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Single iteration of the loop that runs `shlen(strmap)` (== 1) times.
    let _ = write!(out, "{} {}\n", key, value);
}

fn main() {
    let num = read_int_from_stdin().unwrap_or(0);
    str_put(num);
}
