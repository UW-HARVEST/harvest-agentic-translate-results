use libloading::Library;
use std::ffi::{c_int, c_uint, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::ptr;

type PrintFoo = unsafe extern "C" fn(*const c_void);
type Driver = unsafe extern "C" fn(c_uint, c_uint, bool, c_int);
type Main = unsafe extern "C" fn() -> c_int;

#[repr(C)]
struct Foo {
    bit_fields: c_uint,
    z: c_int,
}

#[repr(C)]
struct CFile {
    _private: [u8; 0],
}

extern "C" {
    fn clearerr(stream: *mut CFile);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut CFile) -> c_int;
    fn fork() -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;

    static mut stdin: *mut CFile;
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

fn shared_objects() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("c_src/build/libdriver_c.so"),
        root.join("target/debug/deps/libdriver.so"),
    )
}

fn assert_shared_object(path: &Path) {
    assert!(
        path.is_file(),
        "shared object does not exist: {}; build both libraries first",
        path.display()
    );
}

unsafe fn make_pipe() -> (RawFd, RawFd) {
    let mut fds = [-1; 2];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    (fds[0], fds[1])
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    assert_eq!(fflush(ptr::null_mut()), 0);
    let (read_fd, write_fd) = make_pipe();
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0);
    assert_eq!(dup2(write_fd, 1), 1);
    assert_eq!(close(write_fd), 0);

    call();

    assert_eq!(fflush(ptr::null_mut()), 0);
    assert_eq!(dup2(saved_stdout, 1), 1);
    assert_eq!(close(saved_stdout), 0);

    let mut output = Vec::new();
    File::from_raw_fd(read_fd)
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

unsafe fn invoke_main(function: Main, input: &[u8]) -> (c_int, Vec<u8>) {
    assert_eq!(fflush(ptr::null_mut()), 0);
    let (input_read, input_write) = make_pipe();
    let (output_read, output_write) = make_pipe();

    {
        let mut writer = File::from_raw_fd(input_write);
        writer.write_all(input).expect("write child stdin");
    }

    let pid = fork();
    assert!(pid >= 0);
    if pid == 0 {
        assert_eq!(dup2(input_read, 0), 0);
        assert_eq!(dup2(output_write, 1), 1);
        close(input_read);
        close(output_read);
        close(output_write);
        clearerr(stdin);

        let result = function();
        fflush(ptr::null_mut());
        _exit(result & 0xff);
    }

    assert_eq!(close(input_read), 0);
    assert_eq!(close(output_write), 0);
    let mut output = Vec::new();
    File::from_raw_fd(output_read)
        .read_to_end(&mut output)
        .expect("read child stdout");

    let mut status = 0;
    assert_eq!(waitpid(pid, &mut status, 0), pid);
    (status, output)
}

unsafe fn termination_status(function: PrintFoo) -> c_int {
    assert_eq!(fflush(ptr::null_mut()), 0);
    let pid = fork();
    assert!(pid >= 0);
    if pid == 0 {
        function(ptr::null());
        _exit(0);
    }

    let mut status = 0;
    assert_eq!(waitpid(pid, &mut status, 0), pid);
    status
}

fn z_value(case: usize, rng: &mut Rng) -> i32 {
    match case {
        0 => i32::MIN,
        1 => i32::MAX,
        2 => 0,
        3 => -1,
        _ => rng.next_i32(),
    }
}

fn bounded_value(case: usize, mask: u32, fits: bool, rng: &mut Rng) -> u32 {
    if fits {
        match case {
            0 => 0,
            1 => mask,
            _ => rng.next_u32() & mask,
        }
    } else {
        match case {
            0 => mask + 1,
            1 => u32::MAX,
            _ => rng.next_u32() | (mask + 1),
        }
    }
}

fn scanned_bool(case: usize, enabled: bool, rng: &mut Rng) -> i32 {
    if !enabled {
        return 0;
    }
    match case {
        0 => 1,
        1 => -1,
        2 => i32::MIN,
        3 => i32::MAX,
        _ => {
            let value = rng.next_i32();
            if value == 0 {
                1
            } else {
                value
            }
        }
    }
}

fn assert_same(row: usize, case: usize, c: &[u8], rust: &[u8]) {
    assert_eq!(c, rust, "CONFIGS.md row {row}, randomized case {case}");
}

