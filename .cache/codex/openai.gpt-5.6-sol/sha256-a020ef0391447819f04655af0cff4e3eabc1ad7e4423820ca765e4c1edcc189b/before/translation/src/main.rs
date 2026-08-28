use std::ffi::{CString, c_char, c_double, c_int};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

unsafe extern "C" {
    fn atof(input: *const c_char) -> c_double;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn parse_float(argument: &std::ffi::OsStr) -> f32 {
    let argument = CString::new(argument.as_bytes()).expect("arguments cannot contain NUL bytes");

    // SAFETY: CString supplies the NUL-terminated string required by atof.
    unsafe { atof(argument.as_ptr()) as f32 }
}

fn q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5_f32;
    let bits = 0x5f37_59df_u32.wrapping_sub(number.to_bits() >> 1);
    let y = f32::from_bits(bits);
    let correction = 1.5_f32 - (x2 * y) * y;

    y * correction
}

fn main() {
    let arguments: Vec<_> = std::env::args_os().collect();

    if arguments.len() != 4 {
        let mut stderr = std::io::stderr().lock();
        let _ = stderr.write_all(arguments[0].as_bytes());
        let _ = stderr.write_all(b" requires 4 inputs\n");
        std::process::exit(1);
    }

    let mut inputs = [
        parse_float(&arguments[1]),
        parse_float(&arguments[2]),
        parse_float(&arguments[3]),
    ];

    let squared_length = (inputs[0] * inputs[0] + inputs[1] * inputs[1])
        + inputs[2] * inputs[2];
    let inverse_length = q_rsqrt(squared_length);

    inputs[0] *= inverse_length;
    inputs[1] *= inverse_length;
    inputs[2] *= inverse_length;

    // SAFETY: The static format is NUL-terminated and each %f receives a promoted double.
    unsafe {
        printf(
            b"%f %f %f\n\0".as_ptr().cast(),
            inputs[0] as c_double,
            inputs[1] as c_double,
            inputs[2] as c_double,
        );
    }
}
