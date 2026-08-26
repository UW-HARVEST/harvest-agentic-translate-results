use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

type DriverFn = unsafe extern "C" fn(f32);
type MainFn = unsafe extern "C" fn() -> c_int;

static STDIO_LOCK: Mutex<()> = Mutex::new(());
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    static mut stdin: *mut c_void;

    fn __fpurge(stream: *mut c_void);
    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn getchar() -> c_int;
}

struct Libraries {
    _c: Library,
    _rust: Library,
    c_driver: DriverFn,
    rust_driver: DriverFn,
    c_main: MainFn,
    rust_main: MainFn,
}

impl Libraries {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver_c.so");
        let rust_path = rust_library_path(root);

        assert!(
            c_path.is_file(),
            "missing C shared object: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared object: {}",
            rust_path.display()
        );

        unsafe {
            let c = Library::new(c_path).expect("load C shared object");
            let rust = Library::new(rust_path).expect("load Rust shared object");
            let c_driver = *c.get::<DriverFn>(b"driver\0").expect("C driver export");
            let rust_driver = *rust
                .get::<DriverFn>(b"driver\0")
                .expect("Rust driver export");
            let c_main = *c.get::<MainFn>(b"main\0").expect("C main export");
            let rust_main = *rust.get::<MainFn>(b"main\0").expect("Rust main export");

            Self {
                _c: c,
                _rust: rust,
                c_driver,
                rust_driver,
                c_main,
                rust_main,
            }
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        root.join(target)
    };
    target.join("debug/deps/libdriver_ffi.so")
}

struct SavedStdio {
    stdin_fd: c_int,
    stdout_fd: c_int,
}

impl SavedStdio {
    unsafe fn redirect(input: &File, output: &File) -> Self {
        fflush(std::ptr::null_mut());
        __fpurge(stdin);

        let stdin_fd = dup(0);
        let stdout_fd = dup(1);
        assert!(stdin_fd >= 0 && stdout_fd >= 0, "dup failed");
        assert_eq!(dup2(input.as_raw_fd(), 0), 0, "redirect stdin");
        assert_eq!(dup2(output.as_raw_fd(), 1), 1, "redirect stdout");
        clearerr(stdin);

        Self {
            stdin_fd,
            stdout_fd,
        }
    }
}

impl Drop for SavedStdio {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            assert_eq!(dup2(self.stdin_fd, 0), 0, "restore stdin");
            assert_eq!(dup2(self.stdout_fd, 1), 1, "restore stdout");
            close(self.stdin_fd);
            close(self.stdout_fd);
            clearerr(stdin);
            __fpurge(stdin);
        }
    }
}

fn temporary_file(label: &str) -> File {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{sequence}-{label}",
        std::process::id()
    ));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap_or_else(|error| panic!("create {}: {error}", path.display()));
    std::fs::remove_file(path).expect("unlink temporary file");
    file
}

fn invoke_driver(function: DriverFn, bits: u32) -> Vec<u8> {
    let _lock = STDIO_LOCK.lock().expect("stdio lock");
    let mut input = temporary_file("input");
    let mut output = temporary_file("output");
    input.seek(SeekFrom::Start(0)).expect("rewind input");

    {
        let _stdio = unsafe { SavedStdio::redirect(&input, &output) };
        unsafe {
            function(f32::from_bits(bits));
        }
    }

    output.seek(SeekFrom::Start(0)).expect("rewind output");
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).expect("read output");
    bytes
}

#[derive(Debug, Eq, PartialEq)]
struct MainResult {
    return_code: c_int,
    output: Vec<u8>,
    next_input: Option<c_int>,
}

fn invoke_main(function: MainFn, input_bytes: &[u8], inspect_next: bool) -> MainResult {
    let _lock = STDIO_LOCK.lock().expect("stdio lock");
    let mut input = temporary_file("input");
    let mut output = temporary_file("output");
    input.write_all(input_bytes).expect("write input");
    input.seek(SeekFrom::Start(0)).expect("rewind input");

    let (return_code, next_input) = {
        let _stdio = unsafe { SavedStdio::redirect(&input, &output) };
        let return_code = unsafe { function() };
        let next_input = inspect_next.then(|| unsafe { getchar() });
        (return_code, next_input)
    };

    output.seek(SeekFrom::Start(0)).expect("rewind output");
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).expect("read output");
    MainResult {
        return_code,
        output: bytes,
        next_input,
    }
}

fn assert_driver_matches(libraries: &Libraries, bits: u32) {
    let c = invoke_driver(libraries.c_driver, bits);
    let rust = invoke_driver(libraries.rust_driver, bits);
    assert_eq!(rust, c, "driver mismatch for bits {bits:#010x}");
    assert_eq!(c.len(), 9, "driver output length for bits {bits:#010x}");
}

fn assert_main_matches(libraries: &Libraries, input: &[u8], inspect_next: bool) {
    let c = invoke_main(libraries.c_main, input, inspect_next);
    let rust = invoke_main(libraries.rust_main, input, inspect_next);
    assert_eq!(
        rust,
        c,
        "main mismatch for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(c.return_code, 0);
    assert_eq!(c.output.len(), 9);
}

struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn choose(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }
}

