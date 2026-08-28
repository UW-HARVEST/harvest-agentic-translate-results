use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

const ARCHS: [&str; 12] = [
    "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
    "aarch64", "arm64",
];
const ITERATIONS: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

impl OsData {
    fn empty() -> Self {
        Self {
            os_name: ptr::null_mut(),
            os_version: ptr::null_mut(),
            os_major: ptr::null_mut(),
            os_minor: ptr::null_mut(),
            os_codename: ptr::null_mut(),
            os_platform: ptr::null_mut(),
            os_build: ptr::null_mut(),
            os_uname: ptr::null_mut(),
            os_arch: ptr::null_mut(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct OsSnapshot {
    os_name: Option<Vec<u8>>,
    os_version: Option<Vec<u8>>,
    os_major: Option<Vec<u8>>,
    os_minor: Option<Vec<u8>>,
    os_codename: Option<Vec<u8>>,
    os_platform: Option<Vec<u8>>,
    os_build: Option<Vec<u8>>,
    os_uname: Option<Vec<u8>>,
    os_arch: Option<Vec<u8>>,
}

type GetOsArch = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
type WRegexec = unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut RegMatch) -> c_int;
type ParseUname = unsafe extern "C" fn(*mut c_char, *mut OsData);

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

struct Api {
    _library: Library,
    get_os_arch: GetOsArch,
    w_regexec: WRegexec,
    parse_uname_string: ParseUname,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let get_os_arch = unsafe { *library.get::<GetOsArch>(b"get_os_arch\0").unwrap() };
        let w_regexec = unsafe { *library.get::<WRegexec>(b"w_regexec\0").unwrap() };
        let parse_uname_string =
            unsafe { *library.get::<ParseUname>(b"parse_uname_string\0").unwrap() };
        Self {
            _library: library,
            get_os_arch,
            w_regexec,
            parse_uname_string,
        }
    }
}

struct Apis {
    c: Api,
    rust: Api,
}

impl Apis {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libdriver.so");
        let rust_path = std::env::var_os("DRIVER_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("target/release/libdriver.so"));
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
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

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + (self.next() as usize % (end - start))
    }

    fn word(&mut self, min: usize, max: usize) -> String {
        const SAFE: &[u8] = b"bcdefghjklmnopqstuwyz";
        let len = self.range(min, max + 1);
        (0..len)
            .map(|_| SAFE[self.range(0, SAFE.len())] as char)
            .collect()
    }

    fn digits(&mut self, min: usize, max: usize) -> String {
        let len = self.range(min, max + 1);
        (0..len)
            .map(|index| {
                let low = usize::from(index == 0 && len > 1);
                (b'0' + self.range(low, 10) as u8) as char
            })
            .collect()
    }
}

fn c_buffer(value: &str) -> Vec<u8> {
    assert!(!value.as_bytes().contains(&0));
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    bytes
}

unsafe fn take_string(pointer: *mut c_char) -> Option<Vec<u8>> {
    if pointer.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(pointer) }.to_bytes().to_vec();
    unsafe { free(pointer.cast()) };
    Some(value)
}

unsafe fn take_os_data(data: OsData) -> OsSnapshot {
    OsSnapshot {
        os_name: unsafe { take_string(data.os_name) },
        os_version: unsafe { take_string(data.os_version) },
        os_major: unsafe { take_string(data.os_major) },
        os_minor: unsafe { take_string(data.os_minor) },
        os_codename: unsafe { take_string(data.os_codename) },
        os_platform: unsafe { take_string(data.os_platform) },
        os_build: unsafe { take_string(data.os_build) },
        os_uname: unsafe { take_string(data.os_uname) },
        os_arch: unsafe { take_string(data.os_arch) },
    }
}

fn call_arch(function: GetOsArch, input: &str) -> Option<Vec<u8>> {
    let mut input = c_buffer(input);
    unsafe { take_string(function(input.as_mut_ptr().cast())) }
}

fn compare_arch(apis: &Apis, row: &str, input: &str) {
    let c = call_arch(apis.c.get_os_arch, input);
    let rust = call_arch(apis.rust.get_os_arch, input);
    assert_eq!(c, rust, "{row}: input={input:?}");
}

fn call_regex(
    function: WRegexec,
    pattern: Option<&str>,
    string: Option<&str>,
    nmatch: usize,
) -> (c_int, Vec<RegMatch>) {
    let pattern = pattern.map(c_buffer);
    let string = string.map(c_buffer);
    let pattern_pointer = pattern
        .as_ref()
        .map_or(ptr::null(), |value| value.as_ptr().cast());
    let string_pointer = string
        .as_ref()
        .map_or(ptr::null(), |value| value.as_ptr().cast());
    let mut matches = vec![
        RegMatch {
            rm_so: -777,
            rm_eo: -888,
        };
        nmatch
    ];
    let match_pointer = if nmatch == 0 {
        ptr::null_mut()
    } else {
        matches.as_mut_ptr()
    };
    let result = unsafe { function(pattern_pointer, string_pointer, nmatch, match_pointer) };
    (result, matches)
}

fn compare_regex(
    apis: &Apis,
    row: &str,
    pattern: Option<&str>,
    string: Option<&str>,
    nmatch: usize,
) {
    let c = call_regex(apis.c.w_regexec, pattern, string, nmatch);
    let rust = call_regex(apis.rust.w_regexec, pattern, string, nmatch);
    assert_eq!(
        c, rust,
        "{row}: pattern={pattern:?}, string={string:?}, nmatch={nmatch}"
    );
}

fn call_parse(function: ParseUname, input: &str) -> (Vec<u8>, OsSnapshot) {
    let mut input = c_buffer(input);
    let mut data = OsData::empty();
    unsafe { function(input.as_mut_ptr().cast(), &mut data) };
    let snapshot = unsafe { take_os_data(data) };
    (input, snapshot)
}

fn compare_parse(apis: &Apis, row: &str, input: &str) {
    let c = call_parse(apis.c.parse_uname_string, input);
    let rust = call_parse(apis.rust.parse_uname_string, input);
    assert_eq!(c, rust, "{row}: input={input:?}");
}

fn child_signal(action: impl FnOnce()) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            action();
            _exit(0);
        }
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);
        status & 0x7f
    }
}

