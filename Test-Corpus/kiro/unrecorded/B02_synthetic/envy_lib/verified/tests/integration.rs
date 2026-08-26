use libloading::{Library, Symbol};
use std::ffi::{c_int, CString};
use std::os::raw::c_char;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/debug/
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    dir.join("libenvy_lib.so")
}

/// Clear all PROG_* env vars so tests are isolated
fn clear_env() {
    for k in &["PROG_VERBOSE", "PROG_DEBUG", "PROG_OPTIMIZE", "PROG_BASE_OFFSET", "PROG_MULTIPLIER"] {
        std::env::remove_var(k);
    }
}

// ---- parse_env_numeric tests ----

#[test]
fn test_parse_env_numeric_default() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            c_lib.get(b"parse_env_numeric").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            r_lib.get(b"parse_env_numeric").unwrap();

        let name = CString::new("NONEXISTENT_VAR_12345").unwrap();
        std::env::remove_var("NONEXISTENT_VAR_12345");

        let c_result = c_fn(name.as_ptr(), 42);
        let r_result = r_fn(name.as_ptr(), 42);
        assert_eq!(c_result, r_result, "parse_env_numeric default mismatch");
    }
}

#[test]
fn test_parse_env_numeric_valid() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            c_lib.get(b"parse_env_numeric").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            r_lib.get(b"parse_env_numeric").unwrap();

        let name = CString::new("TEST_PARSE_NUM").unwrap();
        std::env::set_var("TEST_PARSE_NUM", "123");

        let c_result = c_fn(name.as_ptr(), 0);
        let r_result = r_fn(name.as_ptr(), 0);
        assert_eq!(c_result, r_result, "parse_env_numeric valid value mismatch");

        std::env::remove_var("TEST_PARSE_NUM");
    }
}

#[test]
fn test_parse_env_numeric_comma() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            c_lib.get(b"parse_env_numeric").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            r_lib.get(b"parse_env_numeric").unwrap();

        let name = CString::new("TEST_PARSE_COMMA").unwrap();
        std::env::set_var("TEST_PARSE_COMMA", "1,2");

        let c_result = c_fn(name.as_ptr(), 99);
        let r_result = r_fn(name.as_ptr(), 99);
        assert_eq!(c_result, r_result, "parse_env_numeric comma mismatch");

        std::env::remove_var("TEST_PARSE_COMMA");
    }
}

#[test]
fn test_parse_env_numeric_semicolon() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            c_lib.get(b"parse_env_numeric").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            r_lib.get(b"parse_env_numeric").unwrap();

        let name = CString::new("TEST_PARSE_SEMI").unwrap();
        std::env::set_var("TEST_PARSE_SEMI", "1;2");

        let c_result = c_fn(name.as_ptr(), 77);
        let r_result = r_fn(name.as_ptr(), 77);
        assert_eq!(c_result, r_result, "parse_env_numeric semicolon mismatch");

        std::env::remove_var("TEST_PARSE_SEMI");
    }
}

#[test]
fn test_parse_env_numeric_negative() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            c_lib.get(b"parse_env_numeric").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> c_int> =
            r_lib.get(b"parse_env_numeric").unwrap();

        let name = CString::new("TEST_PARSE_NEG").unwrap();
        std::env::set_var("TEST_PARSE_NEG", "-50");

        let c_result = c_fn(name.as_ptr(), 0);
        let r_result = r_fn(name.as_ptr(), 0);
        assert_eq!(c_result, r_result, "parse_env_numeric negative mismatch");

        std::env::remove_var("TEST_PARSE_NEG");
    }
}

// ---- init_config_from_env tests ----

#[test]
fn test_init_config_from_env_defaults() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
            c_lib.get(b"init_config_from_env").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
            r_lib.get(b"init_config_from_env").unwrap();

        let mut c_flags: u32 = 0;
        let mut r_flags: u32 = 0;

        c_fn(&mut c_flags);
        r_fn(&mut r_flags);

        assert_eq!(c_flags, r_flags, "init_config_from_env defaults mismatch: C={:#010x} Rust={:#010x}", c_flags, r_flags);
    }
}

#[test]
fn test_init_config_from_env_verbose_debug() {
    clear_env();
    std::env::set_var("PROG_VERBOSE", "1");
    std::env::set_var("PROG_DEBUG", "1");
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
            c_lib.get(b"init_config_from_env").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
            r_lib.get(b"init_config_from_env").unwrap();

        let mut c_flags: u32 = 0;
        let mut r_flags: u32 = 0;

        c_fn(&mut c_flags);
        r_fn(&mut r_flags);

        assert_eq!(c_flags, r_flags, "init_config_from_env verbose+debug mismatch: C={:#010x} Rust={:#010x}", c_flags, r_flags);
    }
    clear_env();
}

#[test]
fn test_init_config_from_env_optimize() {
    clear_env();
    std::env::set_var("PROG_OPTIMIZE", "yes");
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
            c_lib.get(b"init_config_from_env").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u32)> =
            r_lib.get(b"init_config_from_env").unwrap();

        let mut c_flags: u32 = 0;
        let mut r_flags: u32 = 0;

        c_fn(&mut c_flags);
        r_fn(&mut r_flags);

        assert_eq!(c_flags, r_flags, "init_config_from_env optimize mismatch: C={:#010x} Rust={:#010x}", c_flags, r_flags);
    }
    clear_env();
}

