use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_char;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");

fn clear_env() {
    for var in ["PROG_VERBOSE", "PROG_DEBUG", "PROG_OPTIMIZE", "PROG_BASE_OFFSET", "PROG_MULTIPLIER"] {
        unsafe { libc::unsetenv(format!("{}\0", var).as_ptr() as *const c_char) };
    }
}

fn setenv(name: &str, val: &str) {
    unsafe {
        libc::setenv(
            format!("{}\0", name).as_ptr() as *const c_char,
            format!("{}\0", val).as_ptr() as *const c_char,
            1,
        );
    }
}

// ---- parse_env_numeric ----

#[test]
fn test_parse_env_numeric_default() {
    clear_env();
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
        unsafe { c_lib.get(b"parse_env_numeric").unwrap() };

    let name = b"PROG_BASE_OFFSET\0";
    let c_result = unsafe { c_fn(name.as_ptr() as *const c_char, 64) };
    let rust_result = envy_lib::parse_env_numeric_wrapper(name, 64);
    assert_eq!(c_result, rust_result, "parse_env_numeric default: C={} Rust={}", c_result, rust_result);
}

#[test]
fn test_parse_env_numeric_set() {
    clear_env();
    setenv("PROG_BASE_OFFSET", "42");
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
        unsafe { c_lib.get(b"parse_env_numeric").unwrap() };

    let name = b"PROG_BASE_OFFSET\0";
    let c_result = unsafe { c_fn(name.as_ptr() as *const c_char, 64) };
    let rust_result = envy_lib::parse_env_numeric_wrapper(name, 64);
    assert_eq!(c_result, rust_result, "parse_env_numeric set: C={} Rust={}", c_result, rust_result);
    clear_env();
}

#[test]
fn test_parse_env_numeric_comma() {
    clear_env();
    setenv("PROG_BASE_OFFSET", "4,2");
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
        unsafe { c_lib.get(b"parse_env_numeric").unwrap() };

    let name = b"PROG_BASE_OFFSET\0";
    let c_result = unsafe { c_fn(name.as_ptr() as *const c_char, 64) };
    let rust_result = envy_lib::parse_env_numeric_wrapper(name, 64);
    assert_eq!(c_result, rust_result, "parse_env_numeric comma: C={} Rust={}", c_result, rust_result);
    clear_env();
}

// ---- init_config_from_env ----

#[test]
fn test_init_config_from_env_defaults() {
    clear_env();
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
        unsafe { c_lib.get(b"init_config_from_env").unwrap() };

    let mut c_flags: u32 = 0;
    unsafe { c_fn(&mut c_flags) };

    let mut rust_flags = envy_lib::CConfigFlags { bits: 0 };
    envy_lib::init_config_from_env(&mut rust_flags);

    assert_eq!(c_flags, rust_flags.bits, "init_config_from_env defaults: C=0x{:x} Rust=0x{:x}", c_flags, rust_flags.bits);
}

#[test]
fn test_init_config_from_env_verbose_debug() {
    clear_env();
    setenv("PROG_VERBOSE", "1");
    setenv("PROG_DEBUG", "1");
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
        unsafe { c_lib.get(b"init_config_from_env").unwrap() };

    let mut c_flags: u32 = 0;
    unsafe { c_fn(&mut c_flags) };

    let mut rust_flags = envy_lib::CConfigFlags { bits: 0 };
    envy_lib::init_config_from_env(&mut rust_flags);

    assert_eq!(c_flags, rust_flags.bits, "init_config_from_env v+d: C=0x{:x} Rust=0x{:x}", c_flags, rust_flags.bits);
    clear_env();
}

#[test]
fn test_init_config_from_env_optimize() {
    clear_env();
    setenv("PROG_OPTIMIZE", "yes");
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
        unsafe { c_lib.get(b"init_config_from_env").unwrap() };

    let mut c_flags: u32 = 0;
    unsafe { c_fn(&mut c_flags) };

    let mut rust_flags = envy_lib::CConfigFlags { bits: 0 };
    envy_lib::init_config_from_env(&mut rust_flags);

    assert_eq!(c_flags, rust_flags.bits, "init_config_from_env optimize: C=0x{:x} Rust=0x{:x}", c_flags, rust_flags.bits);
    clear_env();
}

// ---- perform_operation ----

