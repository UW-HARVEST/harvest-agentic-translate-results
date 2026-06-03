// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::Read;

fn driver(c: libc::c_char) {
    unsafe {
        let locale_c = std::ffi::CString::new("C").unwrap();
        libc::setlocale(libc::LC_ALL, locale_c.as_ptr());

        // Pass as int (matching the C macros). To avoid UB on signed chars,
        // mirror the typical C usage by passing the value directly.
        let ic: libc::c_int = c as libc::c_int;

        println!("alphanumeric: {}", libc::isalnum(ic));
        println!("alphabetic: {}", libc::isalpha(ic));
        println!("lowercase: {}", libc::islower(ic));
        println!("uppercase: {}", libc::isupper(ic));
        println!("digit: {}", libc::isdigit(ic));
        println!("hexadecimal: {}", libc::isxdigit(ic));
        println!("control: {}", libc::iscntrl(ic));
        println!("graphical: {}", libc::isgraph(ic));
        println!("space: {}", libc::isspace(ic));
        println!("blank: {}", libc::isblank(ic));
        println!("printing: {}", libc::isprint(ic));
        println!("punctuation: {}", libc::ispunct(ic));

        let lower = libc::tolower(ic) as u8 as char;
        let upper = libc::toupper(ic) as u8 as char;
        println!("to lower: {}", lower);
        println!("to upper: {}", upper);
    }
}

fn main() {
    let mut buf = [0u8; 1];
    let c: libc::c_char = match std::io::stdin().read(&mut buf) {
        Ok(n) if n > 0 => buf[0] as libc::c_char,
        // EOF in C is -1; getchar() returns int. The C code stores into char,
        // which truncates. We mimic that here.
        _ => -1 as libc::c_char,
    };
    driver(c);
}
