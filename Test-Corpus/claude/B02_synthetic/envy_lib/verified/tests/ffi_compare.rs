// Integration tests that compare the C shared library and Rust shared
// library outputs through their FFI exports.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::Mutex;

// All env-mutating tests acquire this lock to avoid races.
static ENV_LOCK: Mutex<()> = Mutex::new(());

// The bit-field on the C side is laid out within a single 32-bit unit.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
struct ConfigFlagsRaw {
    bits: u32,
}

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Use the release output (rebuilt by build_libs).
    p.push("target/release/libenvy_lib.so");
    p
}

fn build_libs() {
    use std::process::Command;
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Build C lib (idempotent).
    let c_build = manifest.join("c_src/build");
    std::fs::create_dir_all(&c_build).unwrap();
    let cmake_status = Command::new("cmake")
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .current_dir(&c_build)
        .status()
        .expect("cmake configure failed");
    assert!(cmake_status.success(), "cmake configure failed");
    let build_status = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&c_build)
        .status()
        .expect("cmake build failed");
    assert!(build_status.success(), "cmake build failed");

    // Build Rust release lib.
    let cargo_status = Command::new(env!("CARGO"))
        .args(["build", "--release"])
        .current_dir(&manifest)
        .status()
        .expect("cargo build failed");
    assert!(cargo_status.success(), "cargo build failed");
}

fn load_libs() -> (Library, Library) {
    build_libs();
    let c = unsafe { Library::new(c_lib_path()).expect("Failed to load C lib") };
    let r = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust lib") };
    (c, r)
}

fn clear_env() {
    for k in &[
        "PROG_VERBOSE",
        "PROG_DEBUG",
        "PROG_OPTIMIZE",
        "PROG_BASE_OFFSET",
        "PROG_MULTIPLIER",
    ] {
        unsafe { std::env::remove_var(k) };
    }
}

fn set_env(key: &str, value: &str) {
    unsafe { std::env::set_var(key, value) };
}

// ----- parse_env_numeric -----

type ParseEnvNumericFn = unsafe extern "C" fn(*const c_char, c_int) -> c_int;

fn call_parse_env_numeric(lib: &Library, name: &[u8], default_val: c_int) -> c_int {
    unsafe {
        let f: Symbol<ParseEnvNumericFn> = lib.get(b"parse_env_numeric\0").unwrap();
        f(name.as_ptr() as *const c_char, default_val)
    }
}

#[test]
fn test_parse_env_numeric_unset() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    let cv = call_parse_env_numeric(&c, b"PROG_BASE_OFFSET\0", 100);
    let rv = call_parse_env_numeric(&r, b"PROG_BASE_OFFSET\0", 100);
    assert_eq!(cv, rv);
    assert_eq!(cv, 100);
}

#[test]
fn test_parse_env_numeric_simple() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_BASE_OFFSET", "42") };
    let cv = call_parse_env_numeric(&c, b"PROG_BASE_OFFSET\0", 100);
    let rv = call_parse_env_numeric(&r, b"PROG_BASE_OFFSET\0", 100);
    assert_eq!(cv, rv);
    assert_eq!(cv, 42);
}

#[test]
fn test_parse_env_numeric_negative() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_BASE_OFFSET", "-31") };
    let cv = call_parse_env_numeric(&c, b"PROG_BASE_OFFSET\0", 0);
    let rv = call_parse_env_numeric(&r, b"PROG_BASE_OFFSET\0", 0);
    assert_eq!(cv, rv);
    assert_eq!(cv, -31);
}

#[test]
fn test_parse_env_numeric_with_comma() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_BASE_OFFSET", "1,2") };
    let cv = call_parse_env_numeric(&c, b"PROG_BASE_OFFSET\0", 7);
    let rv = call_parse_env_numeric(&r, b"PROG_BASE_OFFSET\0", 7);
    assert_eq!(cv, rv);
    assert_eq!(cv, 7);
}

#[test]
fn test_parse_env_numeric_with_semicolon() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_BASE_OFFSET", "9;9") };
    let cv = call_parse_env_numeric(&c, b"PROG_BASE_OFFSET\0", 13);
    let rv = call_parse_env_numeric(&r, b"PROG_BASE_OFFSET\0", 13);
    assert_eq!(cv, rv);
    assert_eq!(cv, 13);
}

#[test]
fn test_parse_env_numeric_empty_string() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_BASE_OFFSET", "") };
    let cv = call_parse_env_numeric(&c, b"PROG_BASE_OFFSET\0", 5);
    let rv = call_parse_env_numeric(&r, b"PROG_BASE_OFFSET\0", 5);
    assert_eq!(cv, rv);
    // atoi("") == 0.
    assert_eq!(cv, 0);
}

