use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const ITERATIONS: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RegMatch {
    rm_so: c_int,
    rm_eo: c_int,
}

#[repr(C)]
struct OsData {
    os_name: *mut c_char,
    os_version: *mut c_char,
    os_major: *mut c_char,
    os_minor: *mut c_char,
    os_codename: *mut c_char,
    os_platform: *mut c_char,
    os_build: *mut c_char,
    os_uname: *mut c_char,
    os_arch: *mut c_char,
}

type GetOsArch = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
type WRegexec = unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut RegMatch) -> c_int;
type ParseUname = unsafe extern "C" fn(*mut c_char, *mut OsData);

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn number(&mut self, max: u64) -> u64 {
        self.next() % max
    }

    fn safe_word(&mut self) -> String {
        const LETTERS: &[u8] = b"BCDEFGHJKLMNOPQRSTUVWXYZ";
        let len = 1 + self.number(18) as usize;
        (0..len)
            .map(|_| LETTERS[self.number(LETTERS.len() as u64) as usize] as char)
            .collect()
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library was not built")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libdriver.so")
        .canonicalize()
        .expect("Rust release shared library was not built")
}

fn libraries() -> (Library, Library) {
    unsafe {
        (
            Library::new(c_library_path()).expect("load C library"),
            Library::new(rust_library_path()).expect("load Rust library"),
        )
    }
}

unsafe fn returned_bytes(pointer: *mut c_char) -> Option<Vec<u8>> {
    if pointer.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(pointer) }.to_bytes().to_vec())
    }
}

unsafe fn compare_arch(c: &Library, rust: &Library, input: &str) {
    let c_input = CString::new(input).unwrap();
    let rust_input = CString::new(input).unwrap();
    let c_function = unsafe { c.get::<GetOsArch>(b"get_os_arch\0").unwrap() };
    let rust_function = unsafe { rust.get::<GetOsArch>(b"get_os_arch\0").unwrap() };
    let c_pointer = unsafe { c_function(c_input.as_ptr().cast_mut()) };
    let rust_pointer = unsafe { rust_function(rust_input.as_ptr().cast_mut()) };
    assert_eq!(
        unsafe { returned_bytes(c_pointer) },
        unsafe { returned_bytes(rust_pointer) },
        "get_os_arch input {input:?}"
    );
    if !c_pointer.is_null() {
        unsafe { free(c_pointer.cast()) };
    }
    if !rust_pointer.is_null() {
        unsafe { free(rust_pointer.cast()) };
    }
}

unsafe fn regex_call(
    library: &Library,
    pattern: Option<&CStr>,
    string: Option<&CStr>,
    nmatch: usize,
) -> (c_int, Vec<RegMatch>) {
    let function = unsafe { library.get::<WRegexec>(b"w_regexec\0").unwrap() };
    let mut matches = vec![
        RegMatch {
            rm_so: -777,
            rm_eo: -888,
        };
        nmatch
    ];
    let match_pointer = if nmatch == 0 {
        std::ptr::null_mut()
    } else {
        matches.as_mut_ptr()
    };
    let result = unsafe {
        function(
            pattern.map_or(std::ptr::null(), CStr::as_ptr),
            string.map_or(std::ptr::null(), CStr::as_ptr),
            nmatch,
            match_pointer,
        )
    };
    (result, matches)
}

unsafe fn compare_regex(pattern: &str, string: &str, nmatch: usize) {
    let (c, rust) = libraries();
    let pattern = CString::new(pattern).unwrap();
    let string = CString::new(string).unwrap();
    let c_result = unsafe {
        regex_call(
            &c,
            Some(pattern.as_c_str()),
            Some(string.as_c_str()),
            nmatch,
        )
    };
    let rust_result = unsafe {
        regex_call(
            &rust,
            Some(pattern.as_c_str()),
            Some(string.as_c_str()),
            nmatch,
        )
    };
    assert_eq!(
        c_result, rust_result,
        "pattern={pattern:?}, string={string:?}"
    );
}

unsafe fn os_snapshot(data: &OsData) -> [Option<Vec<u8>>; 9] {
    let pointers = [
        data.os_name,
        data.os_version,
        data.os_major,
        data.os_minor,
        data.os_codename,
        data.os_platform,
        data.os_build,
        data.os_uname,
        data.os_arch,
    ];
    std::array::from_fn(|index| unsafe { returned_bytes(pointers[index]) })
}

unsafe fn free_os_data(data: &mut OsData) {
    let pointers = [
        data.os_name,
        data.os_version,
        data.os_major,
        data.os_minor,
        data.os_codename,
        data.os_platform,
        data.os_build,
        data.os_uname,
        data.os_arch,
    ];
    for pointer in pointers {
        if !pointer.is_null() {
            unsafe { free(pointer.cast()) };
        }
    }
}

