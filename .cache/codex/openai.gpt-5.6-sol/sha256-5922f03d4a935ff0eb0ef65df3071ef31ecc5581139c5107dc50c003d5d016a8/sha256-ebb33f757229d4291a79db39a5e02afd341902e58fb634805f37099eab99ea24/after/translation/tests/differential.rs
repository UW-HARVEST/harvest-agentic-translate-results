use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ConfigFlags {
    bits: c_uint,
}

type ParseEnvNumeric = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
type InitConfigFromEnv = unsafe extern "C" fn(*mut ConfigFlags);
type PerformOperation = unsafe extern "C" fn(c_int, c_int, *mut ConfigFlags) -> c_int;
type ApplyBitOperations = unsafe extern "C" fn(c_int, *mut ConfigFlags) -> c_int;
type Envy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    parse_env_numeric: ParseEnvNumeric,
    init_config_from_env: InitConfigFromEnv,
    perform_operation: PerformOperation,
    apply_bit_operations: ApplyBitOperations,
    envy: Envy,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let parse_env_numeric = unsafe {
            *library
                .get::<ParseEnvNumeric>(b"parse_env_numeric\0")
                .unwrap()
        };
        let init_config_from_env = unsafe {
            *library
                .get::<InitConfigFromEnv>(b"init_config_from_env\0")
                .unwrap()
        };
        let perform_operation = unsafe {
            *library
                .get::<PerformOperation>(b"perform_operation\0")
                .unwrap()
        };
        let apply_bit_operations = unsafe {
            *library
                .get::<ApplyBitOperations>(b"apply_bit_operations\0")
                .unwrap()
        };
        let envy = unsafe { *library.get::<Envy>(b"envy\0").unwrap() };
        Self {
            _library: library,
            parse_env_numeric,
            init_config_from_env,
            perform_operation,
            apply_bit_operations,
            envy,
        }
    }
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        manifest.join("../c_src/build/libharvest-work-PBWdNu.so"),
        manifest.join("target/release/libenvy_lib.so"),
    )
}

fn set_env(name: &str, value: Option<&str>) {
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}

fn clear_program_env() {
    for name in [
        "PROG_VERBOSE",
        "PROG_DEBUG",
        "PROG_OPTIMIZE",
        "PROG_BASE_OFFSET",
        "PROG_MULTIPLIER",
        "ENVY_DIFFERENTIAL_VALUE",
    ] {
        set_env(name, None);
    }
}

fn read_fd(fd: c_int) -> Vec<u8> {
    let mut bytes = Vec::new();
    unsafe {
        let mut file = File::from_raw_fd(fd);
        file.read_to_end(&mut bytes).unwrap();
    }
    bytes
}

fn capture_io<R>(call: impl FnOnce() -> R) -> (R, Vec<u8>, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut stdout_pipe = [0; 2];
        let mut stderr_pipe = [0; 2];
        assert_eq!(pipe(stdout_pipe.as_mut_ptr()), 0);
        assert_eq!(pipe(stderr_pipe.as_mut_ptr()), 0);

        let saved_stdout = dup(1);
        let saved_stderr = dup(2);
        assert!(saved_stdout >= 0 && saved_stderr >= 0);
        assert_eq!(dup2(stdout_pipe[1], 1), 1);
        assert_eq!(dup2(stderr_pipe[1], 2), 2);

        let result = call();
        assert_eq!(fflush(ptr::null_mut()), 0);

        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(dup2(saved_stderr, 2), 2);
        assert_eq!(close(saved_stdout), 0);
        assert_eq!(close(saved_stderr), 0);
        assert_eq!(close(stdout_pipe[1]), 0);
        assert_eq!(close(stderr_pipe[1]), 0);

        (result, read_fd(stdout_pipe[0]), read_fd(stderr_pipe[0]))
    }
}

fn assert_observation_eq<T: std::fmt::Debug + PartialEq>(
    context: &str,
    c: (T, Vec<u8>, Vec<u8>),
    rust: (T, Vec<u8>, Vec<u8>),
) {
    assert_eq!(c.0, rust.0, "{context}: return/memory result");
    assert_eq!(c.1, rust.1, "{context}: stdout");
    assert_eq!(c.2, rust.2, "{context}: stderr");
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

    fn range(&mut self, low: i32, high: i32) -> i32 {
        assert!(low < high);
        low + (self.next_u32() % ((high - low) as u32)) as i32
    }

    fn nonzero_range(&mut self, low: i32, high: i32) -> i32 {
        loop {
            let value = self.range(low, high);
            if value != 0 {
                return value;
            }
        }
    }
}