fn random_ascii_case(rng: &mut XorShift64, word: &[u8]) -> Vec<u8> {
    word.iter()
        .map(|byte| {
            if rng.next_u64() & 1 == 0 {
                byte.to_ascii_lowercase()
            } else {
                byte.to_ascii_uppercase()
            }
        })
        .collect()
}

#[test]
fn config_1_driver_random_raw_float_bits() {
    let libraries = Libraries::load();
    let boundaries = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x7f7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0x7f80_0001,
        0xffc0_0001,
    ];
    for bits in boundaries {
        assert_driver_matches(&libraries, bits);
    }

    let mut rng = XorShift64(0xd1ff_e4e5_7a11_cafe);
    for _ in 0..2_048 {
        assert_driver_matches(&libraries, rng.next_u32());
    }
}

#[test]
fn config_2_main_random_decimal_floats() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0xdec1_a100_5eed_0002);
    let mut completed = 0;
    while completed < 512 {
        let value = f32::from_bits(rng.next_u32());
        if value.is_finite() {
            let forms = [
                format!("{value:.9e}\n"),
                format!("{value:.8}\n"),
                format!("{value:+.9e}\n"),
            ];
            assert_main_matches(&libraries, forms[rng.choose(forms.len())].as_bytes(), false);
            completed += 1;
        }
    }
}

#[test]
fn config_3_main_random_hexadecimal_floats() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0x0f10_a700_5eed_0003);
    for _ in 0..512 {
        let sign = if rng.next_u64() & 1 == 0 { "" } else { "-" };
        let integer = 1 + rng.choose(15);
        let fraction = rng.next_u32() & 0x00ff_ffff;
        let exponent = rng.choose(201) as i32 - 100;
        let input = format!("{sign}0x{integer:x}.{fraction:06x}p{exponent:+}\n");
        assert_main_matches(&libraries, input.as_bytes(), false);
    }
}

#[test]
fn config_4_main_infinity_case_variants() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0x1af1_0170_5eed_0004);
    for _ in 0..128 {
        let base = if rng.next_u64() & 1 == 0 {
            b"inf".as_slice()
        } else {
            b"infinity".as_slice()
        };
        let mut input = Vec::new();
        if rng.next_u64() & 1 != 0 {
            input.push(if rng.next_u64() & 1 == 0 { b'+' } else { b'-' });
        }
        input.extend(random_ascii_case(&mut rng, base));
        input.push(b'\n');
        assert_main_matches(&libraries, &input, false);
    }
}

#[test]
fn config_5_main_nan_case_variants() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0x0a0a_0a00_5eed_0005);
    for _ in 0..128 {
        let mut input = Vec::new();
        if rng.next_u64() & 1 != 0 {
            input.push(if rng.next_u64() & 1 == 0 { b'+' } else { b'-' });
        }
        input.extend(random_ascii_case(&mut rng, b"nan"));
        input.push(b'\n');
        assert_main_matches(&libraries, &input, false);
    }
}

#[test]
fn config_6_main_whitespace_and_trailing_input() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0x7a11_1a90_5eed_0006);
    let whitespace = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];
    let trailing = [b'!', b'?', b'X', b'_', b'/'];

    for _ in 0..256 {
        let mut input = Vec::new();
        for _ in 0..(1 + rng.choose(12)) {
            input.push(whitespace[rng.choose(whitespace.len())]);
        }
        let value = (rng.next_u32() as i32) as f32 / 65_536.0;
        input.extend(format!("{value:.7e}").as_bytes());
        let marker = trailing[rng.choose(trailing.len())];
        input.push(marker);
        input.extend(b"unread");
        assert_main_matches(&libraries, &input, true);

        let c = invoke_main(libraries.c_main, &input, true);
        assert_eq!(c.next_input, Some(c_int::from(marker)));
    }
}

#[test]
fn error_1_main_matching_failure() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0xe220_0000_5eed_0001);
    let invalid = [b'?', b'_', b'/', b'@', b'z'];
    let whitespace = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];

    for _ in 0..128 {
        let mut input = Vec::new();
        for _ in 0..rng.choose(12) {
            input.push(whitespace[rng.choose(whitespace.len())]);
        }
        let marker = invalid[rng.choose(invalid.len())];
        input.push(marker);
        input.extend(b"not-a-float");
        assert_main_matches(&libraries, &input, true);

        let c = invoke_main(libraries.c_main, &input, true);
        assert_eq!(c.output, b"00000000\n");
        assert_eq!(c.next_input, Some(c_int::from(marker)));
    }
}

#[test]
fn error_2_main_eof_before_conversion() {
    let libraries = Libraries::load();
    let mut rng = XorShift64(0xe0f0_0000_5eed_0002);
    let whitespace = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];

    for _ in 0..128 {
        let mut input = Vec::new();
        for _ in 0..rng.choose(32) {
            input.push(whitespace[rng.choose(whitespace.len())]);
        }
        assert_main_matches(&libraries, &input, true);

        let c = invoke_main(libraries.c_main, &input, true);
        assert_eq!(c.output, b"00000000\n");
        assert_eq!(c.next_input, Some(-1));
    }
}
