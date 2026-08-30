use std::ffi::{c_char, c_int};
use std::io::{self, Write};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn fma_array(values: &mut [c_int]) {
    for value in values {
        *value = value.wrapping_mul(*value).wrapping_add(*value);
    }
}

fn main() {
    let mut data = [0 as c_int; 100];
    let mut len = 0;

    while len < data.len() {
        // SAFETY: the format expects one int pointer, and data[len] is writable.
        let converted = unsafe { scanf(c"%d".as_ptr(), &mut data[len]) };
        if converted != 1 {
            break;
        }
        len += 1;
    }

    fma_array(&mut data[..len]);

    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    for value in &data[..len] {
        let _ = writeln!(output, "{value}");
    }
}
