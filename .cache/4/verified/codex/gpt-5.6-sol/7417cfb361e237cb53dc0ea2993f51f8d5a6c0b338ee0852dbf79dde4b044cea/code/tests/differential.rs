use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

type StaticAlias = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
type CMain = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

static COPY_ID: AtomicU64 = AtomicU64::new(0);
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Pair {
    c: Library,
    rust: Library,
}

#[derive(Clone, Copy)]
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

    fn range(&mut self, low: i32, high: i32) -> i32 {
        assert!(low <= high);
        low + (self.next_u32() % (high - low + 1) as u32) as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libstatic_alias_c.so")
}

fn rust_library_path() -> PathBuf {
    std::env::current_exe()
        .expect("current test executable")
        .parent()
        .expect("test deps directory")
        .join("libstatic_alias.so")
}

fn copy_and_load(source: &Path, label: &str, id: u64) -> Library {
    let directory = std::env::temp_dir().join(format!(
        "static_alias_ffi_{}_{}_{label}",
        std::process::id(),
        id
    ));
    fs::create_dir_all(&directory).expect("create temporary library directory");
    let destination = directory.join(format!("lib{label}.so"));
    fs::copy(source, &destination).unwrap_or_else(|error| {
        panic!(
            "copy shared library {} to {}: {error}",
            source.display(),
            destination.display()
        )
    });
    let library = unsafe { Library::new(&destination) }
        .unwrap_or_else(|error| panic!("load {}: {error}", destination.display()));
    fs::remove_file(&destination).expect("unlink loaded shared library");
    fs::remove_dir(&directory).expect("remove temporary library directory");
    library
}

fn fresh_pair() -> Pair {
    let id = COPY_ID.fetch_add(1, Ordering::Relaxed);
    Pair {
        c: copy_and_load(&c_library_path(), "c", id),
        rust: copy_and_load(&rust_library_path(), "rust", id),
    }
}

unsafe fn static_call(library: &Library, value: *mut c_int) -> *mut c_int {
    let function: Symbol<StaticAlias> = library.get(b"static_alias\0").expect("static_alias");
    function(value)
}

fn compare_static_once(pair: &Pair, c_value: &mut i32, rust_value: &mut i32) -> (bool, i32) {
    let c_input = ptr::addr_of_mut!(*c_value);
    let rust_input = ptr::addr_of_mut!(*rust_value);
    let c_result = unsafe { static_call(&pair.c, c_input) };
    let rust_result = unsafe { static_call(&pair.rust, rust_input) };

    assert_eq!(unsafe { c_result.read().to_ne_bytes() }, unsafe {
        rust_result.read().to_ne_bytes()
    });
    assert_eq!(c_result == c_input, rust_result == rust_input);
    assert_eq!(c_value.to_ne_bytes(), rust_value.to_ne_bytes());
    (c_result == c_input, unsafe { c_result.read() })
}

fn capture_stdout(call: impl FnOnce() -> i32) -> (i32, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut pipe_fds = [0; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        let result = call();
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0);
        (result, output)
    }
}

fn invoke_main(library: &Library, argc: i32, argv: Option<&mut [*mut c_char]>) -> (i32, Vec<u8>) {
    let function: Symbol<CMain> = unsafe { library.get(b"main\0").expect("main") };
    let argv = argv.map_or(ptr::null_mut(), |items| items.as_mut_ptr());
    capture_stdout(|| unsafe { function(argc, argv) })
}

fn compare_main(argc: i32, arguments: &[Option<String>]) -> (i32, Vec<u8>) {
    let _stdout = STDOUT_LOCK.lock().expect("stdout lock");
    let pair = fresh_pair();
    let storage: Vec<Option<CString>> = arguments
        .iter()
        .map(|argument| {
            argument
                .as_ref()
                .map(|value| CString::new(value.as_bytes()).expect("argument without NUL"))
        })
        .collect();
    let mut pointers: Vec<*mut c_char> = storage
        .iter()
        .map(|argument| {
            argument
                .as_ref()
                .map_or(ptr::null_mut(), |value| value.as_ptr().cast_mut())
        })
        .collect();

    let c = invoke_main(&pair.c, argc, Some(&mut pointers));
    let rust = invoke_main(&pair.rust, argc, Some(&mut pointers));
    assert_eq!(c, rust);
    c
}

