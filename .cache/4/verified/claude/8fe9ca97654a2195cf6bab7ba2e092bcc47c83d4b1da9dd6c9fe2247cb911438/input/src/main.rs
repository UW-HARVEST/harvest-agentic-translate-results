//! Translation of c_src/src/main.c
//!
//! ```c
//! int main(int argc, char** argv) {
//!     vec3_t Inputs;
//!     if(argc != 4) {
//!         fprintf(stderr, "%s requires 4 inputs\n", argv[0]);
//!         exit(1);
//!     }
//!
//!     Inputs[0] = atof(argv[1]);
//!     Inputs[1] = atof(argv[2]);
//!     Inputs[2] = atof(argv[3]);
//!
//!     VectorNormalizeFast(Inputs);
//!
//!     printf("%f %f %f\n", Inputs[0], Inputs[1], Inputs[2]);
//!     return 0;
//! }
//! ```

mod cstd;
mod q_math;

use std::io::Write;

#[cfg(unix)]
fn os_bytes(s: &std::ffi::OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    s.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn os_bytes(s: &std::ffi::OsString) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}

fn main() {
    let argv: Vec<Vec<u8>> = std::env::args_os().map(|a| os_bytes(&a)).collect();
    let argc = argv.len();

    let mut inputs: q_math::Vec3 = [0.0f32; 3];

    if argc != 4 {
        let mut stderr = std::io::stderr();
        let mut msg: Vec<u8> = Vec::new();
        // argv[0] may be absent when the program is invoked with an empty argv.
        if let Some(a0) = argv.first() {
            msg.extend_from_slice(a0);
        }
        msg.extend_from_slice(b" requires 4 inputs\n");
        let _ = stderr.write_all(&msg);
        let _ = stderr.flush();
        std::process::exit(1);
    }

    inputs[0] = cstd::atof(&argv[1]) as f32;
    inputs[1] = cstd::atof(&argv[2]) as f32;
    inputs[2] = cstd::atof(&argv[3]) as f32;

    q_math::vector_normalize_fast(&mut inputs);

    let out = format!(
        "{} {} {}\n",
        cstd::printf_f(inputs[0] as f64),
        cstd::printf_f(inputs[1] as f64),
        cstd::printf_f(inputs[2] as f64)
    );
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}