unsafe fn parse_call(library: &Library, input: &str) -> ([Option<Vec<u8>>; 9], Vec<u8>) {
    let function = unsafe { library.get::<ParseUname>(b"parse_uname_string\0").unwrap() };
    let mut bytes = CString::new(input).unwrap().into_bytes_with_nul();
    let mut data: OsData = unsafe { std::mem::zeroed() };
    unsafe { function(bytes.as_mut_ptr().cast(), &mut data) };
    let snapshot = unsafe { os_snapshot(&data) };
    unsafe { free_os_data(&mut data) };
    (snapshot, bytes)
}

unsafe fn compare_parse(c: &Library, rust: &Library, input: &str) {
    let c_result = unsafe { parse_call(c, input) };
    let rust_result = unsafe { parse_call(rust, input) };
    assert_eq!(c_result, rust_result, "parse_uname_string input {input:?}");
}

fn run_parse_cases(seed: u64, mut make_input: impl FnMut(&mut Rng) -> String) {
    let (c, rust) = libraries();
    let mut rng = Rng::new(seed);
    for _ in 0..ITERATIONS {
        let input = make_input(&mut rng);
        unsafe { compare_parse(&c, &rust, &input) };
    }
}

macro_rules! arch_config_test {
    ($name:ident, $token:literal, $seed:literal) => {
        #[test]
        fn $name() {
            let (c, rust) = libraries();
            let mut rng = Rng::new($seed);
            for _ in 0..ITERATIONS {
                let input = format!("{}{}{}", rng.safe_word(), $token, rng.safe_word());
                unsafe { compare_arch(&c, &rust, &input) };
            }
        }
    };
}

arch_config_test!(config_01_x86_64, "x86_64", 1);
arch_config_test!(config_02_i386, "i386", 2);
arch_config_test!(config_03_i686, "i686", 3);
arch_config_test!(config_04_sparc, "sparc", 4);
arch_config_test!(config_05_amd64, "amd64", 5);
arch_config_test!(config_06_i86pc, "i86pc", 6);
arch_config_test!(config_07_ia64, "ia64", 7);
arch_config_test!(config_08_aix, "AIX", 8);
arch_config_test!(config_09_armv6, "armv6", 9);
arch_config_test!(config_10_armv7, "armv7", 10);
arch_config_test!(config_11_aarch64, "aarch64", 11);
arch_config_test!(config_12_arm64, "arm64", 12);

#[test]
fn config_13_arch_table_precedence() {
    const TOKENS: [&str; 12] = [
        "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
        "aarch64", "arm64",
    ];
    let (c, rust) = libraries();
    let mut rng = Rng::new(13);
    for _ in 0..ITERATIONS {
        let first = rng.number(11) as usize;
        let second = first + 1 + rng.number((11 - first) as u64) as usize;
        let input = format!("{}--{}", TOKENS[second], TOKENS[first]);
        unsafe { compare_arch(&c, &rust, &input) };
    }
}

#[test]
fn config_14_regex_match_without_offsets() {
    let mut rng = Rng::new(14);
    for _ in 0..ITERATIONS {
        let value = format!("{}", rng.number(1_000_000));
        unsafe { compare_regex("^[0-9]+$", &value, 0) };
    }
}

#[test]
fn config_15_regex_full_match_offsets() {
    let mut rng = Rng::new(15);
    for _ in 0..ITERATIONS {
        let value = format!("XX{}YY", rng.number(1_000_000));
        unsafe { compare_regex("[0-9]+", &value, 1) };
    }
}

#[test]
fn config_16_regex_capture_offsets() {
    let mut rng = Rng::new(16);
    for _ in 0..ITERATIONS {
        let value = format!("{}-{}", rng.safe_word(), rng.number(1_000_000));
        unsafe { compare_regex("^[A-Z]+-([0-9]+)$", &value, 2) };
    }
}

#[test]
fn config_17_regex_no_match() {
    let mut rng = Rng::new(17);
    for _ in 0..ITERATIONS {
        unsafe { compare_regex("^[0-9]+$", &rng.safe_word(), 2) };
    }
}

#[test]
fn config_18_empty_regex_and_string() {
    for _ in 0..ITERATIONS {
        unsafe { compare_regex("", "", 2) };
    }
}

#[test]
fn config_19_large_nmatch() {
    let mut rng = Rng::new(19);
    for _ in 0..ITERATIONS {
        let value = format!("{}-{}", rng.safe_word(), rng.number(1_000_000));
        unsafe { compare_regex("^([A-Z]+)-([0-9]+)$", &value, 64) };
    }
}

#[test]
fn config_20_windows_major_only() {
    run_parse_cases(20, |rng| {
        format!("{} [Ver: {}]", rng.safe_word(), 1 + rng.number(999))
    });
}

