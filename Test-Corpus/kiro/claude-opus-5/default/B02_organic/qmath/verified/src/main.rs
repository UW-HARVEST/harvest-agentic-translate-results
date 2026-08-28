//! Port of `c_src/src/main.c`.

mod cstd;
mod q_math;
mod q_shared;
mod sse;

use std::io::Write;

use cstd::{atof, format_f};
use q_shared::{vector_normalize_fast, Vec3};

/// Raw bytes of an argument, exactly as C's `argv` would see them.
fn arg_bytes(arg: &std::ffi::OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().into_owned().into_bytes()
    }
}

fn main() {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = argv.len();

    let mut inputs: Vec3 = [0.0; 3];

    if argc != 4 {
        let mut stderr = std::io::stderr();
        let program = argv.first().map(arg_bytes).unwrap_or_default();
        let _ = stderr.write_all(&program);
        let _ = stderr.write_all(b" requires 4 inputs\n");
        let _ = stderr.flush();
        std::process::exit(1);
    }

    // atof() yields a double which is then narrowed to the float element type.
    inputs[0] = atof(&arg_bytes(&argv[1])) as f32;
    inputs[1] = atof(&arg_bytes(&argv[2])) as f32;
    inputs[2] = atof(&arg_bytes(&argv[3])) as f32;

    vector_normalize_fast(&mut inputs);

    // printf("%f %f %f\n", ...) — the floats are promoted to double first.
    let line = format!(
        "{} {} {}\n",
        format_f(inputs[0] as f64),
        format_f(inputs[1] as f64),
        format_f(inputs[2] as f64)
    );
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.flush();
}
