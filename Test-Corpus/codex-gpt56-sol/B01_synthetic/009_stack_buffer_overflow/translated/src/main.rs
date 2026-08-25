use std::ffi::{c_char, c_int};
use std::io::{self, Read, Write};

unsafe extern "C" {
    fn atoi(nptr: *const c_char) -> c_int;
}

fn print_line(output: &mut impl Write, line: &str) -> io::Result<()> {
    writeln!(output, "{line}")
}

fn print_int_line(output: &mut impl Write, number: i32) -> io::Result<()> {
    writeln!(output, "{number}")
}

fn fgets_14(input: &mut impl Read) -> Option<Vec<u8>> {
    let mut buffer = Vec::with_capacity(13);

    while buffer.len() < 13 {
        let mut byte = [0_u8; 1];
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buffer.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    (!buffer.is_empty()).then_some(buffer)
}

fn c_atoi(input: &[u8]) -> i32 {
    let mut nul_terminated = Vec::with_capacity(input.len() + 1);
    nul_terminated.extend_from_slice(input);
    nul_terminated.push(0);

    // The source calls the platform C library's atoi, including its overflow behavior.
    unsafe { atoi(nul_terminated.as_ptr().cast::<c_char>()) as i32 }
}

fn print_buffer(output: &mut impl Write, buffer: &[i32; 10]) -> io::Result<()> {
    for value in buffer {
        print_int_line(output, *value)?;
    }
    Ok(())
}

fn bad(input: &mut impl Read, output: &mut impl Write) -> io::Result<()> {
    let mut data = -1;
    if let Some(input_buffer) = fgets_14(input) {
        data = c_atoi(&input_buffer);
    } else {
        print_line(output, "fgets() failed.")?;
    }

    let mut buffer = [0_i32; 10];
    if data >= 0 {
        // C writes out of bounds here when data >= 10, which has no defined output.
        // Keep the missing upper-bound validation without making safe Rust corrupt memory.
        if let Some(element) = buffer.get_mut(data as usize) {
            *element = 1;
        }
        print_buffer(output, &buffer)?;
    } else {
        print_line(output, "ERROR: Array index is negative.")?;
    }
    Ok(())
}

fn good_g2b(output: &mut impl Write) -> io::Result<()> {
    let data = 7;
    let mut buffer = [0_i32; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        print_buffer(output, &buffer)?;
    } else {
        print_line(output, "ERROR: Array index is negative.")?;
    }
    Ok(())
}

fn good_b2g(input: &mut impl Read, output: &mut impl Write) -> io::Result<()> {
    let mut data = -1;
    if let Some(input_buffer) = fgets_14(input) {
        data = c_atoi(&input_buffer);
    } else {
        print_line(output, "fgets() failed.")?;
    }

    let mut buffer = [0_i32; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        print_buffer(output, &buffer)?;
    } else {
        print_line(output, "ERROR: Array index is out-of-bounds")?;
    }
    Ok(())
}

fn good(input: &mut impl Read, output: &mut impl Write) -> io::Result<()> {
    good_g2b(output)?;
    good_b2g(input, output)
}

fn run(input: &mut impl Read, output: &mut impl Write) -> io::Result<()> {
    print_line(output, "Calling good()...")?;
    good(input, output)?;
    print_line(output, "Finished good()")?;
    print_line(output, "Calling bad()...")?;
    bad(input, output)?;
    print_line(output, "Finished bad()")
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let _ = run(&mut input, &mut output);
}