#[test]
fn phase_b_get_os_arch_rows_c01_c14() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xC01_C14);

    for (index, arch) in ARCHS.iter().enumerate() {
        let row = format!("C{:02}", index + 1);
        for _ in 0..ITERATIONS {
            let input = format!("{}{}{}", rng.word(0, 20), arch, rng.word(0, 20));
            compare_arch(&apis, &row, &input);
        }
    }

    for _ in 0..ITERATIONS {
        let first = rng.range(0, ARCHS.len());
        let mut second = rng.range(0, ARCHS.len());
        if first == second {
            second = (second + 1) % ARCHS.len();
        }
        let input = if rng.next() & 1 == 0 {
            format!("{}-{}", ARCHS[first], ARCHS[second])
        } else {
            format!("{}-{}", ARCHS[second], ARCHS[first])
        };
        compare_arch(&apis, "C13", &input);
    }

    compare_arch(&apis, "C14", "");
    for _ in 1..ITERATIONS {
        let input = rng.word(1, 80);
        compare_arch(&apis, "C14", &input);
    }
}

#[test]
fn phase_b_w_regexec_rows_c15_c20() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xC15_C20);

    for _ in 0..ITERATIONS {
        compare_regex(&apis, "C15", Some(""), Some(""), 0);

        let literal = rng.word(1, 8);
        let string = format!("{}{}{}", rng.word(0, 8), literal, rng.word(0, 8));
        compare_regex(&apis, "C16", Some(&literal), Some(&string), 0);
        compare_regex(&apis, "C17", Some(&literal), Some(&string), 1);

        let digits = rng.digits(1, 8);
        let captured = format!("{}-{digits}-{}", rng.word(1, 5), rng.word(1, 5));
        compare_regex(
            &apis,
            "C18",
            Some("^[a-z]+-([0-9]+)-[a-z]+$"),
            Some(&captured),
            2,
        );
        compare_regex(
            &apis,
            "C19",
            Some("^([a-z]+)-[0-9]+$"),
            Some(&format!("{}-{digits}", rng.word(1, 8))),
            5,
        );
        compare_regex(&apis, "C20", Some("^z[0-9]+$"), Some(&rng.word(0, 8)), 2);
    }
}