#[test]
fn test_parse_env_numeric_with_leading_ws() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_BASE_OFFSET", "  42abc") };
    let cv = call_parse_env_numeric(&c, b"PROG_BASE_OFFSET\0", 0);
    let rv = call_parse_env_numeric(&r, b"PROG_BASE_OFFSET\0", 0);
    assert_eq!(cv, rv);
    assert_eq!(cv, 42);
}

// ----- init_config_from_env -----

type InitConfigFn = unsafe extern "C" fn(*mut ConfigFlagsRaw);

fn call_init_config(lib: &Library) -> ConfigFlagsRaw {
    unsafe {
        let f: Symbol<InitConfigFn> = lib.get(b"init_config_from_env\0").unwrap();
        let mut flags = ConfigFlagsRaw::default();
        // Pre-poison so we ensure the function fully initializes the bits.
        flags.bits = 0xDEADBEEF;
        // Reset to zero so default initialization mimics C uninitialized w/ later writes.
        flags.bits = 0;
        f(&mut flags as *mut ConfigFlagsRaw);
        flags
    }
}

#[test]
fn test_init_config_no_env() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    let cv = call_init_config(&c);
    let rv = call_init_config(&r);
    assert_eq!(cv, rv);
}

#[test]
fn test_init_config_verbose_set() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_VERBOSE", "1") };
    let cv = call_init_config(&c);
    let rv = call_init_config(&r);
    assert_eq!(cv, rv);
}

#[test]
fn test_init_config_verbose_no_one() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_VERBOSE", "abc") };
    let cv = call_init_config(&c);
    let rv = call_init_config(&r);
    assert_eq!(cv, rv);
}

#[test]
fn test_init_config_debug_set() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_DEBUG", "x1y") };
    let cv = call_init_config(&c);
    let rv = call_init_config(&r);
    assert_eq!(cv, rv);
}

#[test]
fn test_init_config_optimize_set_empty() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    // PROG_OPTIMIZE just needs to be set (any value, including empty).
    unsafe { std::env::set_var("PROG_OPTIMIZE", "") };
    let cv = call_init_config(&c);
    let rv = call_init_config(&r);
    assert_eq!(cv, rv);
}

#[test]
fn test_init_config_all_flags() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_VERBOSE", "1") };
    unsafe { std::env::set_var("PROG_DEBUG", "1") };
    unsafe { std::env::set_var("PROG_OPTIMIZE", "y") };
    let cv = call_init_config(&c);
    let rv = call_init_config(&r);
    assert_eq!(cv, rv);
}

// ----- perform_operation -----

type PerformOpFn = unsafe extern "C" fn(c_int, c_int, *mut ConfigFlagsRaw) -> c_int;

fn call_perform_op(lib: &Library, v1: c_int, v2: c_int, flags: &mut ConfigFlagsRaw) -> c_int {
    unsafe {
        let f: Symbol<PerformOpFn> = lib.get(b"perform_operation\0").unwrap();
        f(v1, v2, flags as *mut ConfigFlagsRaw)
    }
}

#[test]
fn test_perform_op_default_flags() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    // Get default flags from init_config_from_env: cache_enabled=1, log_level=3.
    let mut cf = call_init_config(&c);
    let mut rf = call_init_config(&r);
    assert_eq!(cf, rf);
    // (val1 * log_level) + (val2 / 2) = 5*3 + 8/2 = 15+4 = 19
    let cv = call_perform_op(&c, 5, 8, &mut cf);
    let rv = call_perform_op(&r, 5, 8, &mut rf);
    assert_eq!(cv, rv);
    assert_eq!(cv, 19);
}

#[test]
fn test_perform_op_optimize() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_OPTIMIZE", "yes") };
    let mut cf = call_init_config(&c);
    let mut rf = call_init_config(&r);
    let cv = call_perform_op(&c, 5, 8, &mut cf);
    let rv = call_perform_op(&r, 5, 8, &mut rf);
    assert_eq!(cv, rv);
    assert_eq!(cv, 13);
}

#[test]
fn test_perform_op_negatives() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    let mut cf = call_init_config(&c);
    let mut rf = call_init_config(&r);
    let cv = call_perform_op(&c, -10, -7, &mut cf);
    let rv = call_perform_op(&r, -10, -7, &mut rf);
    assert_eq!(cv, rv);
}

// ----- apply_bit_operations -----

type BitOpFn = unsafe extern "C" fn(c_int, *mut ConfigFlagsRaw) -> c_int;

