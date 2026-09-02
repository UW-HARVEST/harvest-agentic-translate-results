mod clib;
mod fpu;
mod q_math;
mod q_shared;

use std::hint::black_box;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

use clib::{atof, format_f, narrow_to_f32, widen_to_f64};
use q_shared::{vector_normalize_fast, Vec3};

fn main() {
    // Raw argument bytes, matching C's `argv` (which is not required to be
    // valid UTF-8).
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_bytes().to_vec())
        .collect();
    let argc = argv.len();

    let mut inputs: Vec3 = [0.0; 3];

    if argc != 4 {
        let prog: &[u8] = argv.first().map(|a| a.as_slice()).unwrap_or(b"");
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(prog);
        let _ = stderr.write_all(b" requires 4 inputs\n");
        let _ = stderr.flush();
        std::process::exit(1);
    }

    // `atof` returns a double which is then truncated to float on assignment.
    inputs[0] = narrow_to_f32(atof(&argv[1]));
    inputs[1] = narrow_to_f32(atof(&argv[2]));
    inputs[2] = narrow_to_f32(atof(&argv[3]));

    // In the C program `Inputs` is filled from runtime `argv` data through an
    // opaque libc call, so the compiler cannot fold the float arithmetic that
    // follows. Preserve that property: LLVM *will* fold this arithmetic when it
    // can see through `atof`, and its folder does not reproduce the x86 NaN
    // sign-propagation rules, which changes the printed output for NaN inputs.
    let mut inputs = black_box(inputs);

    vector_normalize_fast(&mut inputs);

    // printf("%f %f %f\n", ...) — the floats are promoted to double.
    let out = format!(
        "{} {} {}\n",
        format_f(widen_to_f64(inputs[0])),
        format_f(widen_to_f64(inputs[1])),
        format_f(widen_to_f64(inputs[2]))
    );
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}