#[test]
fn phase_b_parse_windows_rows_c21_c25() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xC21_C25);

    for _ in 0..ITERATIONS {
        let name = rng.word(1, 20);
        let versions = [
            ("C21", rng.word(1, 12)),
            ("C22", rng.digits(1, 6)),
            ("C23", format!("{}.{}", rng.digits(1, 5), rng.digits(1, 5))),
            (
                "C24",
                format!(
                    "{}.{}.{}",
                    rng.digits(1, 5),
                    rng.digits(1, 5),
                    rng.digits(1, 7)
                ),
            ),
            (
                "C25",
                format!(
                    "{}.{}.{}.{}.{}",
                    rng.digits(1, 5),
                    rng.digits(1, 5),
                    rng.digits(1, 5),
                    rng.digits(1, 5),
                    rng.digits(1, 5)
                ),
            ),
        ];
        for (row, version) in versions {
            compare_parse(&apis, row, &format!("{name} [Ver: {version}]"));
        }
    }
}

#[test]
fn phase_b_parse_no_marker_rows_c26_c38() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xC26_C38);

    for (index, arch) in ARCHS.iter().enumerate() {
        let row = format!("C{:02}", index + 26);
        for _ in 0..ITERATIONS {
            let input = format!("{}{}{}", rng.word(0, 20), arch, rng.word(0, 20));
            compare_parse(&apis, &row, &input);
        }
    }

    compare_parse(&apis, "C38", "");
    for _ in 1..ITERATIONS {
        compare_parse(&apis, "C38", &rng.word(1, 80));
    }
}

#[test]
fn phase_b_parse_no_colon_rows_c39_c42() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xC39_C42);

    for platform in [false, true] {
        for architecture in [false, true] {
            let offset = usize::from(platform) * 2 + usize::from(architecture);
            let row = format!("C{:02}", 39 + offset);
            for _ in 0..ITERATIONS {
                let prefix = if architecture {
                    format!(
                        "{}{}{}",
                        rng.word(0, 8),
                        ARCHS[rng.range(0, 12)],
                        rng.word(0, 8)
                    )
                } else {
                    rng.word(0, 16)
                };
                let name = rng.word(1, 16);
                let name = if platform {
                    format!("{name}|{}", rng.word(0, 12))
                } else {
                    name
                };
                compare_parse(&apis, &row, &format!("{prefix} [{name}]"));
            }
        }
    }
}

#[test]
fn phase_b_parse_colon_rows_c43_c66() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xC43_C66);

    for version_shape in 0..3 {
        for codename in [false, true] {
            for platform in [false, true] {
                for architecture in [false, true] {
                    let offset = version_shape * 8
                        + usize::from(codename) * 4
                        + usize::from(platform) * 2
                        + usize::from(architecture);
                    let row = format!("C{:02}", 43 + offset);
                    for _ in 0..ITERATIONS {
                        let prefix = if architecture {
                            format!(
                                "{}{}{}",
                                rng.word(0, 8),
                                ARCHS[rng.range(0, 12)],
                                rng.word(0, 8)
                            )
                        } else {
                            rng.word(0, 16)
                        };
                        let name = rng.word(1, 16);
                        let name = if platform {
                            format!("{name}|{}", rng.word(0, 12))
                        } else {
                            name
                        };
                        let version = match version_shape {
                            0 => rng.word(1, 12),
                            1 => rng.digits(1, 8),
                            2 => format!(
                                "{}.{}.{}",
                                rng.digits(1, 6),
                                rng.digits(1, 6),
                                rng.digits(1, 6)
                            ),
                            _ => unreachable!(),
                        };
                        let version = if codename {
                            format!("{version} ({})", rng.word(0, 12))
                        } else {
                            version
                        };
                        compare_parse(&apis, &row, &format!("{prefix} [{name}: {version}]"));
                    }
                }
            }
        }
    }
}