// ---- perform_operation tests ----

#[test]
fn test_perform_operation_no_optimize() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *mut u32) -> c_int> =
            c_lib.get(b"perform_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *mut u32) -> c_int> =
            r_lib.get(b"perform_operation").unwrap();

        // flags: cache_enabled=1, log_level=3, no optimize
        // bits: cache_enabled(bit3)=0x08, log_level(bits4-6)=3<<4=0x30 => 0x38
        let mut c_flags: u32 = 0x38;
        let mut r_flags: u32 = 0x38;

        let c_result = c_fn(10, 20, &mut c_flags);
        let r_result = r_fn(10, 20, &mut r_flags);
        assert_eq!(c_result, r_result, "perform_operation no_optimize mismatch");
    }
}

#[test]
fn test_perform_operation_optimize() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *mut u32) -> c_int> =
            c_lib.get(b"perform_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *mut u32) -> c_int> =
            r_lib.get(b"perform_operation").unwrap();

        // optimize=1 (bit2=0x04), cache_enabled=1 (bit3=0x08), log_level=3 (0x30)
        let mut c_flags: u32 = 0x3C;
        let mut r_flags: u32 = 0x3C;

        let c_result = c_fn(10, 20, &mut c_flags);
        let r_result = r_fn(10, 20, &mut r_flags);
        assert_eq!(c_result, r_result, "perform_operation optimize mismatch");
    }
}

// ---- apply_bit_operations tests ----

#[test]
fn test_apply_bit_operations_no_verbose() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, *mut u32) -> c_int> =
            c_lib.get(b"apply_bit_operations").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, *mut u32) -> c_int> =
            r_lib.get(b"apply_bit_operations").unwrap();

        // cache_enabled=1 (0x08), log_level=3 (0x30) => 0x38
        let mut c_flags: u32 = 0x38;
        let mut r_flags: u32 = 0x38;

        let c_result = c_fn(100, &mut c_flags);
        let r_result = r_fn(100, &mut r_flags);
        assert_eq!(c_result, r_result, "apply_bit_operations no_verbose mismatch");
    }
}

#[test]
fn test_apply_bit_operations_verbose() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, *mut u32) -> c_int> =
            c_lib.get(b"apply_bit_operations").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, *mut u32) -> c_int> =
            r_lib.get(b"apply_bit_operations").unwrap();

        // verbose=1 (0x01), cache_enabled=1 (0x08), log_level=3 (0x30) => 0x39
        let mut c_flags: u32 = 0x39;
        let mut r_flags: u32 = 0x39;

        let c_result = c_fn(100, &mut c_flags);
        let r_result = r_fn(100, &mut r_flags);
        assert_eq!(c_result, r_result, "apply_bit_operations verbose mismatch");
    }
}

// ---- envy (top-level) tests ----

#[test]
fn test_envy_defaults() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"envy").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"envy").unwrap();

        let c_result = c_fn(10, 20, 5, 8);
        let r_result = r_fn(10, 20, 5, 8);
        assert_eq!(c_result, r_result, "envy defaults mismatch");
    }
}

#[test]
fn test_envy_zeros() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"envy").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"envy").unwrap();

        let c_result = c_fn(0, 0, 0, 0);
        let r_result = r_fn(0, 0, 0, 0);
        assert_eq!(c_result, r_result, "envy zeros mismatch");
    }
}

#[test]
fn test_envy_with_optimize() {
    clear_env();
    std::env::set_var("PROG_OPTIMIZE", "1");
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"envy").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"envy").unwrap();

        let c_result = c_fn(10, 20, 5, 8);
        let r_result = r_fn(10, 20, 5, 8);
        assert_eq!(c_result, r_result, "envy with optimize mismatch");
    }
    clear_env();
}

#[test]
fn test_envy_negative_params() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"envy").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"envy").unwrap();

        let c_result = c_fn(-5, -10, -3, -4);
        let r_result = r_fn(-5, -10, -3, -4);
        assert_eq!(c_result, r_result, "envy negative params mismatch");
    }
}

#[test]
fn test_envy_large_values() {
    clear_env();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"envy").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"envy").unwrap();

        let c_result = c_fn(1000, 2000, 500, 800);
        let r_result = r_fn(1000, 2000, 500, 800);
        assert_eq!(c_result, r_result, "envy large values mismatch");
    }
}

#[test]
fn test_envy_with_env_overrides() {
    clear_env();
    std::env::set_var("PROG_BASE_OFFSET", "100");
    std::env::set_var("PROG_MULTIPLIER", "5");
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"envy").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"envy").unwrap();

        let c_result = c_fn(10, 20, 5, 8);
        let r_result = r_fn(10, 20, 5, 8);
        assert_eq!(c_result, r_result, "envy with env overrides mismatch");
    }
    clear_env();
}
