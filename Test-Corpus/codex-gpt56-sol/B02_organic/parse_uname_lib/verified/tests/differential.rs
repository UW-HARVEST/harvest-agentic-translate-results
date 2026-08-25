use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};
use std::ptr;

const ARCHES: [&str; 12] = [
    "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
    "aarch64", "arm64",
];
const RANDOM_CASES: usize = 64;

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
    fn zeroed() -> Self {
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

type GetOsArch = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
type WRegexec = unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut RegMatch) -> c_int;
type ParseUname = unsafe extern "C" fn(*mut c_char, *mut OsData);

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
        let get_os_arch = unsafe { *library.get(b"get_os_arch\0").unwrap() };
        let w_regexec = unsafe { *library.get(b"w_regexec\0").unwrap() };
        let parse_uname_string = unsafe { *library.get(b"parse_uname_string\0").unwrap() };
        Self {
            _library: library,
            get_os_arch,
            w_regexec,
            parse_uname_string,
        }
    }
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
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

unsafe fn copy_bytes(pointer: *mut c_char) -> Option<Vec<u8>> {
    (!pointer.is_null()).then(|| unsafe { CStr::from_ptr(pointer).to_bytes().to_vec() })
}

unsafe fn snapshot_and_free(osd: OsData) -> OsSnapshot {
    let snapshot = OsSnapshot {
        os_name: unsafe { copy_bytes(osd.os_name) },
        os_version: unsafe { copy_bytes(osd.os_version) },
        os_major: unsafe { copy_bytes(osd.os_major) },
        os_minor: unsafe { copy_bytes(osd.os_minor) },
        os_codename: unsafe { copy_bytes(osd.os_codename) },
        os_platform: unsafe { copy_bytes(osd.os_platform) },
        os_build: unsafe { copy_bytes(osd.os_build) },
        os_uname: unsafe { copy_bytes(osd.os_uname) },
        os_arch: unsafe { copy_bytes(osd.os_arch) },
    };
    for pointer in [
        osd.os_name,
        osd.os_version,
        osd.os_major,
        osd.os_minor,
        osd.os_codename,
        osd.os_platform,
        osd.os_build,
        osd.os_uname,
        osd.os_arch,
    ] {
        if !pointer.is_null() {
            unsafe { free(pointer.cast()) };
        }
    }
    snapshot
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/debug/libdriver.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(rust_path.is_file(), "missing {}", rust_path.display());
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn with_nul(input: &str) -> Vec<u8> {
    CString::new(input).unwrap().into_bytes_with_nul()
}

unsafe fn owned_result(pointer: *mut c_char) -> Option<Vec<u8>> {
    if pointer.is_null() {
        None
    } else {
        let bytes = unsafe { CStr::from_ptr(pointer).to_bytes().to_vec() };
        unsafe { free(pointer.cast()) };
        Some(bytes)
    }
}

fn compare_get_os_arch(c: &Api, rust: &Api, input: &str) -> Option<Vec<u8>> {
    let mut c_input = with_nul(input);
    let mut rust_input = with_nul(input);
    let c_result = unsafe { owned_result((c.get_os_arch)(c_input.as_mut_ptr().cast::<c_char>())) };
    let rust_result =
        unsafe { owned_result((rust.get_os_arch)(rust_input.as_mut_ptr().cast::<c_char>())) };
    assert_eq!(
        c_input, rust_input,
        "get_os_arch mutated input for {input:?}"
    );
    assert_eq!(c_result, rust_result, "get_os_arch mismatch for {input:?}");
    c_result
}

fn compare_regex(
    c: &Api,
    rust: &Api,
    pattern: Option<&str>,
    string: Option<&str>,
    nmatch: usize,
    slots: usize,
) -> (c_int, Vec<RegMatch>) {
    let pattern = pattern.map(|value| CString::new(value).unwrap());
    let string = string.map(|value| CString::new(value).unwrap());
    let pattern_pointer = pattern.as_ref().map_or(ptr::null(), |value| value.as_ptr());
    let string_pointer = string.as_ref().map_or(ptr::null(), |value| value.as_ptr());
    let mut c_matches = vec![
        RegMatch {
            rm_so: -77,
            rm_eo: -88,
        };
        slots
    ];
    let mut rust_matches = c_matches.clone();
    let c_match_pointer = if slots == 0 {
        ptr::null_mut()
    } else {
        c_matches.as_mut_ptr()
    };
    let rust_match_pointer = if slots == 0 {
        ptr::null_mut()
    } else {
        rust_matches.as_mut_ptr()
    };
    let c_result =
        unsafe { (c.w_regexec)(pattern_pointer, string_pointer, nmatch, c_match_pointer) };
    let rust_result =
        unsafe { (rust.w_regexec)(pattern_pointer, string_pointer, nmatch, rust_match_pointer) };
    assert_eq!(
        c_result, rust_result,
        "w_regexec return mismatch for pattern={pattern:?}, string={string:?}, nmatch={nmatch}"
    );
    assert_eq!(
        c_matches, rust_matches,
        "w_regexec slots mismatch for pattern={pattern:?}, string={string:?}, nmatch={nmatch}"
    );
    (c_result, c_matches)
}

fn compare_parse(c: &Api, rust: &Api, input: &str) -> OsSnapshot {
    let mut c_input = with_nul(input);
    let mut rust_input = with_nul(input);
    let mut c_osd = OsData::zeroed();
    let mut rust_osd = OsData::zeroed();
    unsafe {
        (c.parse_uname_string)(c_input.as_mut_ptr().cast(), &mut c_osd);
        (rust.parse_uname_string)(rust_input.as_mut_ptr().cast(), &mut rust_osd);
    }
    let c_snapshot = unsafe { snapshot_and_free(c_osd) };
    let rust_snapshot = unsafe { snapshot_and_free(rust_osd) };
    assert_eq!(c_input, rust_input, "uname mutation mismatch for {input:?}");
    assert_eq!(
        c_snapshot, rust_snapshot,
        "parse_uname_string mismatch for {input:?}"
    );
    c_snapshot
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + self.next() as usize % (maximum - minimum)
    }

    fn noise(&mut self, minimum: usize, maximum: usize) -> String {
        const ALPHABET: &[u8] = b"qzQZ_-";
        let length = self.range(minimum, maximum);
        (0..length)
            .map(|_| ALPHABET[self.range(0, ALPHABET.len())] as char)
            .collect()
    }

    fn letters(&mut self, minimum: usize, maximum: usize) -> String {
        const ALPHABET: &[u8] = b"bcdefghjklmnopqstuvwxyz";
        let length = self.range(minimum, maximum);
        (0..length)
            .map(|_| ALPHABET[self.range(0, ALPHABET.len())] as char)
            .collect()
    }

    fn digits(&mut self, minimum: usize, maximum: usize) -> String {
        let length = self.range(minimum, maximum);
        (0..length)
            .map(|_| b'0' + self.range(0, 10) as u8)
            .map(char::from)
            .collect()
    }
}

