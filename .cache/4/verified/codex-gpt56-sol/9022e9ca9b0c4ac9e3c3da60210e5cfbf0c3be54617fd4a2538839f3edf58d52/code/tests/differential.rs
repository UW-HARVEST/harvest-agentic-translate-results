use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type CallFma = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
type Main = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library();
        let rust_path = rust_library();
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
            Self {
                c: Library::new(c_path).expect("load C shared object"),
                rust: Library::new(rust_path).expect("load Rust shared object"),
            }
        }
    }

    unsafe fn fma_array(&self) -> (Symbol<'_, FmaArray>, Symbol<'_, FmaArray>) {
        (
            self.c.get(b"fma_array").expect("C fma_array"),
            self.rust.get(b"fma_array").expect("Rust fma_array"),
        )
    }

    unsafe fn call_fma(&self) -> (Symbol<'_, CallFma>, Symbol<'_, CallFma>) {
        (
            self.c.get(b"call_fma").expect("C call_fma"),
            self.rust.get(b"call_fma").expect("Rust call_fma"),
        )
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        min + (self.next_u32() % ((max - min + 1) as u32)) as i32
    }

    fn range_usize(&mut self, min: usize, max: usize) -> usize {
        min + self.next_u32() as usize % (max - min + 1)
    }
}

fn random_vec(rng: &mut Rng, len: usize, min: i32, max: i32) -> Vec<i32> {
    (0..len).map(|_| rng.range_i32(min, max)).collect()
}

fn compare_fma(mul1: &[i32], mul2: &[i32], add: &[i32]) {
    let libraries = Libraries::load();
    let mut c_out = vec![0x1357_2468; mul1.len()];
    let mut rust_out = c_out.clone();
    unsafe {
        let (c_fma, rust_fma) = libraries.fma_array();
        c_fma(
            c_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            mul1.len() as c_int,
        );
        rust_fma(
            rust_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            mul1.len() as c_int,
        );
    }
    assert_eq!(rust_out, c_out);
}

fn compare_call(data: &[i32]) -> i32 {
    let libraries = Libraries::load();
    unsafe {
        let (c_call, rust_call) = libraries.call_fma();
        let c_result = c_call(data.as_ptr(), data.len() as c_int);
        let rust_result = rust_call(data.as_ptr(), data.len() as c_int);
        assert_eq!(rust_result, c_result);
        c_result
    }
}

static OUTPUT_ID: AtomicU64 = AtomicU64::new(0);

fn helper_output_path() -> PathBuf {
    let id = OUTPUT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-differential-{}-{id}.out",
        std::process::id()
    ))
}