fn compare_main_null_argv(argc: i32) -> (i32, Vec<u8>) {
    let _stdout = STDOUT_LOCK.lock().expect("stdout lock");
    let pair = fresh_pair();
    let c = invoke_main(&pair.c, argc, None);
    let rust = invoke_main(&pair.rust, argc, None);
    assert_eq!(c, rust);
    c
}

fn args(first: impl Into<String>, second: impl Into<String>) -> Vec<Option<String>> {
    vec![
        Some("driver".to_owned()),
        Some(first.into()),
        Some(second.into()),
    ]
}

fn signed_text(value: i32, rng: &mut Rng) -> String {
    match rng.next_u32() % 4 {
        0 => value.to_string(),
        1 if value >= 0 => format!("+{value}"),
        1 => value.to_string(),
        2 => format!("  {value}"),
        _ if value >= 0 => format!("{value:03}"),
        _ => value.to_string(),
    }
}

#[test]
fn config_01_external_less_than_inner() {
    let mut rng = Rng::new(0x0101_5eed);
    for _ in 0..64 {
        let pair = fresh_pair();
        let mut prep_c = rng.range(2, 10_000);
        let mut prep_rust = prep_c;
        let (_, inner) = compare_static_once(&pair, &mut prep_c, &mut prep_rust);
        let mut c_value = rng.range(-10_000, inner - 1);
        let mut rust_value = c_value;
        let (returned_input, _) = compare_static_once(&pair, &mut c_value, &mut rust_value);
        assert!(returned_input);
    }
}

#[test]
fn config_02_external_equal_to_inner() {
    let mut rng = Rng::new(0x0202_5eed);
    for _ in 0..64 {
        let pair = fresh_pair();
        let mut prep_c = rng.range(1, 10_000);
        let mut prep_rust = prep_c;
        let (_, inner) = compare_static_once(&pair, &mut prep_c, &mut prep_rust);
        let mut c_value = inner;
        let mut rust_value = inner;
        let (returned_input, _) = compare_static_once(&pair, &mut c_value, &mut rust_value);
        assert!(!returned_input);
    }
}

#[test]
fn config_03_external_greater_than_inner() {
    let mut rng = Rng::new(0x0303_5eed);
    for _ in 0..64 {
        let pair = fresh_pair();
        let mut prep_c = rng.range(1, 1_000);
        let mut prep_rust = prep_c;
        let (_, inner) = compare_static_once(&pair, &mut prep_c, &mut prep_rust);
        let mut c_value = inner + rng.range(1, 10_000);
        let mut rust_value = c_value;
        let (returned_input, _) = compare_static_once(&pair, &mut c_value, &mut rust_value);
        assert!(!returned_input);
    }
}

#[test]
fn config_04_static_alias_input() {
    let mut rng = Rng::new(0x0404_5eed);
    for _ in 0..64 {
        let pair = fresh_pair();
        let mut c_value = rng.range(1, 10_000);
        let mut rust_value = c_value;
        let c_inner = unsafe { static_call(&pair.c, &mut c_value) };
        let rust_inner = unsafe { static_call(&pair.rust, &mut rust_value) };
        let c_result = unsafe { static_call(&pair.c, c_inner) };
        let rust_result = unsafe { static_call(&pair.rust, rust_inner) };
        assert_eq!(c_result, c_inner);
        assert_eq!(rust_result, rust_inner);
        assert_eq!(unsafe { c_result.read().to_ne_bytes() }, unsafe {
            rust_result.read().to_ne_bytes()
        });
    }
}

#[test]
fn config_05_random_returned_pointer_sequences() {
    let mut rng = Rng::new(0x0505_5eed);
    for _ in 0..128 {
        let pair = fresh_pair();
        let mut c_value = rng.range(-20, 20);
        let mut rust_value = c_value;
        let mut c_current = ptr::addr_of_mut!(c_value);
        let mut rust_current = ptr::addr_of_mut!(rust_value);
        for _ in 0..rng.range(2, 12) {
            let c_input = c_current;
            let rust_input = rust_current;
            c_current = unsafe { static_call(&pair.c, c_current) };
            rust_current = unsafe { static_call(&pair.rust, rust_current) };
            assert_eq!(c_current == c_input, rust_current == rust_input);
            assert_eq!(unsafe { c_current.read().to_ne_bytes() }, unsafe {
                rust_current.read().to_ne_bytes()
            });
        }
    }
}

