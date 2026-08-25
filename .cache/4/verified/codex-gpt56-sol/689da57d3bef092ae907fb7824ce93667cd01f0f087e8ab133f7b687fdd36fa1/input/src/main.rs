use std::ffi::{c_char, c_double, CString};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process;

extern "C" {
    fn atof(input: *const c_char) -> c_double;
    fn printf(format: *const c_char, ...) -> i32;
}

fn parse_like_atof(input: &[u8]) -> f32 {
    let input = CString::new(input).expect("arguments cannot contain NUL bytes");
    unsafe { atof(input.as_ptr()) as f32 }
}

fn q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5_f32;
    let mut y = number;
    let mut bits = y.to_bits();

    bits = 0x5f37_59df_u32.wrapping_sub(bits >> 1);
    y = f32::from_bits(bits);
    y = y * (1.5_f32 - (x2 * y * y));

    y
}

fn vector_normalize_fast(vector: &mut [f32; 3]) {
    let dot = vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2];
    let inverse_length = q_rsqrt(dot);

    vector[0] *= inverse_length;
    vector[1] *= inverse_length;
    vector[2] *= inverse_length;
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 {
        let program = args
            .first()
            .map_or(&[][..], |arg| arg.as_os_str().as_bytes());
        let mut message = Vec::with_capacity(program.len() + 19);
        message.extend_from_slice(program);
        message.extend_from_slice(b" requires 4 inputs\n");
        let _ = io::stderr().write_all(&message);
        process::exit(1);
    }

    let mut inputs = [
        parse_like_atof(args[1].as_os_str().as_bytes()),
        parse_like_atof(args[2].as_os_str().as_bytes()),
        parse_like_atof(args[3].as_os_str().as_bytes()),
    ];

    vector_normalize_fast(&mut inputs);

    unsafe {
        printf(
            b"%f %f %f\n\0".as_ptr().cast(),
            inputs[0] as c_double,
            inputs[1] as c_double,
            inputs[2] as c_double,
        );
    }
}