#[test]
fn phase_c_explicit_errors_e01_e06() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xE01_E06);

    for _ in 0..ITERATIONS {
        compare_arch(&apis, "E01", &rng.word(0, 64));
        compare_regex(&apis, "E02", None, Some(&rng.word(0, 20)), 0);
        compare_regex(&apis, "E03", Some("^[a-z]+$"), None, 0);
        compare_regex(&apis, "E04", Some("([unterminated"), Some("text"), 2);
        compare_regex(&apis, "E05", Some("^[0-9]+$"), Some(&rng.word(0, 20)), 2);

        let input = rng.word(0, 32);
        let mut c_input = c_buffer(&input);
        let mut rust_input = c_input.clone();
        unsafe {
            (apis.c.parse_uname_string)(c_input.as_mut_ptr().cast(), ptr::null_mut());
            (apis.rust.parse_uname_string)(rust_input.as_mut_ptr().cast(), ptr::null_mut());
        }
        assert_eq!(c_input, rust_input, "E06: input={input:?}");
        assert_eq!(c_input, c_buffer(&input), "E06 modified the input");
    }
}

#[test]
fn phase_c_generic_pointer_and_length_boundaries() {
    let apis = Apis::load();

    compare_arch(&apis, "empty string", "");
    compare_arch(&apis, "large string", &"q".repeat(65_536));
    compare_regex(&apis, "zero nmatch", Some("^$"), Some(""), 0);
    compare_parse(&apis, "empty uname", "");
    compare_parse(&apis, "large uname", &"q".repeat(65_536));

    let c_arch_signal = child_signal(|| unsafe {
        (apis.c.get_os_arch)(ptr::null_mut());
    });
    let rust_arch_signal = child_signal(|| unsafe {
        (apis.rust.get_os_arch)(ptr::null_mut());
    });
    assert_eq!(c_arch_signal, rust_arch_signal, "get_os_arch(NULL)");

    let c_parse_signal = child_signal(|| unsafe {
        let mut data = OsData::empty();
        (apis.c.parse_uname_string)(ptr::null_mut(), &mut data);
    });
    let rust_parse_signal = child_signal(|| unsafe {
        let mut data = OsData::empty();
        (apis.rust.parse_uname_string)(ptr::null_mut(), &mut data);
    });
    assert_eq!(
        c_parse_signal, rust_parse_signal,
        "parse_uname_string(NULL, osd)"
    );

    let pattern = c_buffer("^q$");
    let string = c_buffer("q");
    let c_match_signal = child_signal(|| unsafe {
        (apis.c.w_regexec)(
            pattern.as_ptr().cast(),
            string.as_ptr().cast(),
            1,
            ptr::null_mut(),
        );
    });
    let rust_match_signal = child_signal(|| unsafe {
        (apis.rust.w_regexec)(
            pattern.as_ptr().cast(),
            string.as_ptr().cast(),
            1,
            ptr::null_mut(),
        );
    });
    assert_eq!(
        c_match_signal, rust_match_signal,
        "w_regexec matching with nmatch=1 and pmatch=NULL"
    );

    let no_match = c_buffer("not-q");
    let c_oversized_signal = child_signal(|| unsafe {
        (apis.c.w_regexec)(
            pattern.as_ptr().cast(),
            no_match.as_ptr().cast(),
            usize::MAX,
            ptr::null_mut(),
        );
    });
    let rust_oversized_signal = child_signal(|| unsafe {
        (apis.rust.w_regexec)(
            pattern.as_ptr().cast(),
            no_match.as_ptr().cast(),
            usize::MAX,
            ptr::null_mut(),
        );
    });
    assert_eq!(
        c_oversized_signal, rust_oversized_signal,
        "w_regexec oversized nmatch"
    );
}
