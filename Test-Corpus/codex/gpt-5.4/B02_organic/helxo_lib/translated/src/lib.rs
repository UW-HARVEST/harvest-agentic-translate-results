use std::ffi::{c_char, c_int};

const PRINTF_FORMAT: &[u8] = b"%s %c\n\0";
const BOB: &[u8] = b"bob\0";
const SALLY: &[u8] = b"sally\0";
const FRED: &[u8] = b"fred\0";
const JEN: &[u8] = b"jen\0";
const DOUG: &[u8] = b"doug\0";

fn print_entry(name: &[u8], value: c_int) {
    unsafe {
        libc::printf(
            PRINTF_FORMAT.as_ptr().cast(),
            name.as_ptr().cast::<c_char>(),
            value,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    print_entry(BOB, 'h' as c_int);
    print_entry(SALLY, 'e' as c_int);
    print_entry(FRED, 'l' as c_int);
    print_entry(JEN, letter as c_int);
    print_entry(DOUG, 'o' as c_int);
}
