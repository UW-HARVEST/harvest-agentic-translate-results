use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::path::{Path, PathBuf};

type DriverFn = unsafe extern "C" fn(c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

#[derive(Clone, Copy)]
enum Invocation<'a> {
    Driver(&'a [i32]),
    Main { calls: usize },
}

fn libraries() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = root.join("c_src/build/libdriver_c.so");
    let rust = root.join("target/debug/libdriver.so");
    assert!(c.is_file(), "missing C shared library: {}", c.display());
    assert!(
        rust.is_file(),
        "missing Rust shared library: {}",
        rust.display()
    );
    (c, rust)
}

fn make_pipe() -> (RawFd, RawFd) {
    let mut fds = [-1; 2];
    let result = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(
        result,
        0,
        "pipe failed: {}",
        std::io::Error::last_os_error()
    );
    (fds[0], fds[1])
}

fn child_run(path: &Path, invocation: Invocation<'_>) -> ! {
    let library = match unsafe { Library::new(path) } {
        Ok(library) => library,
        Err(_) => unsafe { _exit(110) },
    };

    match invocation {
        Invocation::Driver(values) => {
            let driver: Symbol<'_, DriverFn> = match unsafe { library.get(b"driver\0") } {
                Ok(driver) => driver,
                Err(_) => unsafe { _exit(111) },
            };
            for value in values {
                unsafe { driver(*value) };
            }
        }
        Invocation::Main { calls } => {
            let main: Symbol<'_, MainFn> = match unsafe { library.get(b"main\0") } {
                Ok(main) => main,
                Err(_) => unsafe { _exit(112) },
            };
            for _ in 0..calls {
                if unsafe { main() } != 0 {
                    unsafe { _exit(113) };
                }
            }
        }
    }

    unsafe {
        fflush(std::ptr::null_mut());
        _exit(0);
    }
}

fn invoke(path: &Path, input: &[u8], invocation: Invocation<'_>) -> Vec<u8> {
    let (input_read, input_write) = make_pipe();
    let (output_read, output_write) = make_pipe();
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed: {}", std::io::Error::last_os_error());

    if pid == 0 {
        unsafe {
            close(input_write);
            close(output_read);
            if dup2(input_read, 0) < 0 || dup2(output_write, 1) < 0 {
                _exit(114);
            }
            close(input_read);
            close(output_write);
        }
        child_run(path, invocation);
    }

    unsafe {
        close(input_read);
        close(output_write);
    }

    let mut input_file = unsafe { File::from_raw_fd(input_write) };
    input_file.write_all(input).expect("write child stdin");
    drop(input_file);

    let mut output = Vec::new();
    let mut output_file = unsafe { File::from_raw_fd(output_read) };
    output_file
        .read_to_end(&mut output)
        .expect("read child stdout");

    let mut status = -1;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
    assert_eq!(status, 0, "{} child wait status {status}", path.display());
    output
}

fn compare_driver(c: &Path, rust: &Path, values: &[i32]) {
    let c_output = invoke(c, &[], Invocation::Driver(values));
    let rust_output = invoke(rust, &[], Invocation::Driver(values));
    assert_eq!(rust_output, c_output);
}

fn compare_main(c: &Path, rust: &Path, input: &[u8], calls: usize) {
    let c_output = invoke(c, input, Invocation::Main { calls });
    let rust_output = invoke(rust, input, Invocation::Main { calls });
    assert_eq!(
        rust_output,
        c_output,
        "different output for stdin {:?}",
        String::from_utf8_lossy(input)
    );
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn phase_b_all_configuration_rows_match() {
    let (c, rust) = libraries();
    let mut state = 0x8d26_7a4f_c915_30e1;

    // CONFIGS row 1: full-domain driver inputs.
    let mut values = vec![i32::MIN, -1, 0, 1, i32::MAX];
    for _ in 0..1024 {
        values.push(next_random(&mut state) as i32);
    }
    compare_driver(&c, &rust, &values);

    // CONFIGS row 2: whitespace and explicitly signed decimal tokens.
    let mut signed_stream = Vec::new();
    let mut signed_count = 0;
    for index in 0..256 {
        let value = next_random(&mut state) as i32;
        let whitespace = match index % 4 {
            0 => " ",
            1 => "\n",
            2 => "\t",
            _ => "\r\n",
        };
        let token = if value >= 0 && index % 2 == 0 {
            format!("{whitespace}+{value}")
        } else {
            format!("{whitespace}{value}")
        };
        signed_stream.extend_from_slice(token.as_bytes());
        signed_count += 1;
    }
    compare_main(&c, &rust, &signed_stream, signed_count);

    // CONFIGS row 3: EOF retains initialized zero.
    compare_main(&c, &rust, b"", 1);

    // CONFIGS row 4: randomized nonnumeric first bytes fail conversion.
    let invalid_initial =
        b"!\"#$%&'()*,./:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~";
    for _ in 0..64 {
        let byte = invalid_initial[next_random(&mut state) as usize % invalid_initial.len()];
        compare_main(&c, &rust, &[byte], 1);
    }

    // CONFIGS row 5: scanf consumes randomized numeric prefixes and leaves suffixes.
    for _ in 0..64 {
        let value = next_random(&mut state) as i32;
        let suffix = invalid_initial[next_random(&mut state) as usize % invalid_initial.len()];
        let mut input = value.to_string().into_bytes();
        input.push(suffix);
        compare_main(&c, &rust, &input, 1);
    }

    // CONFIGS row 6: repeated calls consume successive tokens from one stream.
    let mut repeated_stream = Vec::new();
    let mut repeated_count = 0;
    for index in 0..512 {
        let value = next_random(&mut state) as i32;
        let token = format!("{value}{}", if index % 3 == 0 { "\n" } else { " " });
        repeated_stream.extend_from_slice(token.as_bytes());
        repeated_count += 1;
    }
    compare_main(&c, &rust, &repeated_stream, repeated_count);

    // CONFIGS row 7: randomized samples of this platform's scanf overflow behavior.
    for index in 0..128 {
        let excess = 1 + next_random(&mut state) % 4_000_000_000;
        let input = if index % 2 == 0 {
            (i32::MAX as u64 + excess).to_string()
        } else {
            format!("-{}", i32::MAX as u64 + 1 + excess)
        };
        compare_main(&c, &rust, input.as_bytes(), 1);
    }
    for digits in [40, 127, 511] {
        let positive = "9".repeat(digits);
        let negative = format!("-{positive}");
        compare_main(&c, &rust, positive.as_bytes(), 1);
        compare_main(&c, &rust, negative.as_bytes(), 1);
    }
}

#[test]
fn phase_c_has_no_explicit_rejections_and_generic_boundaries_match() {
    let (c, rust) = libraries();

    // The FFI has no pointer, length, or enum arguments. These are the
    // applicable EOF, malformed, boundary, and oversized main inputs.
    for input in [
        b"".as_slice(),
        b"?".as_slice(),
        b"0".as_slice(),
        b"2147483647".as_slice(),
        b"-2147483648".as_slice(),
        b"2147483648".as_slice(),
        b"-2147483649".as_slice(),
    ] {
        compare_main(&c, &rust, input, 1);
    }
}
