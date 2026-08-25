use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

type Foo = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
type Driver = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const SIGSEGV: i32 = 11;
const RANDOM_CASES: usize = 192;

struct LoadedApis {
    _c_library: Library,
    _rust_library: Library,
    c_foo: Foo,
    rust_foo: Foo,
}

impl LoadedApis {
    fn load() -> Self {
        let c_library = unsafe { Library::new(c_library_path()) }
            .expect("failed to load the C reference library");
        let rust_library =
            unsafe { Library::new(rust_library_path()) }.expect("failed to load the Rust library");

        let c_foo = load_symbol::<Foo>(&c_library, b"foo\0");
        let rust_foo = load_symbol::<Foo>(&rust_library, b"foo\0");

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_foo,
            rust_foo,
        }
    }
}

fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
    let symbol: Symbol<'_, T> = unsafe { library.get(name) }.unwrap_or_else(|error| {
        panic!(
            "failed to load symbol {}: {error}",
            String::from_utf8_lossy(&name[..name.len() - 1])
        )
    });
    *symbol
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = env::var_os("DRIVER_RUST_LIBRARY") {
        return path.into();
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

#[derive(Clone, Copy)]
enum CountClass {
    Zero,
    One,
    Many,
}

impl CountClass {
    fn count(self, rng: &mut Rng) -> usize {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Many => 2 + rng.range(7),
        }
    }
}

#[derive(Clone, Copy)]
enum ByteClass {
    Ascii,
    HighBit,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn range(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }

    fn nonzero_byte_except(&mut self, excluded: &[u8]) -> u8 {
        loop {
            let byte = (self.next_u64() & 0xff) as u8;
            if byte != 0 && !excluded.contains(&byte) {
                return byte;
            }
        }
    }
}

fn shuffled_input(rng: &mut Rng, counts: &[(u8, usize)], minimum_extra: usize) -> Vec<u8> {
    let mut input = Vec::new();
    for &(byte, count) in counts {
        input.extend(std::iter::repeat_n(byte, count));
    }

    let extra = minimum_extra + rng.range(97);
    let excluded: Vec<u8> = counts.iter().map(|&(byte, _)| byte).collect();
    input.extend((0..extra).map(|_| rng.nonzero_byte_except(&excluded)));

    for index in (1..input.len()).rev() {
        let swap_with = rng.range(index + 1);
        input.swap(index, swap_with);
    }
    input
}

fn assert_foo_class(byte_class: ByteClass, count_class: CountClass, seed: u64) {
    let apis = LoadedApis::load();
    let mut rng = Rng::new(seed);

    for case in 0..RANDOM_CASES {
        let needle = match byte_class {
            ByteClass::Ascii => 1 + rng.range(0x7f) as u8,
            ByteClass::HighBit => 0x80 + rng.range(0x80) as u8,
        };
        let expected = count_class.count(&mut rng);
        let minimum_extra = if expected == 0 { case % 3 } else { 0 };
        let mut input = shuffled_input(&mut rng, &[(needle, expected)], minimum_extra);
        input.push(0);

        let c_result = unsafe { (apis.c_foo)(input.as_ptr().cast(), needle as c_char) };
        let rust_result = unsafe { (apis.rust_foo)(input.as_ptr().cast(), needle as c_char) };

        assert_eq!(c_result, expected as c_int, "C mismatch in case {case}");
        assert_eq!(
            rust_result, c_result,
            "Rust/C mismatch in case {case}, needle {needle:#04x}"
        );
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);

        let mut pipe_fds = [-1, -1];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut read_end = File::from_raw_fd(pipe_fds[0]);
        read_end
            .read_to_end(&mut output)
            .expect("failed to read captured stdout");
        output
    }
}

fn hex_encode(input: &[u8]) -> String {
    input.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(input: &str) -> Vec<u8> {
    assert_eq!(input.len() % 2, 0);
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|digits| {
            let digits = std::str::from_utf8(digits).expect("hex is not UTF-8");
            u8::from_str_radix(digits, 16).expect("invalid hex input")
        })
        .collect()
}