#[test]
fn configs_c01_through_c13_get_os_arch() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x1357_2468_9abc_def0);

    for architecture in ARCHES {
        for _ in 0..RANDOM_CASES {
            let input = format!("{}{}{}", rng.noise(0, 12), architecture, rng.noise(0, 12));
            assert_eq!(
                compare_get_os_arch(&c, &rust, &input),
                Some(architecture.as_bytes().to_vec())
            );
        }
    }

    for _ in 0..RANDOM_CASES {
        let first_index = rng.range(0, ARCHES.len() - 1);
        let second_index = rng.range(first_index + 1, ARCHES.len());
        let input = format!(
            "{}{}{}{}{}",
            rng.noise(0, 8),
            ARCHES[second_index],
            rng.noise(1, 8),
            ARCHES[first_index],
            rng.noise(0, 8)
        );
        assert_eq!(
            compare_get_os_arch(&c, &rust, &input),
            Some(ARCHES[first_index].as_bytes().to_vec())
        );
    }
}

#[test]
fn configs_c14_through_c19_w_regexec() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xfedc_ba98_7654_3210);

    for _ in 0..RANDOM_CASES {
        let string = format!("{}needle{}", rng.letters(0, 12), rng.letters(0, 12));
        assert_eq!(
            compare_regex(&c, &rust, Some("needle"), Some(&string), 0, 0).0,
            1
        );

        let digits = rng.digits(1, 10);
        let string = format!("{}{}{}", rng.letters(0, 8), digits, rng.letters(0, 8));
        let (result, matches) = compare_regex(&c, &rust, Some("[0-9]+"), Some(&string), 1, 1);
        assert_eq!(result, 1);
        assert!(matches[0].rm_so >= 0);

        let letters = rng.letters(1, 10);
        let digits = rng.digits(1, 10);
        let string = format!("{letters}-{digits}");
        assert_eq!(
            compare_regex(&c, &rust, Some("^([a-z]+)-[0-9]+$"), Some(&string), 2, 2,).0,
            1
        );
        assert_eq!(
            compare_regex(&c, &rust, Some("^([a-z]+)-([0-9]+)$"), Some(&string), 2, 2,).0,
            1
        );

        let (_, matches) = compare_regex(&c, &rust, Some("^([a-z]+)-[0-9]+$"), Some(&string), 5, 5);
        assert_eq!(
            matches[2..],
            [RegMatch {
                rm_so: -1,
                rm_eo: -1,
            }; 3]
        );

        let repetitions = rng.range(1, 8);
        let atom = if rng.next() & 1 == 0 { "ab" } else { "cd" };
        let string = format!("{}{}", atom.repeat(repetitions), rng.digits(1, 4));
        assert_eq!(
            compare_regex(
                &c,
                &rust,
                Some("^(ab|cd)+[[:digit:]]{1,3}$"),
                Some(&string),
                3,
                3,
            )
            .0,
            1
        );
    }
}