fn flags(verbose: bool, debug: bool, optimize: bool, cache: bool, log: u32) -> ConfigFlags {
    ConfigFlags {
        bits: u32::from(verbose)
            | (u32::from(debug) << 1)
            | (u32::from(optimize) << 2)
            | (u32::from(cache) << 3)
            | ((log & 7) << 4),
    }
}

fn compare_parse(
    c: &Api,
    rust: &Api,
    env_name: &CString,
    default: i32,
    context: &str,
) -> (i32, Vec<u8>, Vec<u8>) {
    let c_result = capture_io(|| unsafe { (c.parse_env_numeric)(env_name.as_ptr(), default) });
    let rust_result =
        capture_io(|| unsafe { (rust.parse_env_numeric)(env_name.as_ptr(), default) });
    assert_observation_eq(context, c_result.clone(), rust_result);
    c_result
}

fn test_parse_valid(c: &Api, rust: &Api, rng: &mut Rng) {
    let name = CString::new("ENVY_DIFFERENTIAL_VALUE").unwrap();
    set_env("ENVY_DIFFERENTIAL_VALUE", None);
    for case in 0..64 {
        let default = rng.range(-1_000_000, 1_000_001);
        let observed = compare_parse(
            c,
            rust,
            &name,
            default,
            &format!("CONFIGS row 1 case {case}"),
        );
        assert_eq!(observed.0, default);
        assert!(observed.1.is_empty() && observed.2.is_empty());
    }

    let fixed = [
        "0",
        "1",
        "-1",
        "+17",
        "  42",
        "\t-99tail",
        "",
        "words",
        "2147483647",
        "-2147483648",
    ];
    for (case, value) in fixed.iter().enumerate() {
        set_env("ENVY_DIFFERENTIAL_VALUE", Some(value));
        let observed = compare_parse(
            c,
            rust,
            &name,
            12345,
            &format!("CONFIGS row 2 fixed {case}"),
        );
        assert!(observed.1.is_empty() && observed.2.is_empty());
    }
    for case in 0..128 {
        let value = rng.range(-1_000_000, 1_000_001).to_string();
        set_env("ENVY_DIFFERENTIAL_VALUE", Some(&value));
        compare_parse(c, rust, &name, -7, &format!("CONFIGS row 2 random {case}"));
    }
}

fn env_state(state: usize) -> Option<&'static str> {
    match state {
        0 => None,
        1 => Some("enabled"),
        2 => Some("x1x"),
        _ => unreachable!(),
    }
}

fn test_init_config(c: &Api, rust: &Api, rng: &mut Rng) {
    let mut row = 3;
    for verbose_state in 0..3 {
        for debug_state in 0..3 {
            for optimize_present in [false, true] {
                set_env("PROG_VERBOSE", env_state(verbose_state));
                set_env("PROG_DEBUG", env_state(debug_state));
                set_env(
                    "PROG_OPTIMIZE",
                    optimize_present.then_some("present-even-without-1"),
                );
                for case in 0..32 {
                    let initial = rng.next_u32();
                    let mut c_flags = ConfigFlags { bits: initial };
                    let mut rust_flags = c_flags;
                    let c_result = capture_io(|| unsafe {
                        (c.init_config_from_env)(&mut c_flags);
                        c_flags.bits
                    });
                    let rust_result = capture_io(|| unsafe {
                        (rust.init_config_from_env)(&mut rust_flags);
                        rust_flags.bits
                    });
                    assert_observation_eq(
                        &format!("CONFIGS row {row} case {case}"),
                        c_result,
                        rust_result,
                    );
                }
                row += 1;
            }
        }
    }
    assert_eq!(row, 21);
}

fn test_perform_operation(c: &Api, rust: &Api, rng: &mut Rng) {
    for optimize in [false, true] {
        for debug in [false, true] {
            let row = 21 + usize::from(optimize) * 2 + usize::from(debug);
            for case in 0..256 {
                let val1 = rng.range(-100_000, 100_001);
                let val2 = rng.range(-100_000, 100_001);
                let log = rng.next_u32() & 7;
                let high_bits = rng.next_u32() & 0xffff_ff00;
                let mut c_flags = flags(false, debug, optimize, false, log);
                c_flags.bits |= high_bits;
                let mut rust_flags = c_flags;
                let c_result =
                    capture_io(|| unsafe { (c.perform_operation)(val1, val2, &mut c_flags) });
                let rust_result =
                    capture_io(|| unsafe { (rust.perform_operation)(val1, val2, &mut rust_flags) });
                assert_observation_eq(
                    &format!("CONFIGS row {row} case {case}"),
                    c_result,
                    rust_result,
                );
            }
        }
    }
}