fn driver_output(library: PathBuf, input: &[u8]) -> Vec<u8> {
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    let output_path = env::temp_dir().join(format!(
        "driver-differential-{}-{}.out",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let status = Command::new(env::current_exe().expect("failed to locate test executable"))
        .args(["--exact", "ffi_driver_child", "--nocapture"])
        .env("DRIVER_CALL_LIBRARY", library)
        .env("DRIVER_CALL_INPUT", hex_encode(input))
        .env("DRIVER_CALL_OUTPUT", &output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run driver child");
    assert!(status.success(), "driver child failed with {status}");

    let output = std::fs::read(&output_path).expect("failed to read driver child output");
    std::fs::remove_file(output_path).expect("failed to remove driver child output");
    output
}

fn assert_driver_counts(a_class: CountClass, x_class: CountClass, seed: u64) {
    let mut rng = Rng::new(seed);

    for case in 0..RANDOM_CASES {
        let a_count = a_class.count(&mut rng);
        let x_count = x_class.count(&mut rng);
        let minimum_extra = if a_count == 0 && x_count == 0 {
            case % 3
        } else {
            0
        };
        let mut input =
            shuffled_input(&mut rng, &[(b'A', a_count), (b'x', x_count)], minimum_extra);
        input.push(0);

        let c_output = driver_output(c_library_path(), &input);
        let rust_output = driver_output(rust_library_path(), &input);
        let expected = format!("A: {a_count}\nx: {x_count}\n").into_bytes();

        assert_eq!(c_output, expected, "C mismatch in case {case}");
        assert_eq!(rust_output, c_output, "Rust/C mismatch in case {case}");
    }
}

#[test]
fn c01_foo_ascii_zero_matches() {
    assert_foo_class(ByteClass::Ascii, CountClass::Zero, 0xc01);
}

#[test]
fn c02_foo_ascii_one_match() {
    assert_foo_class(ByteClass::Ascii, CountClass::One, 0xc02);
}

#[test]
fn c03_foo_ascii_many_matches() {
    assert_foo_class(ByteClass::Ascii, CountClass::Many, 0xc03);
}

#[test]
fn c04_foo_high_bit_zero_matches() {
    assert_foo_class(ByteClass::HighBit, CountClass::Zero, 0xc04);
}

#[test]
fn c05_foo_high_bit_one_match() {
    assert_foo_class(ByteClass::HighBit, CountClass::One, 0xc05);
}

#[test]
fn c06_foo_high_bit_many_matches() {
    assert_foo_class(ByteClass::HighBit, CountClass::Many, 0xc06);
}

#[test]
fn c07_bytes_after_first_nul_are_ignored() {
    let apis = LoadedApis::load();
    let mut rng = Rng::new(0xc07);

    for case in 0..RANDOM_CASES {
        let prefix_a_count = rng.range(9);
        let prefix_x_count = rng.range(9);
        let mut buffer = shuffled_input(
            &mut rng,
            &[(b'A', prefix_a_count), (b'x', prefix_x_count)],
            case % 3,
        );
        buffer.push(0);
        buffer.extend_from_slice(b"AAAAxxxx");
        buffer.extend((0..rng.range(64)).map(|_| rng.nonzero_byte_except(&[])));
        buffer.push(0);

        for &(needle, expected) in &[(b'A', prefix_a_count), (b'x', prefix_x_count)] {
            let c_result = unsafe { (apis.c_foo)(buffer.as_ptr().cast(), needle as c_char) };
            let rust_result = unsafe { (apis.rust_foo)(buffer.as_ptr().cast(), needle as c_char) };
            assert_eq!(c_result, expected as c_int, "C mismatch in case {case}");
            assert_eq!(rust_result, c_result, "Rust/C mismatch in case {case}");
        }

        let c_output = driver_output(c_library_path(), &buffer);
        let rust_output = driver_output(rust_library_path(), &buffer);
        assert_eq!(rust_output, c_output, "Rust/C mismatch in case {case}");
    }
}

#[test]
fn c08_driver_zero_a_zero_x() {
    assert_driver_counts(CountClass::Zero, CountClass::Zero, 0xc08);
}

#[test]
fn c09_driver_zero_a_one_x() {
    assert_driver_counts(CountClass::Zero, CountClass::One, 0xc09);
}

#[test]
fn c10_driver_zero_a_many_x() {
    assert_driver_counts(CountClass::Zero, CountClass::Many, 0xc10);
}

#[test]
fn c11_driver_one_a_zero_x() {
    assert_driver_counts(CountClass::One, CountClass::Zero, 0xc11);
}

#[test]
fn c12_driver_one_a_one_x() {
    assert_driver_counts(CountClass::One, CountClass::One, 0xc12);
}

#[test]
fn c13_driver_one_a_many_x() {
    assert_driver_counts(CountClass::One, CountClass::Many, 0xc13);
}

#[test]
fn c14_driver_many_a_zero_x() {
    assert_driver_counts(CountClass::Many, CountClass::Zero, 0xc14);
}

#[test]
fn c15_driver_many_a_one_x() {
    assert_driver_counts(CountClass::Many, CountClass::One, 0xc15);
}

#[test]
fn c16_driver_many_a_many_x() {
    assert_driver_counts(CountClass::Many, CountClass::Many, 0xc16);
}

#[test]
fn ffi_driver_child() {
    let Some(library_path) = env::var_os("DRIVER_CALL_LIBRARY") else {
        return;
    };
    let input = hex_decode(&env::var("DRIVER_CALL_INPUT").expect("missing driver child input"));
    let output_path = env::var_os("DRIVER_CALL_OUTPUT").expect("missing driver child output path");
    let library = unsafe { Library::new(library_path) }.expect("child failed to load library");
    let function = load_symbol::<Driver>(&library, b"driver\0");
    let output = capture_stdout(|| unsafe { function(input.as_ptr().cast()) });
    std::fs::write(output_path, output).expect("failed to write driver child output");
}

#[test]
fn ffi_null_child() {
    let Some(library_path) = env::var_os("DRIVER_CHILD_LIBRARY") else {
        return;
    };
    let symbol_name = env::var("DRIVER_CHILD_SYMBOL").expect("missing child symbol");
    let library = unsafe { Library::new(library_path) }.expect("child failed to load library");

    match symbol_name.as_str() {
        "foo" => {
            let function = load_symbol::<Foo>(&library, b"foo\0");
            unsafe {
                function(std::ptr::null(), b'A' as c_char);
            }
        }
        "driver" => {
            let function = load_symbol::<Driver>(&library, b"driver\0");
            unsafe {
                function(std::ptr::null());
            }
        }
        _ => panic!("unknown child symbol {symbol_name}"),
    }
    panic!("{symbol_name}(NULL) unexpectedly returned");
}

fn null_child_status(library: PathBuf, symbol: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("failed to locate test executable"))
        .args(["--exact", "ffi_null_child", "--nocapture"])
        .env("DRIVER_CHILD_LIBRARY", library)
        .env("DRIVER_CHILD_SYMBOL", symbol)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run null-pointer child")
}

fn assert_null_parity(symbol: &str) {
    let c_status = null_child_status(c_library_path(), symbol);
    let rust_status = null_child_status(rust_library_path(), symbol);

    assert_eq!(
        c_status.signal(),
        Some(SIGSEGV),
        "C {symbol}(NULL) did not terminate with SIGSEGV: {c_status}"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "Rust/C null-pointer termination differs for {symbol}"
    );
}

#[test]
fn e01_foo_null_input() {
    assert_null_parity("foo");
}

#[test]
fn e02_driver_null_input() {
    assert_null_parity("driver");
}