#[test]
fn config_21_windows_major_minor() {
    run_parse_cases(21, |rng| {
        format!(
            "{} [Ver: {}.{}]",
            rng.safe_word(),
            1 + rng.number(999),
            rng.number(999)
        )
    });
}

#[test]
fn config_22_windows_three_part_build() {
    run_parse_cases(22, |rng| {
        format!(
            "{} [Ver: {}.{}.{}]",
            rng.safe_word(),
            1 + rng.number(999),
            rng.number(999),
            rng.number(99_999)
        )
    });
}

#[test]
fn config_23_windows_dotted_build() {
    run_parse_cases(23, |rng| {
        format!(
            "{} [Ver: {}.{}.{}.{}]",
            rng.safe_word(),
            1 + rng.number(999),
            rng.number(999),
            rng.number(99_999),
            rng.number(99_999)
        )
    });
}

#[test]
fn config_24_windows_nonnumeric_or_empty() {
    run_parse_cases(24, |rng| {
        let version = if rng.number(2) == 0 {
            String::new()
        } else {
            rng.safe_word()
        };
        format!("{} [Ver: {version}]", rng.safe_word())
    });
}

#[test]
fn config_25_generic_no_colon_no_pipe() {
    run_parse_cases(25, |rng| {
        format!("{} [{}]", rng.safe_word(), rng.safe_word())
    });
}

#[test]
fn config_26_generic_no_colon_with_pipe() {
    run_parse_cases(26, |rng| {
        format!(
            "{} [{}|{}]",
            rng.safe_word(),
            rng.safe_word(),
            rng.safe_word()
        )
    });
}

#[test]
fn config_27_generic_major_only() {
    run_parse_cases(27, |rng| {
        format!(
            "{} [{}: {}]",
            rng.safe_word(),
            rng.safe_word(),
            1 + rng.number(999)
        )
    });
}

#[test]
fn config_28_generic_major_minor() {
    run_parse_cases(28, |rng| {
        format!(
            "{} [{}: {}.{}]",
            rng.safe_word(),
            rng.safe_word(),
            1 + rng.number(999),
            rng.number(999)
        )
    });
}

#[test]
fn config_29_generic_codename() {
    run_parse_cases(29, |rng| {
        format!(
            "{} [{}: {}.{} ({})]",
            rng.safe_word(),
            rng.safe_word(),
            1 + rng.number(999),
            rng.number(999),
            rng.safe_word()
        )
    });
}

#[test]
fn config_30_generic_nonnumeric_codename() {
    run_parse_cases(30, |rng| {
        format!(
            "{} [{}: {} ({})]",
            rng.safe_word(),
            rng.safe_word(),
            rng.safe_word(),
            rng.safe_word()
        )
    });
}

#[test]
fn config_31_generic_platform_and_version() {
    run_parse_cases(31, |rng| {
        format!(
            "{} [{}|{}: {}.{}]",
            rng.safe_word(),
            rng.safe_word(),
            rng.safe_word(),
            1 + rng.number(999),
            rng.number(999)
        )
    });
}

#[test]
fn config_32_no_marker_with_arch() {
    run_parse_cases(32, |rng| {
        format!("{} x86_64 {}", rng.safe_word(), rng.safe_word())
    });
}

#[test]
fn config_33_no_marker_without_arch() {
    run_parse_cases(33, |rng| rng.safe_word());
}

#[test]
fn config_34_generic_arch_in_prefix() {
    run_parse_cases(34, |rng| {
        format!("{} armv7 [{}]", rng.safe_word(), rng.safe_word())
    });
}

#[test]
fn config_35_generic_arch_only_after_marker() {
    run_parse_cases(35, |rng| {
        format!("{} [{} armv7]", rng.safe_word(), rng.safe_word())
    });
}

#[test]
fn config_36_windows_precedence_and_no_arch() {
    run_parse_cases(36, |rng| {
        format!(
            "{} arm64 [Ver: {}.{}.{}]",
            rng.safe_word(),
            1 + rng.number(99),
            rng.number(99),
            rng.number(9999)
        )
    });
}

#[test]
fn config_37_empty_input() {
    run_parse_cases(37, |_| String::new());
}

#[test]
fn config_38_one_byte_input() {
    run_parse_cases(38, |rng| {
        char::from(b'B' + rng.number(24) as u8).to_string()
    });
}

#[test]
fn error_01_arch_not_found_returns_null() {
    let (c, rust) = libraries();
    let mut rng = Rng::new(101);
    for _ in 0..ITERATIONS {
        unsafe { compare_arch(&c, &rust, &rng.safe_word()) };
    }
}

