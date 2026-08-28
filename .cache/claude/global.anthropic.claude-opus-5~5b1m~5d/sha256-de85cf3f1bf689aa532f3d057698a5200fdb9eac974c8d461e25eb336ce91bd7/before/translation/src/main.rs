//! Port of `main.c`.

mod cstd;
mod fpu;
mod q_math;
mod q_shared;

use std::io::Write;
use std::os::unix::ffi::OsStrExt;

use cstd::{atof, printf_f_float};
use q_shared::{vector_normalize_fast, Vec3};

fn main() {
    // argv as raw bytes, exactly as C sees it.
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_os_str().as_bytes().to_vec())
        .collect();
    let argc = argv.len();

    let mut inputs: Vec3 = [0.0; 3];

    if argc != 4 {
        // fprintf(stderr, "%s requires 4 inputs\n", argv[0]);
        let arg0: &[u8] = if argc > 0 { &argv[0] } else { b"(null)" };
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();
        let _ = stderr.write_all(arg0);
        let _ = stderr.write_all(b" requires 4 inputs\n");
        let _ = stderr.flush();
        std::process::exit(1);
    }

    inputs[0] = atof(&argv[1]) as f32;
    inputs[1] = atof(&argv[2]) as f32;
    inputs[2] = atof(&argv[3]) as f32;

    vector_normalize_fast(&mut inputs);

    // printf("%f %f %f\n", Inputs[0], Inputs[1], Inputs[2]);
    let line = format!(
        "{} {} {}\n",
        printf_f_float(inputs[0]),
        printf_f_float(inputs[1]),
        printf_f_float(inputs[2])
    );
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.flush();
}
