use std::io::{self, Read};
use std::os::raw::c_char;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

fn fgets_like(buffer: &mut [u8]) -> bool {
    if buffer.is_empty() {
        return false;
    }

    let mut stdin = io::stdin().lock();
    let mut count = 0usize;

    while count + 1 < buffer.len() {
        let mut byte = [0u8; 1];
        match stdin.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buffer[count] = byte[0];
                count += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if count == 0 {
        return false;
    }

    buffer[count] = 0;
    true
}

fn main() {
    let mut data: i32 = -1;

    {
        let mut input_buffer = [0u8; 14];
        if fgets_like(&mut input_buffer) {
            unsafe {
                data = libc::atoi(input_buffer.as_ptr() as *const c_char);
            }
        } else {
            print_line(b"fgets() failed.\0".as_ptr() as *const c_char);
        }
    }

    {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];

        unsafe {
            libc::memset(source.as_mut_ptr() as *mut libc::c_void, b'A' as i32, 100 - 1);
        }
        source[99] = 0;

        if data < 100 {
            unsafe {
                libc::strncpy(
                    dest.as_mut_ptr() as *mut c_char,
                    source.as_ptr() as *const c_char,
                    data as usize,
                );
                *dest.as_mut_ptr().offset(data as isize) = 0;
            }
        }

        print_line(dest.as_ptr() as *const c_char);
    }
}