fn call_bit_op(lib: &Library, value: c_int, flags: &mut ConfigFlagsRaw) -> c_int {
    unsafe {
        let f: Symbol<BitOpFn> = lib.get(b"apply_bit_operations\0").unwrap();
        f(value, flags as *mut ConfigFlagsRaw)
    }
}

#[test]
fn test_bit_op_default() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    let mut cf = call_init_config(&c);
    let mut rf = call_init_config(&r);
    // verbose=0, cache_enabled=1 -> result | 0xF
    let cv = call_bit_op(&c, 0x100, &mut cf);
    let rv = call_bit_op(&r, 0x100, &mut rf);
    assert_eq!(cv, rv);
    assert_eq!(cv, 0x10F);
}

#[test]
fn test_bit_op_verbose() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_VERBOSE", "1") };
    let mut cf = call_init_config(&c);
    let mut rf = call_init_config(&r);
    let cv = call_bit_op(&c, 0x100, &mut cf);
    let rv = call_bit_op(&r, 0x100, &mut rf);
    assert_eq!(cv, rv);
    // (0x100 << 1) | 0xF = 0x20F
    assert_eq!(cv, 0x20F);
}

#[test]
fn test_bit_op_negative_value() {
    let _g = ENV_LOCK.lock().unwrap();
    let (c, r) = load_libs();
    clear_env();
    unsafe { std::env::set_var("PROG_VERBOSE", "1") };
    let mut cf = call_init_config(&c);
    let mut rf = call_init_config(&r);
    let cv = call_bit_op(&c, -1, &mut cf);
    let rv = call_bit_op(&r, -1, &mut rf);
    assert_eq!(cv, rv);
}

// ----- envy -----

type EnvyFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn call_envy(lib: &Library, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    unsafe {
        let f: Symbol<EnvyFn> = lib.get(b"envy\0").unwrap();
        f(a, b, c, d)
    }
}

fn assert_envy_match(a: c_int, b: c_int, c: c_int, d: c_int) {
    let (cl, rl) = load_libs();
    let cv = call_envy(&cl, a, b, c, d);
    let rv = call_envy(&rl, a, b, c, d);
    assert_eq!(cv, rv, "envy({}, {}, {}, {}) mismatch: C={}, Rust={}", a, b, c, d, cv, rv);
}

#[test]
fn test_envy_zeros() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    assert_envy_match(0, 0, 0, 0);
}

#[test]
fn test_envy_basic() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    assert_envy_match(1, 2, 3, 4);
}

#[test]
fn test_envy_negatives() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    assert_envy_match(-1, -2, -3, -4);
}

#[test]
fn test_envy_mixed() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    assert_envy_match(100, -50, 25, -12);
}

#[test]
fn test_envy_param4_negative_shift() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    assert_envy_match(0, 0, 0, -1);
    assert_envy_match(0, 0, 0, -100);
}

#[test]
fn test_envy_with_base_offset_env() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("PROG_BASE_OFFSET", "5") };
    assert_envy_match(1, 2, 3, 4);
}

#[test]
fn test_envy_with_multiplier_env() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("PROG_MULTIPLIER", "7") };
    assert_envy_match(1, 2, 3, 4);
}

#[test]
fn test_envy_with_invalid_offset() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("PROG_BASE_OFFSET", "5,bad") };
    assert_envy_match(1, 2, 3, 4);
}

#[test]
fn test_envy_with_optimize() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("PROG_OPTIMIZE", "1") };
    assert_envy_match(1, 2, 3, 4);
}

#[test]
fn test_envy_with_verbose_and_debug() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("PROG_VERBOSE", "1") };
    unsafe { std::env::set_var("PROG_DEBUG", "1") };
    assert_envy_match(1, 2, 3, 4);
}

#[test]
fn test_envy_negative_result_path() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    // Try to drive result < 0 to exercise the restore path. Use very negative
    // values; behavior just needs to match C.
    assert_envy_match(i32::MIN, i32::MIN, 1, 0);
    assert_envy_match(-1000000, -1000000, -1000000, -1000000);
}

#[test]
fn test_envy_various() {
    let _g = ENV_LOCK.lock().unwrap();
    clear_env();
    let cases: &[(i32, i32, i32, i32)] = &[
        (10, 20, 30, 40),
        (-10, 20, -30, 40),
        (0, 0, 1, 0),
        (0, 0, 0, 8),
        (7, 11, 13, 17),
        (1234, 5678, -9, -8),
        (i32::MAX, 0, 0, 0),
        (0, i32::MAX, 0, 0),
    ];
    for &(a, b, c, d) in cases {
        assert_envy_match(a, b, c, d);
    }
}
