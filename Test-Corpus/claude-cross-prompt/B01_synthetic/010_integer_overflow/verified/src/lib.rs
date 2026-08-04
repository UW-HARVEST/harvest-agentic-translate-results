// Translation of c_src/src/main.c to Rust.
// Produces byte-identical output for the same inputs.

use std::ffi::{c_char, c_int, c_uint};
use std::io::{self, Read, Write};

/// Equivalent of:
///     void printHexCharLine(char charHex) { printf("%02x\n", charHex); }
///
/// In C, `charHex` (a `char`) undergoes default argument promotion to `int`.
/// `%x` then reinterprets the int's bit pattern as unsigned. So a negative
/// `char` value like -128 (0x80 as signed char) becomes -128 as int, whose
/// unsigned representation is 0xFFFFFF80, printing as "ffffff80".
#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    // Promote `char` to `int` (sign-extending if `c_char` is signed, which it
    // is on Linux x86_64), then reinterpret as unsigned to match printf %x.
    let promoted: c_int = char_hex as c_int;
    let as_unsigned: c_uint = promoted as c_uint;
    let line = format!("{:02x}\n", as_unsigned);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(line.as_bytes());
}

/// Equivalent of `int main(void)` in the original C program.
/// Reads one character from stdin via fscanf("%c", ...) semantics, computes
/// `data + 1` (preserving C's char arithmetic / truncation behavior), and
/// passes the result to printHexCharLine.
#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    // Initial value matches the C source: data = ' ';
    let mut data: c_char = b' ' as c_char;

    // fscanf(stdin, "%c", &data): read exactly one byte from stdin without
    // skipping whitespace. If reading fails or returns EOF, `data` retains
    // its prior value (which is ' ').
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    if handle.read_exact(&mut buf).is_ok() {
        data = buf[0] as c_char;
    }

    // C: char result = data + 1;
    // The expression `data + 1` is evaluated in int, then truncated back to
    // char on assignment. Use wrapping_add so values like 0x7F correctly
    // wrap to 0x80 (which is -128 when c_char is signed).
    let result: c_char = data.wrapping_add(1);
    printHexCharLine(result);

    0
}