#[test]
fn config_06_negative_iterations() {
    let mut rng = Rng::new(0x0606_5eed);
    for _ in 0..64 {
        let initial = rng.range(-100_000, 100_000);
        let iterations = rng.range(-1_000, -1);
        assert_eq!(
            compare_main(3, &args(initial.to_string(), iterations.to_string())),
            (0, vec![])
        );
    }
}

#[test]
fn config_07_zero_iterations() {
    let mut rng = Rng::new(0x0707_5eed);
    for _ in 0..64 {
        let initial = rng.range(-1_000_000, 1_000_000);
        assert_eq!(
            compare_main(3, &args(initial.to_string(), "0")),
            (0, vec![])
        );
    }
}

#[test]
fn config_08_one_iteration_below_inner() {
    let mut rng = Rng::new(0x0808_5eed);
    for _ in 0..64 {
        let initial = rng.range(-100_000, 0);
        compare_main(3, &args(initial.to_string(), "1"));
    }
}

#[test]
fn config_09_one_iteration_equal_to_inner() {
    let mut rng = Rng::new(0x0909_5eed);
    for _ in 0..64 {
        let first = signed_text(1, &mut rng);
        let second = signed_text(1, &mut rng);
        compare_main(3, &args(first, second));
    }
}

#[test]
fn config_10_one_iteration_above_inner() {
    let mut rng = Rng::new(0x1010_5eed);
    for _ in 0..64 {
        compare_main(3, &args(rng.range(2, 1_000_000).to_string(), "1"));
    }
}

#[test]
fn config_11_many_iterations_before_transition() {
    let mut rng = Rng::new(0x1111_5eed);
    for _ in 0..64 {
        let initial = rng.range(-30, -2);
        let iterations = rng.range(2, 1 - initial);
        compare_main(3, &args(initial.to_string(), iterations.to_string()));
    }
}

#[test]
fn config_12_transition_on_final_iteration() {
    let mut rng = Rng::new(0x1212_5eed);
    for _ in 0..64 {
        let initial = rng.range(-30, 0);
        let iterations = 2 - initial;
        compare_main(3, &args(initial.to_string(), iterations.to_string()));
    }
}

#[test]
fn config_13_transition_before_final_iteration() {
    let mut rng = Rng::new(0x1313_5eed);
    for _ in 0..64 {
        let initial = rng.range(-20, 0);
        let iterations = 2 - initial + rng.range(1, 5);
        compare_main(3, &args(initial.to_string(), iterations.to_string()));
    }
}

#[test]
fn config_14_many_iterations_equal_to_inner() {
    let mut rng = Rng::new(0x1414_5eed);
    for _ in 0..64 {
        compare_main(3, &args("1", rng.range(2, 15).to_string()));
    }
}

#[test]
fn config_15_many_iterations_above_inner() {
    let mut rng = Rng::new(0x1515_5eed);
    for _ in 0..64 {
        let initial = rng.range(2, 1_000);
        let iterations = rng.range(2, 15);
        compare_main(3, &args(initial.to_string(), iterations.to_string()));
    }
}

#[test]
fn config_16_leading_whitespace_and_signs() {
    let mut rng = Rng::new(0x1616_5eed);
    for _ in 0..64 {
        let initial = rng.range(-1_000, 1_000);
        let iterations = rng.range(0, 8);
        compare_main(
            3,
            &args(format!(" \t{initial:+}"), format!("\n {iterations:+}")),
        );
    }
}

#[test]
fn config_17_numeric_prefix_with_trailing_bytes() {
    let mut rng = Rng::new(0x1717_5eed);
    for _ in 0..64 {
        let initial = rng.range(-1_000, 1_000);
        let iterations = rng.range(0, 8);
        compare_main(
            3,
            &args(format!("{initial}tail"), format!("{iterations}!ignored")),
        );
    }
}