#[test]
fn test_perform_operation() {
    clear_env();
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *const u32) -> c_int> =
        unsafe { c_lib.get(b"perform_operation").unwrap() };

    // flags: cache_enabled=1, log_level=3, no optimize => bits = 0x38
    let flags_no_opt: u32 = 0x38;
    // flags: cache_enabled=1, log_level=3, optimize=1 => bits = 0x3C
    let flags_opt: u32 = 0x3C;

    let cases: &[(c_int, c_int, u32)] = &[
        (0, 0, flags_no_opt),
        (1, 2, flags_no_opt),
        (10, 20, flags_no_opt),
        (100, 200, flags_no_opt),
        (0, 0, flags_opt),
        (5, 10, flags_opt),
        (100, 200, flags_opt),
    ];

    for &(v1, v2, flags) in cases {
        let c_result = unsafe { c_fn(v1, v2, &flags) };
        let rust_flags = envy_lib::CConfigFlags { bits: flags };
        let rust_result = envy_lib::perform_operation(v1, v2, &rust_flags);
        assert_eq!(c_result, rust_result, "perform_operation({},{},0x{:x}): C={} Rust={}", v1, v2, flags, c_result, rust_result);
    }
}

// ---- apply_bit_operations ----

#[test]
fn test_apply_bit_operations() {
    clear_env();
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int, *const u32) -> c_int> =
        unsafe { c_lib.get(b"apply_bit_operations").unwrap() };

    // flags: cache_enabled=1, log_level=3 => 0x38
    let flags_base: u32 = 0x38;
    // flags: verbose=1, cache_enabled=1, log_level=3 => 0x39
    let flags_verbose: u32 = 0x39;

    let cases: &[(c_int, u32)] = &[
        (0, flags_base),
        (42, flags_base),
        (255, flags_base),
        (0, flags_verbose),
        (42, flags_verbose),
        (255, flags_verbose),
    ];

    for &(val, flags) in cases {
        let c_result = unsafe { c_fn(val, &flags) };
        let rust_flags = envy_lib::CConfigFlags { bits: flags };
        let rust_result = envy_lib::apply_bit_operations(val, &rust_flags);
        assert_eq!(c_result, rust_result, "apply_bit_operations({},0x{:x}): C={} Rust={}", val, flags, c_result, rust_result);
    }
}

// ---- envy (top-level) ----

#[test]
fn test_envy_defaults() {
    clear_env();
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"envy").unwrap() };

    let inputs: &[(c_int, c_int, c_int, c_int)] = &[
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (10, 20, 0, 0),
        (100, 200, 50, 25),
        (-5, 10, 3, 7),
        (0, 0, 0, 1),
        (1, 0, 0, 0),
    ];

    for &(a, b, c, d) in inputs {
        let c_result = unsafe { c_fn(a, b, c, d) };
        let rust_result = envy_lib::envy(a, b, c, d);
        assert_eq!(c_result, rust_result, "envy({},{},{},{}) defaults: C={} Rust={}", a, b, c, d, c_result, rust_result);
    }
}

#[test]
fn test_envy_verbose() {
    clear_env();
    setenv("PROG_VERBOSE", "1");
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"envy").unwrap() };

    for &(a, b, c, d) in &[(0, 0, 0, 0), (1, 2, 3, 4), (10, 20, 0, 0), (100, 200, 50, 25)] {
        let c_result = unsafe { c_fn(a, b, c, d) };
        let rust_result = envy_lib::envy(a, b, c, d);
        assert_eq!(c_result, rust_result, "envy({},{},{},{}) verbose: C={} Rust={}", a, b, c, d, c_result, rust_result);
    }
    clear_env();
}

#[test]
fn test_envy_optimize() {
    clear_env();
    setenv("PROG_OPTIMIZE", "1");
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"envy").unwrap() };

    for &(a, b, c, d) in &[(5, 10, 3, 7), (100, 200, 50, 25), (0, 0, 0, 0)] {
        let c_result = unsafe { c_fn(a, b, c, d) };
        let rust_result = envy_lib::envy(a, b, c, d);
        assert_eq!(c_result, rust_result, "envy({},{},{},{}) optimize: C={} Rust={}", a, b, c, d, c_result, rust_result);
    }
    clear_env();
}

#[test]
fn test_envy_custom_env() {
    clear_env();
    setenv("PROG_BASE_OFFSET", "100");
    setenv("PROG_MULTIPLIER", "5");
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"envy").unwrap() };

    for &(a, b, c, d) in &[(5, 10, 3, 7), (0, 0, 0, 0)] {
        let c_result = unsafe { c_fn(a, b, c, d) };
        let rust_result = envy_lib::envy(a, b, c, d);
        assert_eq!(c_result, rust_result, "envy({},{},{},{}) custom_env: C={} Rust={}", a, b, c, d, c_result, rust_result);
    }
    clear_env();
}

#[test]
fn test_envy_negative_result() {
    clear_env();
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"envy").unwrap() };

    for &(a, b, c, d) in &[(-1000000, -1000000, -1000000, 0), (i32::MIN / 4, i32::MIN / 4, i32::MIN / 4, 0)] {
        let c_result = unsafe { c_fn(a, b, c, d) };
        let rust_result = envy_lib::envy(a, b, c, d);
        assert_eq!(c_result, rust_result, "envy({},{},{},{}) negative: C={} Rust={}", a, b, c, d, c_result, rust_result);
    }
}
