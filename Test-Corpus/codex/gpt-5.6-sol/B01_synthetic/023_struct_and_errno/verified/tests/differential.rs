use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::mem::{size_of, MaybeUninit};
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

type RunFn = unsafe extern "C" fn(*mut House, c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
    fn _exit(status: c_int) -> !;
}

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_run: RunFn,
    rust_run: RunFn,
    c_main: MainFn,
    rust_main: MainFn,
}

impl Apis {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver_c.so");
        let rust_path = root.join("target/debug/libdriver.so");
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        let c_library = unsafe { Library::new(c_path).expect("load C shared library") };
        let rust_library = unsafe { Library::new(rust_path).expect("load Rust shared library") };
        let c_run = *unsafe { c_library.get::<RunFn>(b"run\0").expect("C run") };
        let rust_run = *unsafe { rust_library.get::<RunFn>(b"run\0").expect("Rust run") };
        let c_main = *unsafe { c_library.get::<MainFn>(b"main\0").expect("C main") };
        let rust_main = *unsafe { rust_library.get::<MainFn>(b"main\0").expect("Rust main") };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_run,
            rust_run,
            c_main,
            rust_main,
        }
    }
}

#[derive(Debug)]
struct ChildResult {
    stdout: Vec<u8>,
    data: Vec<u8>,
    signal: c_int,
    exit_code: c_int,
}

fn make_pipe() -> [c_int; 2] {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    fds
}

fn finish_child(pid: c_int, stdout_fd: c_int, data_fd: c_int) -> ChildResult {
    let mut stdout = Vec::new();
    let mut data = Vec::new();
    unsafe { File::from_raw_fd(stdout_fd) }
        .read_to_end(&mut stdout)
        .unwrap();
    unsafe { File::from_raw_fd(data_fd) }
        .read_to_end(&mut data)
        .unwrap();

    let mut status = 0;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
    let signal = status & 0x7f;
    let exit_code = if signal == 0 {
        (status >> 8) & 0xff
    } else {
        -1
    };
    ChildResult {
        stdout,
        data,
        signal,
        exit_code,
    }
}

fn call_run(function: RunFn, initial: House, extra_bedrooms: c_int) -> ChildResult {
    unsafe {
        fflush(ptr::null_mut());
    }
    let output_pipe = make_pipe();
    let data_pipe = make_pipe();
    let pid = unsafe { fork() };
    assert!(pid >= 0);

    if pid == 0 {
        unsafe {
            close(output_pipe[0]);
            close(data_pipe[0]);
            assert_eq!(dup2(output_pipe[1], 1), 1);
            close(output_pipe[1]);
            let mut house = initial;
            function(&mut house, extra_bedrooms);
            fflush(ptr::null_mut());
            let written = write(
                data_pipe[1],
                (&house as *const House).cast(),
                size_of::<House>(),
            );
            if written != size_of::<House>() as isize {
                _exit(120);
            }
            close(data_pipe[1]);
            _exit(0);
        }
    }

    unsafe {
        close(output_pipe[1]);
        close(data_pipe[1]);
    }
    finish_child(pid, output_pipe[0], data_pipe[0])
}

fn call_main(function: MainFn, input: &[u8]) -> ChildResult {
    unsafe {
        fflush(ptr::null_mut());
    }
    let input_pipe = make_pipe();
    let output_pipe = make_pipe();
    let data_pipe = make_pipe();
    let pid = unsafe { fork() };
    assert!(pid >= 0);

    if pid == 0 {
        unsafe {
            close(input_pipe[1]);
            close(output_pipe[0]);
            close(data_pipe[0]);
            assert_eq!(dup2(input_pipe[0], 0), 0);
            assert_eq!(dup2(output_pipe[1], 1), 1);
            close(input_pipe[0]);
            close(output_pipe[1]);
            let result = function();
            fflush(ptr::null_mut());
            let written = write(
                data_pipe[1],
                (&result as *const c_int).cast(),
                size_of::<c_int>(),
            );
            if written != size_of::<c_int>() as isize {
                _exit(121);
            }
            close(data_pipe[1]);
            _exit(0);
        }
    }

    unsafe {
        close(input_pipe[0]);
        close(output_pipe[1]);
        close(data_pipe[1]);
    }
    let mut input_file = unsafe { File::from_raw_fd(input_pipe[1]) };
    input_file.write_all(input).unwrap();
    drop(input_file);
    finish_child(pid, output_pipe[0], data_pipe[0])
}