#[test]
fn config_18_strtol_overflow_is_accepted() {
    let mut rng = Rng::new(0x1818_5eed);
    for _ in 0..64 {
        let digits = "9".repeat(rng.range(20, 80) as usize);
        let first = if rng.next_u32() & 1 == 0 {
            digits
        } else {
            format!("-{digits}")
        };
        compare_main(3, &args(first, rng.range(0, 5).to_string()));
    }
}

#[test]
fn config_19_null_unused_argv_zero() {
    let mut rng = Rng::new(0x1919_5eed);
    for _ in 0..64 {
        let mut arguments = args(
            rng.range(-100, 100).to_string(),
            rng.range(0, 8).to_string(),
        );
        arguments[0] = None;
        compare_main(3, &arguments);
    }
}

#[test]
fn error_01_wrong_argument_count() {
    let expected = b"Error: should only be two (integer) arguments!\n".to_vec();
    for argc in [-100, -1, 0, 1, 2, 4, 5, 100] {
        assert_eq!(compare_main_null_argv(argc), (1, expected.clone()));
    }
}

#[test]
fn error_02_first_argument_has_no_conversion() {
    for invalid in ["", "abc", "   ", "+", "-", "\t xyz", "++1", "--2"] {
        assert_eq!(
            compare_main(3, &args(invalid, "1")),
            (1, b"Error: first argument must be an integer!\n".to_vec())
        );
    }
}

#[test]
fn error_03_second_argument_has_no_conversion() {
    for invalid in ["", "abc", "   ", "+", "-", "\n xyz", "+-1", "-+2"] {
        assert_eq!(
            compare_main(3, &args("1", invalid)),
            (1, b"Error: second argument must be an integer!\n".to_vec())
        );
    }
}

fn child_library() -> Library {
    let implementation = std::env::var("STATIC_ALIAS_CHILD_IMPL").expect("child implementation");
    let path = match implementation.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown child implementation {implementation}"),
    };
    unsafe { Library::new(&path) }
        .unwrap_or_else(|error| panic!("load {}: {error}", path.display()))
}

#[test]
fn null_boundary_child() {
    let Ok(boundary) = std::env::var("STATIC_ALIAS_CHILD_BOUNDARY") else {
        return;
    };
    let library = child_library();
    unsafe {
        match boundary.as_str() {
            "static_alias" => {
                let function: Symbol<StaticAlias> =
                    library.get(b"static_alias\0").expect("static_alias");
                function(ptr::null_mut());
            }
            "main_argv" => {
                let function: Symbol<CMain> = library.get(b"main\0").expect("main");
                function(3, ptr::null_mut());
            }
            "main_first" => {
                let function: Symbol<CMain> = library.get(b"main\0").expect("main");
                let program = CString::new("driver").unwrap();
                let second = CString::new("1").unwrap();
                let mut argv = [
                    program.as_ptr().cast_mut(),
                    ptr::null_mut(),
                    second.as_ptr().cast_mut(),
                ];
                function(3, argv.as_mut_ptr());
            }
            "main_second" => {
                let function: Symbol<CMain> = library.get(b"main\0").expect("main");
                let program = CString::new("driver").unwrap();
                let first = CString::new("1").unwrap();
                let mut argv = [
                    program.as_ptr().cast_mut(),
                    first.as_ptr().cast_mut(),
                    ptr::null_mut(),
                ];
                function(3, argv.as_mut_ptr());
            }
            _ => panic!("unknown null boundary {boundary}"),
        }
    }
    panic!("null boundary unexpectedly returned");
}

fn assert_sigsegv_parity(boundary: &str) {
    for implementation in ["c", "rust"] {
        let status = Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", "null_boundary_child", "--nocapture"])
            .env("STATIC_ALIAS_CHILD_BOUNDARY", boundary)
            .env("STATIC_ALIAS_CHILD_IMPL", implementation)
            .status()
            .expect("run null-boundary child");
        assert_eq!(
            status.signal(),
            Some(11),
            "{implementation} {boundary} status was {status}"
        );
    }
}

#[test]
fn error_04_static_alias_null() {
    assert_sigsegv_parity("static_alias");
}

#[test]
fn error_05_main_argv_null() {
    assert_sigsegv_parity("main_argv");
}

#[test]
fn error_06_main_first_argument_null() {
    assert_sigsegv_parity("main_first");
}

#[test]
fn error_07_main_second_argument_null() {
    assert_sigsegv_parity("main_second");
}