#[test]
fn all_dynamic_ffi_surfaces_match() {
    let (c_path, rust_path) = shared_objects();
    assert_shared_object(&c_path);
    assert_shared_object(&rust_path);

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared object");
        let rust_library = Library::new(&rust_path).expect("load Rust shared object");

        let c_print_foo: PrintFoo = *c_library.get(b"print_foo\0").expect("C print_foo");
        let rust_print_foo: PrintFoo = *rust_library.get(b"print_foo\0").expect("Rust print_foo");
        let c_driver: Driver = *c_library.get(b"driver\0").expect("C driver");
        let rust_driver: Driver = *rust_library.get(b"driver\0").expect("Rust driver");
        let c_main: Main = *c_library.get(b"main\0").expect("C main");
        let rust_main: Main = *rust_library.get(b"main\0").expect("Rust main");

        let mut rng = Rng::new(0x6f4d_31a2_b987_c5e1);

        // CONFIGS.md row 1: exact field cross-product plus randomized padding/z.
        for case in 0..64 {
            let x = (case & 3) as u32;
            let y = ((case >> 2) & 7) as u32;
            let b = ((case >> 5) & 1) as u32;
            let foo = Foo {
                bit_fields: (rng.next_u32() & !0x3f) | x | (y << 2) | (b << 5),
                z: z_value(case, &mut rng),
            };
            let pointer = (&foo as *const Foo).cast();
            let c_output = capture_stdout(|| c_print_foo(pointer));
            let rust_output = capture_stdout(|| rust_print_foo(pointer));
            assert_same(1, case, &c_output, &rust_output);
        }

        // CONFIGS.md rows 2-9: fit/overflow for both bit fields and both booleans.
        for x_fits in [true, false] {
            for y_fits in [true, false] {
                for b in [false, true] {
                    let row = 2 + (!x_fits as usize) * 4 + (!y_fits as usize) * 2 + b as usize;
                    for case in 0..64 {
                        let x = bounded_value(case, 3, x_fits, &mut rng);
                        let y = bounded_value(case, 7, y_fits, &mut rng);
                        let z = z_value(case, &mut rng);
                        let c_output = capture_stdout(|| c_driver(x, y, b, z));
                        let rust_output = capture_stdout(|| rust_driver(x, y, b, z));
                        assert_same(row, case, &c_output, &rust_output);
                    }
                }
            }
        }

        // CONFIGS.md rows 10-17: successful scans through the public main wrapper.
        for x_fits in [true, false] {
            for y_fits in [true, false] {
                for b_enabled in [false, true] {
                    let row =
                        10 + (!x_fits as usize) * 4 + (!y_fits as usize) * 2 + b_enabled as usize;
                    for case in 0..32 {
                        let x = bounded_value(case, 3, x_fits, &mut rng);
                        let y = bounded_value(case, 7, y_fits, &mut rng);
                        let b = scanned_bool(case, b_enabled, &mut rng);
                        let z = z_value(case, &mut rng);
                        let input = format!("{x}\n{y}\n{b}\n{z}\n");
                        let c_result = invoke_main(c_main, input.as_bytes());
                        let rust_result = invoke_main(rust_main, input.as_bytes());
                        assert_eq!(
                            c_result, rust_result,
                            "CONFIGS.md row {row}, randomized case {case}"
                        );
                    }
                }
            }
        }

        // CONFIGS.md rows 18-25: EOF or a non-numeric token at each scan.
        for malformed in [false, true] {
            for failed_scan in 0..4 {
                let row = 18 + malformed as usize * 4 + failed_scan;
                for case in 0..32 {
                    let values = [
                        rng.next_u32().to_string(),
                        rng.next_u32().to_string(),
                        rng.next_i32().to_string(),
                        z_value(case, &mut rng).to_string(),
                    ];
                    let mut input = values[..failed_scan].join("\n");
                    if failed_scan != 0 {
                        input.push('\n');
                    }
                    if malformed {
                        input.push_str("not-a-number\n");
                    }

                    let c_result = invoke_main(c_main, input.as_bytes());
                    let rust_result = invoke_main(rust_main, input.as_bytes());
                    assert_eq!(
                        c_result, rust_result,
                        "CONFIGS.md row {row}, randomized case {case}"
                    );
                }
            }
        }

        // ERRORS.md has no rows. The only pointer boundary is unchecked by C,
        // so compare its process termination rather than inventing an error code.
        let c_null_status = termination_status(c_print_foo);
        let rust_null_status = termination_status(rust_print_foo);
        assert_eq!(
            c_null_status, rust_null_status,
            "null print_foo termination differs"
        );
        assert_ne!(c_null_status, 0, "C unexpectedly accepted a null foo");
    }
}