fn call_null_run(function: RunFn) -> ChildResult {
    unsafe {
        fflush(ptr::null_mut());
    }
    let output_pipe = make_pipe();
    let data_pipe = make_pipe();
    let pid = unsafe { fork() };
    assert!(pid >= 0);

    if pid == 0 {
        unsafe {
            close(output_pipe[0]);
            close(data_pipe[0]);
            assert_eq!(dup2(output_pipe[1], 1), 1);
            close(output_pipe[1]);
            function(ptr::null_mut(), 0);
            fflush(ptr::null_mut());
            close(data_pipe[1]);
            _exit(0);
        }
    }

    unsafe {
        close(output_pipe[1]);
        close(data_pipe[1]);
    }
    finish_child(pid, output_pipe[0], data_pipe[0])
}

fn assert_same(context: &str, c: ChildResult, rust: ChildResult) {
    assert_eq!(rust.signal, c.signal, "{context}: signal");
    assert_eq!(rust.exit_code, c.exit_code, "{context}: exit code");
    assert_eq!(rust.stdout, c.stdout, "{context}: stdout");
    assert_eq!(rust.data, c.data, "{context}: result bytes");
}

fn compare_run(apis: &Apis, row: usize, case: usize, house: House, extra: c_int) {
    let c = call_run(apis.c_run, house, extra);
    let rust = call_run(apis.rust_run, house, extra);
    assert_same(&format!("CONFIGS row {row}, case {case}"), c, rust);
}

fn compare_main(apis: &Apis, table: &str, row: usize, case: usize, input: &[u8]) {
    let c = call_main(apis.c_main, input);
    let rust = call_main(apis.rust_main, input);
    assert_same(&format!("{table} row {row}, case {case}"), c, rust);
}

struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn range(&mut self, upper: u32) -> u32 {
        self.next_u32() % upper
    }

    fn ordinary_house(&mut self) -> House {
        House {
            floors: self.range(1000) as c_int - 500,
            bedrooms: self.range(100_000) as c_int - 50_000,
            bathrooms: (self.range(20_001) as f64 - 10_000.0) / 10.0,
        }
    }
}

#[test]
fn dynamic_symbols_are_loadable() {
    let _apis = unsafe { Apis::load() };
}