#[test]
fn errors_02_to_04_regex_null_inputs_return_zero() {
    let (c, rust) = libraries();
    let pattern = CString::new("^[0-9]+$").unwrap();
    let string = CString::new("123").unwrap();
    let cases = [
        (None, Some(string.as_c_str())),
        (Some(pattern.as_c_str()), None),
        (None, None),
    ];
    for (case_number, (pattern, string)) in cases.into_iter().enumerate() {
        let c_result = unsafe { regex_call(&c, pattern, string, 2) };
        let rust_result = unsafe { regex_call(&rust, pattern, string, 2) };
        assert_eq!(c_result, rust_result, "ERRORS.md row {}", case_number + 2);
        assert_eq!(c_result.0, 0);
    }
}

#[test]
fn error_05_invalid_regex_returns_zero() {
    let (c, rust) = libraries();
    let pattern = CString::new("([").unwrap();
    let string = CString::new("anything").unwrap();
    let c_result = unsafe { regex_call(&c, Some(pattern.as_c_str()), Some(string.as_c_str()), 2) };
    let rust_result =
        unsafe { regex_call(&rust, Some(pattern.as_c_str()), Some(string.as_c_str()), 2) };
    assert_eq!(c_result, rust_result);
    assert_eq!(c_result.0, 0);

    let c_output = stderr_probe_output(&c_library_path());
    let rust_output = stderr_probe_output(&rust_library_path());
    assert!(c_output.status.success());
    assert!(rust_output.status.success());
    assert_eq!(c_output.stderr, rust_output.stderr);
}

#[test]
fn error_06_null_osd_returns_immediately() {
    let (c, rust) = libraries();
    for library in [&c, &rust] {
        let function = unsafe { library.get::<ParseUname>(b"parse_uname_string\0").unwrap() };
        unsafe {
            function(std::ptr::null_mut(), std::ptr::null_mut());
            let mut input = b"ignored\0".to_vec();
            function(input.as_mut_ptr().cast(), std::ptr::null_mut());
        }
    }
}

#[test]
fn generic_zero_and_oversized_length_boundaries() {
    unsafe {
        compare_regex("a", "a", 0);
        compare_regex("a", "a", 4_096);
        compare_regex("z", "a", 4_096);
    }
}

#[test]
fn ffi_crash_probe() {
    let Ok(probe) = std::env::var("DIFF_CRASH_PROBE") else {
        return;
    };
    let path = std::env::var_os("DIFF_LIBRARY").expect("DIFF_LIBRARY");
    let library = unsafe { Library::new(path).unwrap() };
    unsafe {
        match probe.as_str() {
            "get_os_arch_null" => {
                let function = library.get::<GetOsArch>(b"get_os_arch\0").unwrap();
                let _ = function(std::ptr::null_mut());
            }
            "parse_null_uname" => {
                let function = library.get::<ParseUname>(b"parse_uname_string\0").unwrap();
                let mut data: OsData = std::mem::zeroed();
                function(std::ptr::null_mut(), &mut data);
            }
            "regex_null_matches" => {
                let function = library.get::<WRegexec>(b"w_regexec\0").unwrap();
                let pattern = c"^(a)$";
                let string = c"a";
                let _ = function(pattern.as_ptr(), string.as_ptr(), 2, std::ptr::null_mut());
            }
            other => panic!("unknown probe {other}"),
        }
    }
}

#[test]
fn ffi_stderr_probe() {
    let Some(path) = std::env::var_os("DIFF_STDERR_LIBRARY") else {
        return;
    };
    let library = unsafe { Library::new(path).unwrap() };
    let pattern = c"([";
    let string = c"anything";
    let mut matches = [RegMatch {
        rm_so: -1,
        rm_eo: -1,
    }; 2];
    let function = unsafe { library.get::<WRegexec>(b"w_regexec\0").unwrap() };
    assert_eq!(
        unsafe { function(pattern.as_ptr(), string.as_ptr(), 2, matches.as_mut_ptr()) },
        0
    );
}

fn crash_status(library: &Path, probe: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("ffi_crash_probe")
        .arg("--exact")
        .env("DIFF_LIBRARY", library)
        .env("DIFF_CRASH_PROBE", probe)
        .env("RUST_TEST_THREADS", "1")
        .status()
        .unwrap()
}

fn stderr_probe_output(library: &Path) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("ffi_stderr_probe")
        .arg("--exact")
        .arg("--nocapture")
        .env("DIFF_STDERR_LIBRARY", library)
        .env("RUST_TEST_THREADS", "1")
        .output()
        .unwrap()
}

#[cfg(unix)]
#[test]
fn generic_invalid_pointer_crash_parity() {
    use std::os::unix::process::ExitStatusExt;

    for probe in ["get_os_arch_null", "parse_null_uname", "regex_null_matches"] {
        let c_status = crash_status(&c_library_path(), probe);
        let rust_status = crash_status(&rust_library_path(), probe);
        assert!(!c_status.success(), "C unexpectedly accepted {probe}");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different terminating signal for {probe}"
        );
    }
}