fn test_apply_bit_operations(c: &Api, rust: &Api, rng: &mut Rng) {
    for verbose in [false, true] {
        for cache in [false, true] {
            let row = 25 + usize::from(verbose) * 2 + usize::from(cache);
            for case in 0..512 {
                let value = if verbose {
                    rng.range(i32::MIN / 2, i32::MAX / 2)
                } else {
                    rng.next_u32() as i32
                };
                let mut c_flags = flags(verbose, false, false, cache, rng.next_u32() & 7);
                c_flags.bits |= rng.next_u32() & 0xffff_ff00;
                let mut rust_flags = c_flags;
                let c_result =
                    capture_io(|| unsafe { (c.apply_bit_operations)(value, &mut c_flags) });
                let rust_result =
                    capture_io(|| unsafe { (rust.apply_bit_operations)(value, &mut rust_flags) });
                assert_observation_eq(
                    &format!("CONFIGS row {row} case {case}"),
                    c_result,
                    rust_result,
                );
            }
        }
    }
}

fn modeled_pre_restore(
    p1: i32,
    p2: i32,
    p3: i32,
    p4: i32,
    verbose: bool,
    optimize: bool,
    base: i32,
    multiplier: i32,
) -> i32 {
    let mut result = if optimize {
        p1.wrapping_add(p2)
    } else {
        p1.wrapping_mul(3).wrapping_add(p2 / 2)
    };
    if p3 != 0 {
        result = result.wrapping_add(p3.wrapping_mul(multiplier));
    }
    if p4 != 0 {
        result = result.wrapping_add(p4 >> 2);
    }
    if verbose {
        result = result.wrapping_shl(1);
    }
    result |= 0x0f;
    result.wrapping_add(base)
}

fn find_envy_case(
    rng: &mut Rng,
    verbose: bool,
    optimize: bool,
    p3_nonzero: bool,
    p4_nonzero: bool,
    negative: bool,
    base: i32,
    multiplier: i32,
) -> (i32, i32, i32, i32) {
    for _ in 0..100_000 {
        let p1 = rng.range(-20_000, 20_001);
        let p2 = rng.range(-20_000, 20_001);
        let p3 = if p3_nonzero {
            rng.nonzero_range(-2_000, 2_001)
        } else {
            0
        };
        let p4 = if p4_nonzero {
            rng.nonzero_range(-20_000, 20_001)
        } else {
            0
        };
        let result = modeled_pre_restore(p1, p2, p3, p4, verbose, optimize, base, multiplier);
        if (result < 0) == negative {
            return (p1, p2, p3, p4);
        }
    }
    panic!("could not construct requested envy sign");
}

fn set_flag_env(verbose: bool, debug: bool, optimize: bool, variant: usize) {
    set_env(
        "PROG_VERBOSE",
        if verbose {
            Some(if variant % 2 == 0 { "1" } else { "v1v" })
        } else if variant % 2 == 0 {
            None
        } else {
            Some("verbose")
        },
    );
    set_env(
        "PROG_DEBUG",
        if debug {
            Some(if variant % 2 == 0 { "1" } else { "d1d" })
        } else if variant % 2 == 0 {
            None
        } else {
            Some("debug")
        },
    );
    set_env("PROG_OPTIMIZE", optimize.then_some(""));
}

