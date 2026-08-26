//! Rust translation of the Quake III Arena math library that
//! `c_src/CMakeLists.txt` builds (`src/q_math.c` + `src/main.c`, with
//! `inc/q_shared.h`).
//!
//! The crate is built as a `cdylib` so that it exports exactly the same C ABI
//! symbols as the shared object produced from the C sources, and as an `rlib`
//! so that the `driver` binary can reuse the translated `main`.
//!
//! Module map:
//!
//! | Rust module        | C source                                              |
//! |--------------------|-------------------------------------------------------|
//! | [`q_shared`]       | `inc/q_shared.h` (types, macros, `static ID_INLINE`)  |
//! | [`q_math`]         | `src/q_math.c`                                        |
//! | [`wrappers`]       | test-only `w_*` exports for the header's inline code   |
//! | [`cstd`]           | the `atof` / `printf("%f")` behaviour used by `main.c` |

pub mod cstd;
pub mod q_math;
pub mod q_shared;
pub mod wrappers;

use core::ffi::{c_char, c_int};

/// Translation of `c_src/src/main.c`:
///
/// ```c
/// int main(int argc, char** argv) {
///     vec3_t Inputs;
///     if(argc != 4) {
///         fprintf(stderr, "%s requires 4 inputs\n", argv[0]);
///         exit(1);
///     }
///
///     Inputs[0] = atof(argv[1]);
///     Inputs[1] = atof(argv[2]);
///     Inputs[2] = atof(argv[3]);
///
///     VectorNormalizeFast(Inputs);
///
///     printf("%f %f %f\n", Inputs[0], Inputs[1], Inputs[2]);
///     return 0;
/// }
/// ```
///
/// `argv` is handled as raw bytes so that non-UTF-8 arguments behave exactly
/// like they do in C.  Returns the process exit status; `exit(1)` is a real
/// `std::process::exit(1)` because `exit()` does not return in C either.
pub fn driver_main(argv: &[Vec<u8>]) -> i32 {
    use std::io::Write;

    let argc = argv.len();

    let mut inputs: [f32; 3] = [0.0; 3];

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

    unsafe { q_shared::VectorNormalizeFast(inputs.as_mut_ptr()) };

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
    0
}

/// `int main(int argc, char** argv)` from `c_src/src/main.c`, exported with C
/// linkage so that the shared library has the same symbol table as the C one.
///
/// # Safety
/// `argv` must point to `argc` valid NUL-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    let mut args: Vec<Vec<u8>> = Vec::new();
    if !argv.is_null() {
        for i in 0..argc.max(0) as usize {
            let p = *argv.add(i);
            if p.is_null() {
                break;
            }
            args.push(core::ffi::CStr::from_ptr(p).to_bytes().to_vec());
        }
    }
    // `argc` drives the error branch in main.c, not the number of non-NULL
    // pointers, so use it verbatim when it disagrees with what we could read.
    if args.len() != argc as usize {
        args.resize(argc.max(0) as usize, Vec::new());
    }
    driver_main(&args)
}
