use std::env;
use std::ffi::CString;
use std::io::{self, Write};
use std::os::raw::{c_char, c_double, c_int};
use std::os::unix::ffi::OsStrExt;

unsafe extern "C" {
    fn atof(nptr: *const c_char) -> c_double;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5_f32;
    let mut i = number.to_bits();
    i = 0x5f37_59df_u32.wrapping_sub(i >> 1);
    let mut y = f32::from_bits(i);
    y = y * (1.5_f32 - (x2 * y * y));
    y
}

fn vector_normalize_fast(v: &mut [f32; 3]) {
    let dot = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    let ilength = q_rsqrt(dot);

    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

fn c_atof(bytes: &[u8]) -> f32 {
    let c_string = CString::new(bytes).expect("argv entries cannot contain interior NUL bytes");
    unsafe { atof(c_string.as_ptr()) as f32 }
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 4 {
        let mut stderr = io::stderr().lock();
        let _ = stderr.write_all(args[0].as_os_str().as_bytes());
        let _ = stderr.write_all(b" requires 4 inputs\n");
        std::process::exit(1);
    }

    let mut inputs = [
        c_atof(args[1].as_os_str().as_bytes()),
        c_atof(args[2].as_os_str().as_bytes()),
        c_atof(args[3].as_os_str().as_bytes()),
    ];

    vector_normalize_fast(&mut inputs);

    unsafe {
        printf(
            c"%f %f %f\n".as_ptr(),
            inputs[0] as c_double,
            inputs[1] as c_double,
            inputs[2] as c_double,
        );
    }
}
