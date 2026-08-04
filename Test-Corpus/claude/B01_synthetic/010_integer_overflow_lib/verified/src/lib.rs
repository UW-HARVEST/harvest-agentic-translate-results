use std::ffi::c_char;
use std::io::Write;

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    // C: printf("%02x\n", charHex);
    // In C, char (signed) is promoted to int via varargs; %x reinterprets as unsigned int.
    // For negative chars, sign extension produces values like 0xffffff80.
    let promoted: i32 = char_hex as i32; // sign-extend
    let as_unsigned: u32 = promoted as u32;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    // %02x means minimum width 2, lowercase hex, zero-padded.
    let _ = write!(handle, "{:02x}\n", as_unsigned);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result: c_char = data.wrapping_add(1);
    printHexCharLine(result);
}