#[test]
fn configs_c20_through_c24_windows_parse() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x0123_4567_89ab_cdef);

    for _ in 0..RANDOM_CASES {
        let name = rng.letters(1, 12);
        let major = rng.digits(1, 8);
        compare_parse(&c, &rust, &format!("{name} [Ver: {major}]"));

        let minor = rng.digits(1, 8);
        compare_parse(&c, &rust, &format!("{name} [Ver: {major}.{minor}]"));

        let build = rng.digits(1, 8);
        compare_parse(&c, &rust, &format!("{name} [Ver: {major}.{minor}.{build}]"));

        let more = rng.digits(1, 8);
        compare_parse(
            &c,
            &rust,
            &format!("{name} [Ver: {major}.{minor}.{build}.{more}]"),
        );

        let version = rng.letters(1, 12);
        compare_parse(&c, &rust, &format!("{name} [Ver: {version}]"));
    }
}

#[test]
fn configs_c25_through_c33_bracketed_parse() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xa5a5_5a5a_1122_3344);

    for _ in 0..RANDOM_CASES {
        let name = rng.letters(1, 12);
        let major = rng.digits(1, 8);
        let minor = rng.digits(1, 8);
        let codename = rng.letters(1, 12);
        let platform = rng.letters(1, 12);
        let text = rng.letters(1, 12);

        compare_parse(&c, &rust, &format!("Host [{name}: {major}]"));
        compare_parse(&c, &rust, &format!("Host [{name}: {major}.{minor}]"));
        compare_parse(
            &c,
            &rust,
            &format!("Host [{name}: {major}.{minor} ({codename})]"),
        );
        compare_parse(&c, &rust, &format!("Host [{name}: {text}]"));
        compare_parse(
            &c,
            &rust,
            &format!("Host [{name}|{platform}: {major}.{minor}]"),
        );
        compare_parse(&c, &rust, &format!("Host [{name}: {major}.{minor}|{text}]"));
        compare_parse(&c, &rust, &format!("Host [{name}]"));
        compare_parse(&c, &rust, &format!("Host [{name}|{platform}]"));

        let architecture = ARCHES[rng.range(0, ARCHES.len())];
        compare_parse(
            &c,
            &rust,
            &format!("{architecture} Host [{name}: {major}.{minor} ({codename})]"),
        );
    }
}

#[test]
fn configs_c34_through_c46_arch_only_and_empty_parse() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x55aa_33cc_77ee_1199);

    for architecture in ARCHES {
        for _ in 0..RANDOM_CASES {
            let input = format!("{} {} {}", rng.noise(0, 10), architecture, rng.noise(0, 10));
            let snapshot = compare_parse(&c, &rust, &input);
            assert_eq!(snapshot.os_arch, Some(architecture.as_bytes().to_vec()));
        }
    }

    compare_parse(&c, &rust, "");
    for _ in 0..RANDOM_CASES {
        compare_parse(&c, &rust, &rng.noise(1, 24));
    }
}