fn run_main(library: &Path, input: &[u8]) -> Vec<u8> {
    let output_path = helper_output_path();
    let mut child = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_subprocess")
        .arg("--nocapture")
        .env("DRIVER_FFI_HELPER_MODE", "main")
        .env("DRIVER_FFI_LIBRARY", library)
        .env("DRIVER_FFI_OUTPUT", &output_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn FFI main helper");
    child
        .stdin
        .take()
        .expect("helper stdin")
        .write_all(input)
        .expect("write helper stdin");
    let output = child.wait_with_output().expect("wait for FFI main helper");
    assert!(
        output.status.success(),
        "FFI main helper failed: status={:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&output_path).expect("read helper output");
    fs::remove_file(output_path).expect("remove helper output");
    bytes
}

fn compare_main(input: impl AsRef<[u8]>) -> Vec<u8> {
    let input = input.as_ref();
    let c_output = run_main(&c_library(), input);
    let rust_output = run_main(&rust_library(), input);
    assert_eq!(
        rust_output,
        c_output,
        "main output differs for input {:?}",
        String::from_utf8_lossy(input)
    );
    c_output
}

fn run_crash(library: &Path, mode: &str) -> Option<i32> {
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_subprocess")
        .arg("--nocapture")
        .env("DRIVER_FFI_HELPER_MODE", mode)
        .env("DRIVER_FFI_LIBRARY", library)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run crash helper");
    output.signal()
}

fn token(value: i32, style: u32) -> String {
    match style % 4 {
        0 => value.to_string(),
        1 if value >= 0 => format!("+{value}"),
        1 => value.to_string(),
        2 => format!(" \t{value}\r\n"),
        _ if value >= 0 => format!("000{value}"),
        _ => format!("-000{}", value.unsigned_abs()),
    }
}

#[test]
fn ffi_subprocess() {
    let Ok(mode) = std::env::var("DRIVER_FFI_HELPER_MODE") else {
        return;
    };
    let library_path = std::env::var_os("DRIVER_FFI_LIBRARY").expect("helper library path");
    let library = unsafe { Library::new(library_path).expect("load helper shared object") };

    unsafe {
        match mode.as_str() {
            "main" => {
                let output_path =
                    std::env::var_os("DRIVER_FFI_OUTPUT").expect("helper output path");
                let output = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(output_path)
                    .expect("open helper output");
                assert_eq!(fflush(std::ptr::null_mut()), 0);
                let saved_stdout = dup(1);
                assert!(saved_stdout >= 0);
                assert_eq!(dup2(output.as_raw_fd(), 1), 1);
                let ffi_main: Symbol<'_, Main> = library.get(b"main").expect("main symbol");
                assert_eq!(ffi_main(), 0);
                assert_eq!(fflush(std::ptr::null_mut()), 0);
                assert_eq!(dup2(saved_stdout, 1), 1);
                assert_eq!(close(saved_stdout), 0);
            }
            "fma_null" => {
                let fma: Symbol<'_, FmaArray> =
                    library.get(b"fma_array").expect("fma_array symbol");
                fma(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1,
                );
            }
            "call_null" => {
                let call: Symbol<'_, CallFma> = library.get(b"call_fma").expect("call_fma symbol");
                call(std::ptr::null(), 1);
            }
            other => panic!("unknown helper mode {other}"),
        }
    }
}

#[test]
fn config_fma_nonpositive_lengths() {
    let libraries = Libraries::load();
    unsafe {
        let (c_fma, rust_fma) = libraries.fma_array();
        for len in [-10_000, -257, -2, -1, 0] {
            c_fma(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                len,
            );
            rust_fma(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                len,
            );
        }
    }
}

#[test]
fn config_fma_single() {
    let mut rng = Rng::new(0x7d72_8a4b_a53c_91e1);
    for _ in 0..512 {
        compare_fma(
            &[rng.range_i32(-1_000, 1_000)],
            &[rng.range_i32(-1_000, 1_000)],
            &[rng.range_i32(-1_000, 1_000)],
        );
    }
}

#[test]
fn config_fma_many() {
    let mut rng = Rng::new(0x9684_7e11_f2d6_c035);
    for _ in 0..256 {
        let len = rng.range_usize(2, 257);
        let mul1 = random_vec(&mut rng, len, -1_000, 1_000);
        let mul2 = random_vec(&mut rng, len, -1_000, 1_000);
        let add = random_vec(&mut rng, len, -1_000, 1_000);
        compare_fma(&mul1, &mul2, &add);
    }
}

#[test]
fn config_call_zero() {
    let libraries = Libraries::load();
    unsafe {
        let (c_call, rust_call) = libraries.call_fma();
        for _ in 0..128 {
            assert_eq!(c_call(std::ptr::null(), 0), 0);
            assert_eq!(rust_call(std::ptr::null(), 0), 0);
        }
    }
}

#[test]
fn config_call_single() {
    let mut rng = Rng::new(0x52f3_a799_d184_c620);
    for value in [i32::MIN, i32::MAX, -1, 0, 1] {
        assert_eq!(compare_call(&[value]), value);
    }
    for _ in 0..512 {
        let value = rng.next_u32() as i32;
        assert_eq!(compare_call(&[value]), value);
    }
}

#[test]
fn config_call_many_and_large() {
    let mut rng = Rng::new(0xbf02_845d_a3e6_7c19);
    for _ in 0..256 {
        let len = rng.range_usize(2, 512);
        let mut data: Vec<i32> = (0..len).map(|_| rng.next_u32() as i32).collect();
        data[0] = i32::MIN;
        data[len - 1] = if len % 2 == 0 { i32::MAX } else { i32::MIN };
        assert_eq!(compare_call(&data), data[len - 1]);
    }
    let data: Vec<i32> = (0..65_536).map(|index| index as i32).collect();
    assert_eq!(compare_call(&data), 65_535);
}

#[test]
fn config_main_single() {
    let mut rng = Rng::new(0x5c4a_9128_763e_b0df);
    for index in 0..64 {
        let value = rng.range_i32(-1_000_000, 1_000_000);
        let output = compare_main(token(value, index));
        assert_eq!(output, format!("{value}\n").as_bytes());
    }
}

#[test]
fn config_main_many() {
    let mut rng = Rng::new(0xd1e8_734c_2b90_f65a);
    for case in 0..32 {
        let len = rng.range_usize(2, 99);
        let values: Vec<i32> = (0..len)
            .map(|_| rng.range_i32(-1_000_000, 1_000_000))
            .collect();
        let input: String = values
            .iter()
            .enumerate()
            .map(|(index, value)| token(*value, index as u32 + case))
            .collect::<Vec<_>>()
            .join(" \n");
        assert_eq!(
            compare_main(input),
            format!("{}\n", values[len - 1]).as_bytes()
        );
    }
}

#[test]
fn config_main_exactly_100() {
    let mut rng = Rng::new(0x3a95_e724_68dc_b10f);
    for case in 0..16 {
        let values: Vec<i32> = (0..100).map(|_| rng.next_u32() as i32).collect();
        let input: String = values
            .iter()
            .enumerate()
            .map(|(index, value)| token(*value, index as u32 + case))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(compare_main(input), format!("{}\n", values[99]).as_bytes());
    }
}

#[test]
fn config_main_over_100() {
    let mut rng = Rng::new(0x184f_c2a6_9d73_50be);
    for case in 0..16 {
        let len = rng.range_usize(101, 180);
        let values: Vec<i32> = (0..len).map(|_| rng.next_u32() as i32).collect();
        let input: String = values
            .iter()
            .enumerate()
            .map(|(index, value)| token(*value, index as u32 + case))
            .collect::<Vec<_>>()
            .join("\t");
        assert_eq!(compare_main(input), format!("{}\n", values[99]).as_bytes());
    }
}

#[test]
fn config_main_integer_boundaries() {
    let inputs = [
        "-2147483648",
        "2147483647",
        "+2147483647",
        "2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "-9223372036854775808",
        "18446744073709551615",
        "-18446744073709551616",
    ];
    for input in inputs {
        compare_main(input);
    }
}

#[test]
fn error_scan_eof() {
    let mut rng = Rng::new(0xa7cb_501d_e923_684f);
    assert_eq!(compare_main([]), b"0\n");
    for len in 1..=32 {
        let values: Vec<i32> = (0..len)
            .map(|_| rng.range_i32(-1_000_000, 1_000_000))
            .collect();
        let input = values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            compare_main(input),
            format!("{}\n", values[len - 1]).as_bytes()
        );
    }
}

#[test]
fn error_scan_matching_failure() {
    let mut rng = Rng::new(0x649b_f831_2ca7_d05e);
    for len in 0..32 {
        let values: Vec<i32> = (0..len)
            .map(|_| rng.range_i32(-1_000_000, 1_000_000))
            .collect();
        let prefix = values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        let input = format!("{prefix} {} trailing", ["x", "--1", "+", ".10"][len % 4]);
        let expected = values.last().copied().unwrap_or(0);
        assert_eq!(compare_main(input), format!("{expected}\n").as_bytes());
    }
}

#[test]
fn generic_positive_length_null_pointers() {
    for mode in ["fma_null", "call_null"] {
        let c_signal = run_crash(&c_library(), mode);
        let rust_signal = run_crash(&rust_library(), mode);
        assert!(c_signal.is_some(), "C {mode} unexpectedly returned");
        assert_eq!(rust_signal, c_signal, "{mode} termination differs");
    }
}
