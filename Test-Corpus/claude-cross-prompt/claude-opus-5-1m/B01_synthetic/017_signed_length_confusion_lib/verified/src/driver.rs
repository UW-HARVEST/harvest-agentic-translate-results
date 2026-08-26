// Translated from c_src/src/driver.c
//
// Original C function:
//
//     void driver(int data)
//     {
//         char source[100];
//         char dest[100] = "";
//         memset(source, 'A', 100-1);
//         source[100-1] = '\0';
//         if (data < 100)
//         {
//             strncpy(dest, source, data);
//             dest[data] = '\0';
//         }
//         printLine(dest);
//     }
//
// Behavior reproduced for defined inputs (0 <= data < 100 and data >= 100):
//   - 0 <= data < 100: prints `data` 'A' characters followed by '\n'
//   - data >= 100:     prints just '\n' (dest is the empty string)
//
// Negative `data` is undefined behavior in the original C (passing a negative
// value through size_t to strncpy results in a huge length and out-of-bounds
// writes). Mirror the most conservative observable behavior: produce no
// crash but print just '\n', equivalent to the dest buffer remaining empty.

use std::io::{self, Write};

fn print_line(line: &[u8]) {
    // Equivalent to: if (line != NULL) printf("%s\n", line);
    // In Rust we always have a valid slice here.
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(line);
    let _ = out.write_all(b"\n");
}

pub fn driver(data: i32) {
    // char source[100]; memset(source, 'A', 99); source[99] = '\0';
    let mut source = [0u8; 100];
    for b in source.iter_mut().take(99) {
        *b = b'A';
    }
    source[99] = 0;

    // char dest[100] = "";  // zero-initialized
    let mut dest = [0u8; 100];

    if data < 100 {
        if data >= 0 {
            let n = data as usize;
            // strncpy(dest, source, data): copy up to n bytes from source.
            // If a NUL is encountered, the rest is filled with NULs (already zero).
            // Source is 99 'A's then NUL, so for n <= 99 we copy n 'A's.
            let copy_len = n.min(source.len());
            dest[..copy_len].copy_from_slice(&source[..copy_len]);
            // dest[data] = '\0'
            if n < dest.len() {
                dest[n] = 0;
            }
        }
        // For negative data, the C is undefined behavior. We deliberately do
        // not attempt to reproduce a crash; dest stays the empty string.
    }

    // Find the C string length (up to first NUL) for printf("%s", dest).
    let len = dest.iter().position(|&b| b == 0).unwrap_or(dest.len());
    print_line(&dest[..len]);
}
