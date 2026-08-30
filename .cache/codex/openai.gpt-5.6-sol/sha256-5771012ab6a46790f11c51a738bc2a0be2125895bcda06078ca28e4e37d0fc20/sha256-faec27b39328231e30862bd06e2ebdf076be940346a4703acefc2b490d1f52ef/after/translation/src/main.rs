use std::ffi::{c_char, c_int};
use std::io::{self, Write};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_line(output: &mut impl Write, line: &str) -> io::Result<()> {
    writeln!(output, "{line}")
}

fn print_hex_char_line(output: &mut impl Write, value: i8) -> io::Result<()> {
    writeln!(output, "{:02x}", value as i32 as u32)
}

fn bad(output: &mut impl Write) -> io::Result<()> {
    let data = i8::MAX;
    if data > 0 {
        let result = data.wrapping_mul(2);
        print_hex_char_line(output, result)?;
    }
    Ok(())
}

fn good_g2b(output: &mut impl Write) -> io::Result<()> {
    let data = 2_i8;
    if data > 0 {
        let result = data.wrapping_mul(2);
        print_hex_char_line(output, result)?;
    }
    Ok(())
}

fn good_b2g(output: &mut impl Write) -> io::Result<()> {
    let data = i8::MAX;
    if data > 0 {
        if data < i8::MAX / 2 {
            let result = data.wrapping_mul(2);
            print_hex_char_line(output, result)?;
        } else {
            print_line(
                output,
                "data value is too large to perform arithmetic safely.",
            )?;
        }
    }
    Ok(())
}

fn good(output: &mut impl Write) -> io::Result<()> {
    good_g2b(output)?;
    good_b2g(output)
}

fn main() {
    let mut x: c_int = 0;

    // This narrow FFI boundary preserves scanf's tokenization and failure behavior.
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    let stdout = io::stdout();
    let mut output = stdout.lock();
    let _ = if x != 0 {
        good(&mut output)
    } else {
        bad(&mut output)
    };
}
