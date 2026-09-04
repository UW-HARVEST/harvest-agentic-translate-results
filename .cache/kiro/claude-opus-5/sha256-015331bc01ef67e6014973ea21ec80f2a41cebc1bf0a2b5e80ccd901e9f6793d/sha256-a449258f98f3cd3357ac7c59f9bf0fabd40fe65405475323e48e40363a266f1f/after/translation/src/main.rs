mod clib;
mod fops;
mod q_math;
mod q_shared;

use std::io::Write;
use std::os::unix::ffi::OsStrExt;

use clib::{atof, format_f};
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
    inputs[0] = atof(&argv[1]) as f32;
    inputs[1] = atof(&argv[2]) as f32;
    inputs[2] = atof(&argv[3]) as f32;

    vector_normalize_fast(&mut inputs);

    // printf("%f %f %f\n", ...) — the floats are promoted to double.
    let out = format!(
        "{} {} {}\n",
        format_f(inputs[0] as f64),
        format_f(inputs[1] as f64),
        format_f(inputs[2] as f64)
    );
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}
