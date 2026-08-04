// Translated from c_src/src/slicing.c
//
// The original C code is a library (no `main`). This Rust port preserves
// the `slice` function's behavior exactly and exposes a no-op `main`
// so the crate builds as an executable. With no main in the C source,
// invoking the resulting program produces no output for any input,
// which is the behavior the `main` here mirrors.

/// Index into a passed string and print the substring indexed by
/// [start, stop). If there is no start, use 0. If there is no stop,
/// use the end of the string.
///
/// Mirrors the C signature `int slice(char *mystr, int *start_ptr, int *stop_ptr)`.
pub fn slice(mystr: &str, start_ptr: Option<i32>, stop_ptr: Option<i32>) -> i32 {
    let len: usize = mystr.len();

    let start: i32;
    let stop: i32;

    if let Some(s) = start_ptr {
        start = s;
        // C compares int to size_t; size_t is unsigned, so the int is
        // promoted to size_t. Negative starts wrap to a huge value and
        // would trigger this branch, matching C behavior.
        if (start as i64) > (len as i64) && start >= 0 {
            // Reproduce the same comparison semantics: if start is
            // negative, the C cast to size_t makes it a very large
            // unsigned, exceeding len. Handle separately for fidelity.
            println!("Error: start is off the end of the string!");
            return 1;
        }
        if start < 0 {
            // Negative start in C: (size_t)start > len is true
            println!("Error: start is off the end of the string!");
            return 1;
        }
    } else {
        start = 0;
    }

    if let Some(s) = stop_ptr {
        stop = s;
        if (stop as i64) > (len as i64) && stop >= 0 {
            println!("Error: stop is off the end of the string!");
            return 1;
        }
        if stop < 0 {
            println!("Error: stop is off the end of the string!");
            return 1;
        }
        if stop <= start {
            println!("Error: stop must come after start!");
            return 1;
        }
    } else {
        stop = len as i32;
    }

    // Char arithmetic: skip ahead `start` characters in the array,
    // print `stop - start` bytes. Mirrors `printf("%.*s\n", ...)`.
    let bytes = mystr.as_bytes();
    let begin = start as usize;
    let end = stop as usize;
    // Safe slice; bounds were validated above.
    let slice_bytes = &bytes[begin..end];
    // Use stdout write so non-UTF-8 bytes (if any) print byte-identically.
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(slice_bytes).unwrap();
    handle.write_all(b"\n").unwrap();

    0
}

fn main() {
    // The original C file has no `main`. Running an executable derived
    // from that code produces no output, so this `main` is a no-op.
    let _ = slice;
}