#[test]
fn errors_e01_through_e06_match_exactly() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xdead_beef_cafe_babe);

    assert_eq!(compare_get_os_arch(&c, &rust, ""), None);
    for _ in 0..RANDOM_CASES {
        assert_eq!(compare_get_os_arch(&c, &rust, &rng.noise(1, 32)), None);
    }

    for _ in 0..RANDOM_CASES {
        let string = rng.letters(0, 24);
        assert_eq!(compare_regex(&c, &rust, None, Some(&string), 1, 1).0, 0);
        assert_eq!(compare_regex(&c, &rust, Some("[a-z]+"), None, 1, 1).0, 0);
        assert_eq!(
            compare_regex(&c, &rust, Some("("), Some(&string), 1, 1).0,
            0
        );
        assert_eq!(
            compare_regex(&c, &rust, Some("^must-match$"), Some(&string), 1, 1).0,
            0
        );
    }

    let mut c_input = with_nul("unchanged [Ver: 1.2.3]");
    let mut rust_input = c_input.clone();
    unsafe {
        (c.parse_uname_string)(c_input.as_mut_ptr().cast(), ptr::null_mut());
        (rust.parse_uname_string)(rust_input.as_mut_ptr().cast(), ptr::null_mut());
        (c.parse_uname_string)(ptr::null_mut(), ptr::null_mut());
        (rust.parse_uname_string)(ptr::null_mut(), ptr::null_mut());
    }
    assert_eq!(c_input, rust_input);
    assert_eq!(c_input, with_nul("unchanged [Ver: 1.2.3]"));
}

fn run_probe(library: &Path, probe: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_boundary_probe")
        .arg("--nocapture")
        .env("DIFFERENTIAL_PROBE_LIBRARY", library)
        .env("DIFFERENTIAL_PROBE_CASE", probe)
        .output()
        .unwrap()
}

fn same_termination(left: ExitStatus, right: ExitStatus) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        left.code() == right.code() && left.signal() == right.signal()
    }
    #[cfg(not(unix))]
    {
        left.code() == right.code()
    }
}

#[test]
fn generic_ffi_boundaries_match() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    for probe in [
        "get-null",
        "regex-null-pmatch",
        "regex-oversized-nmatch",
        "parse-null-uname",
    ] {
        let c_output = run_probe(&c_path, probe);
        let rust_output = run_probe(&rust_path, probe);
        assert!(
            same_termination(c_output.status, rust_output.status),
            "{probe}: C status {:?}, Rust status {:?}",
            c_output.status,
            rust_output.status
        );
    }

    let (c, rust) = load_apis();
    assert_eq!(compare_regex(&c, &rust, Some("a"), Some("a"), 0, 0).0, 1);
}

#[test]
fn invalid_regex_diagnostic_matches() {
    let c_output = run_probe(&c_library_path(), "invalid-regex");
    let rust_output = run_probe(&rust_library_path(), "invalid-regex");
    assert!(c_output.status.success());
    assert!(rust_output.status.success());
    assert_eq!(c_output.stderr, rust_output.stderr);
    assert_eq!(
        c_output.stderr,
        b"Couldn't compile regular expression '('\n"
    );
}

#[test]
fn ffi_boundary_probe() {
    let Ok(library_path) = std::env::var("DIFFERENTIAL_PROBE_LIBRARY") else {
        return;
    };
    let probe = std::env::var("DIFFERENTIAL_PROBE_CASE").unwrap();
    let api = unsafe { Api::load(Path::new(&library_path)) };
    unsafe {
        match probe.as_str() {
            "get-null" => {
                (api.get_os_arch)(ptr::null_mut());
            }
            "regex-null-pmatch" => {
                (api.w_regexec)(c"a".as_ptr(), c"a".as_ptr(), 1, ptr::null_mut());
            }
            "regex-oversized-nmatch" => {
                let mut one_match = RegMatch {
                    rm_so: -1,
                    rm_eo: -1,
                };
                (api.w_regexec)(c"a".as_ptr(), c"a".as_ptr(), usize::MAX, &mut one_match);
            }
            "parse-null-uname" => {
                let mut osd = OsData::zeroed();
                (api.parse_uname_string)(ptr::null_mut(), &mut osd);
            }
            "invalid-regex" => {
                let mut one_match = RegMatch {
                    rm_so: -1,
                    rm_eo: -1,
                };
                let result = (api.w_regexec)(c"(".as_ptr(), c"text".as_ptr(), 1, &mut one_match);
                assert_eq!(result, 0);
            }
            other => panic!("unknown probe {other}"),
        }
    }
}
