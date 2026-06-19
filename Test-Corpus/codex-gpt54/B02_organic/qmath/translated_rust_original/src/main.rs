use std::env;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::process;

type Vec3 = [f32; 3];

unsafe extern "C" {
    static mut stderr: *mut libc::FILE;
}

fn q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5_f32;
    let mut i = number.to_bits();
    i = 0x5f37_59df_u32.wrapping_sub(i >> 1);
    let mut y = f32::from_bits(i);
    y = y * (1.5_f32 - (x2 * y * y));
    y
}

fn vector_normalize_fast(v: &mut Vec3) {
    let ilength = q_rsqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

fn os_arg_to_cstring(arg: &std::ffi::OsStr) -> CString {
    CString::new(arg.as_bytes()).expect("argv entries cannot contain NUL bytes")
}

fn main() {
    let args: Vec<_> = env::args_os().collect();

    if args.len() != 4 {
        let program = args
            .first()
            .map(|arg| os_arg_to_cstring(arg.as_os_str()))
            .unwrap_or_else(|| CString::new("translated_rust").expect("static string is valid"));
        let format = CString::new("%s requires 4 inputs\n").expect("static string is valid");

        unsafe {
            libc::fprintf(stderr, format.as_ptr(), program.as_ptr());
        }
        process::exit(1);
    }

    let arg1 = os_arg_to_cstring(args[1].as_os_str());
    let arg2 = os_arg_to_cstring(args[2].as_os_str());
    let arg3 = os_arg_to_cstring(args[3].as_os_str());

    let mut inputs = [0.0_f32; 3];
    unsafe {
        inputs[0] = libc::atof(arg1.as_ptr()) as f32;
        inputs[1] = libc::atof(arg2.as_ptr()) as f32;
        inputs[2] = libc::atof(arg3.as_ptr()) as f32;
    }

    vector_normalize_fast(&mut inputs);

    let format = CString::new("%f %f %f\n").expect("static string is valid");
    unsafe {
        libc::printf(
            format.as_ptr(),
            f64::from(inputs[0]),
            f64::from(inputs[1]),
            f64::from(inputs[2]),
        );
    }
}