fn test_envy(c: &Api, rust: &Api, rng: &mut Rng) {
    let mut row = 29;
    for verbose in [false, true] {
        for debug in [false, true] {
            for optimize in [false, true] {
                for p3_nonzero in [false, true] {
                    for p4_nonzero in [false, true] {
                        for negative in [false, true] {
                            for env_mode in 0..4 {
                                let base_set = env_mode & 1 != 0;
                                let multiplier_set = env_mode & 2 != 0;
                                for case in 0..8 {
                                    set_flag_env(verbose, debug, optimize, case);
                                    let base = if base_set {
                                        rng.range(-128, 129)
                                    } else {
                                        0o100
                                    };
                                    let multiplier = if multiplier_set {
                                        rng.nonzero_range(-20, 21)
                                    } else {
                                        0o12
                                    };
                                    let base_text = base.to_string();
                                    let multiplier_text = multiplier.to_string();
                                    set_env(
                                        "PROG_BASE_OFFSET",
                                        base_set.then_some(base_text.as_str()),
                                    );
                                    set_env(
                                        "PROG_MULTIPLIER",
                                        multiplier_set.then_some(multiplier_text.as_str()),
                                    );
                                    let (p1, p2, p3, p4) = find_envy_case(
                                        rng, verbose, optimize, p3_nonzero, p4_nonzero, negative,
                                        base, multiplier,
                                    );
                                    let c_result =
                                        capture_io(|| unsafe { (c.envy)(p1, p2, p3, p4) });
                                    let rust_result =
                                        capture_io(|| unsafe { (rust.envy)(p1, p2, p3, p4) });
                                    assert_observation_eq(
                                        &format!(
                                            "CONFIGS row {row}, env mode {env_mode}, case {case}"
                                        ),
                                        c_result,
                                        rust_result,
                                    );
                                }
                            }
                            row += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(row, 93);
}

fn test_error_rows(c: &Api, rust: &Api) {
    let name = CString::new("ENVY_DIFFERENTIAL_VALUE").unwrap();

    set_env("ENVY_DIFFERENTIAL_VALUE", None);
    let observed = compare_parse(c, rust, &name, -4567, "ERRORS row 1");
    assert_eq!(observed.0, -4567);
    assert!(observed.1.is_empty() && observed.2.is_empty());

    for value in ["1,2", ",", "9,8;7"] {
        set_env("ENVY_DIFFERENTIAL_VALUE", Some(value));
        let observed = compare_parse(c, rust, &name, 314, "ERRORS row 2");
        assert_eq!(observed.0, 314);
        assert_eq!(
            observed.2,
            b"Warning: Invalid character in ENVY_DIFFERENTIAL_VALUE\n"
        );
    }

    for value in ["1;2", ";", "abc;def"] {
        set_env("ENVY_DIFFERENTIAL_VALUE", Some(value));
        let observed = compare_parse(c, rust, &name, -271, "ERRORS row 3");
        assert_eq!(observed.0, -271);
        assert_eq!(
            observed.2,
            b"Warning: Semicolon found in ENVY_DIFFERENTIAL_VALUE\n"
        );
    }
}

fn null_probe_status(library_kind: &str, function: &str) -> (Option<i32>, Option<i32>) {
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("null_probe_child")
        .arg("--nocapture")
        .env("ENVY_NULL_PROBE_LIBRARY", library_kind)
        .env("ENVY_NULL_PROBE_FUNCTION", function)
        .status()
        .unwrap();
    (status.code(), status.signal())
}

fn test_null_boundaries() {
    for function in ["parse", "init", "perform", "apply"] {
        let c = null_probe_status("c", function);
        let rust = null_probe_status("rust", function);
        assert_eq!(c, rust, "null boundary differs for {function}");
        assert!(
            c.0 != Some(0),
            "null boundary unexpectedly succeeded for {function}"
        );
    }
}

#[test]
fn differential_surface() {
    clear_program_env();
    let (c_path, rust_path) = paths();
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(rust_path.is_file(), "missing {}", rust_path.display());
    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    let mut rng = Rng::new(0x5eed_c0de_d15c_a11e);

    test_parse_valid(&c, &rust, &mut rng);
    test_init_config(&c, &rust, &mut rng);
    test_perform_operation(&c, &rust, &mut rng);
    test_apply_bit_operations(&c, &rust, &mut rng);
    test_envy(&c, &rust, &mut rng);
    test_error_rows(&c, &rust);
    test_null_boundaries();
    clear_program_env();
}

#[test]
fn null_probe_child() {
    let Ok(library_kind) = std::env::var("ENVY_NULL_PROBE_LIBRARY") else {
        return;
    };
    let function = std::env::var("ENVY_NULL_PROBE_FUNCTION").unwrap();
    let (c_path, rust_path) = paths();
    let path = if library_kind == "c" {
        c_path
    } else {
        rust_path
    };
    let api = unsafe { Api::load(&path) };

    unsafe {
        match function.as_str() {
            "parse" => {
                (api.parse_env_numeric)(ptr::null(), 1);
            }
            "init" => {
                (api.init_config_from_env)(ptr::null_mut());
            }
            "perform" => {
                (api.perform_operation)(1, 2, ptr::null_mut());
            }
            "apply" => {
                (api.apply_bit_operations)(1, ptr::null_mut());
            }
            _ => panic!("unknown null probe"),
        }
    }
}