#[test]
fn valid_configuration_surface_matches() {
    let apis = unsafe { Apis::load() };
    let mut rng = Rng(0x4d59_5df4_d0f3_3173);

    for case in 0..32 {
        compare_run(&apis, 1, case, rng.ordinary_house(), 0);
        compare_run(
            &apis,
            2,
            case,
            rng.ordinary_house(),
            rng.range(10_000) as c_int + 1,
        );
        compare_run(
            &apis,
            3,
            case,
            rng.ordinary_house(),
            -(rng.range(10_000) as c_int) - 1,
        );

        let mut house = rng.ordinary_house();
        house.floors = c_int::MAX;
        compare_run(&apis, 4, case, house, rng.range(100) as c_int);

        let mut house = rng.ordinary_house();
        house.bedrooms = c_int::MAX - rng.range(1000) as c_int;
        compare_run(&apis, 5, case, house, rng.range(1000) as c_int + 1001);

        let mut house = rng.ordinary_house();
        house.bedrooms = c_int::MIN + rng.range(1000) as c_int;
        compare_run(&apis, 6, case, house, -(rng.range(1000) as c_int) - 1001);

        let mut house = rng.ordinary_house();
        let whole = rng.range(2000) as f64 - 1000.0;
        let fractions = [0.049, 0.05, 0.051, 0.149, 0.15, -0.0];
        house.bathrooms = whole + fractions[case % fractions.len()];
        compare_run(&apis, 7, case, house, rng.range(200) as c_int - 100);
    }

    let non_finite = [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0xfff8_1234_5678_9abc),
    ];
    for case in 0..32 {
        let mut house = rng.ordinary_house();
        house.bathrooms = non_finite[case % non_finite.len()];
        compare_run(&apis, 8, case, house, rng.next_u32() as c_int);
    }

    for case in 0..32 {
        let value = rng.range(1_000_000);
        compare_main(&apis, "CONFIGS", 9, case, format!("{value}\n").as_bytes());

        let value = rng.range(1_000_000) + 1;
        compare_main(&apis, "CONFIGS", 10, case, format!("-{value}\n").as_bytes());

        let value = rng.range(1_000_000);
        compare_main(&apis, "CONFIGS", 11, case, format!("+{value}\n").as_bytes());

        let whitespace = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];
        let mut input = vec![whitespace[case % whitespace.len()]; case % 7 + 1];
        input.extend_from_slice(value.to_string().as_bytes());
        input.push(b'\n');
        compare_main(&apis, "CONFIGS", 12, case, &input);

        let suffixes: [&[u8]; 4] = [b"xyz\n", b".5\n", b" x\n", b"+9\n"];
        let input = [value.to_string().as_bytes(), suffixes[case % 4]].concat();
        compare_main(&apis, "CONFIGS", 13, case, &input);

        let leading_zeros = "0".repeat(case % 12);
        compare_main(
            &apis,
            "CONFIGS",
            14,
            case,
            format!("{leading_zeros}2147483647 tail\n").as_bytes(),
        );
        compare_main(
            &apis,
            "CONFIGS",
            15,
            case,
            format!("-{leading_zeros}2147483648 tail\n").as_bytes(),
        );

        let mut input = value.to_string().into_bytes();
        input.push(0);
        input.extend((0..case % 13).map(|_| (rng.range(255) + 1) as u8));
        input.push(b'\n');
        compare_main(&apis, "CONFIGS", 16, case, &input);

        let mut input = vec![b'0'; 98];
        input.push(b'0' + rng.range(10) as u8);
        compare_main(&apis, "CONFIGS", 17, case, &input);

        compare_main(&apis, "CONFIGS", 18, case, value.to_string().as_bytes());
    }
}

#[test]
fn error_surface_matches() {
    let apis = unsafe { Apis::load() };
    let no_conversion: [&[u8]; 12] = [
        b"",
        b"\n",
        b" ",
        b"\t\r\n",
        b"+",
        b"-",
        b"+\n",
        b"- \n",
        b"x",
        b"x123\n",
        b".5\n",
        b"\0digits\n",
    ];
    for case in 0..32 {
        compare_main(
            &apis,
            "ERRORS",
            1,
            case,
            no_conversion[case % no_conversion.len()],
        );
    }

    for case in 0..32 {
        let zeros = "0".repeat(case % 8);
        let sign = if case % 2 == 0 { "" } else { "-" };
        let input = format!("{sign}{zeros}999999999999999999999999999999999999\n");
        compare_main(&apis, "ERRORS", 2, case, input.as_bytes());

        let below = i64::from(c_int::MIN) - 1 - case as i64;
        compare_main(&apis, "ERRORS", 3, case, format!("{below}\n").as_bytes());

        let above = i64::from(c_int::MAX) + 1 + case as i64;
        compare_main(&apis, "ERRORS", 4, case, format!("{above}\n").as_bytes());
    }

    let c = call_null_run(apis.c_run);
    let rust = call_null_run(apis.rust_run);
    assert_same("ERRORS row 5", c, rust);
}

#[test]
fn ffi_layout_is_exactly_sixteen_bytes() {
    assert_eq!(size_of::<House>(), 16);
    assert_eq!(size_of::<MaybeUninit<House>>(), 16);
}
