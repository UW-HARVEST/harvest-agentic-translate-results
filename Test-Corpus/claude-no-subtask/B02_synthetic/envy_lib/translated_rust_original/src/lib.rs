// Translated from c_src/src/lib.c

use std::ffi::{c_char, c_int, CStr, CString};

#[derive(Clone, Copy)]
struct ConfigFlags {
    verbose: u32,       // 1 bit
    debug: u32,         // 1 bit
    optimize: u32,      // 1 bit
    cache_enabled: u32, // 1 bit
    log_level: u32,     // 3 bits
    #[allow(dead_code)]
    reserved: u32,      // 1 bit
}

impl ConfigFlags {
    fn new() -> Self {
        ConfigFlags {
            verbose: 0,
            debug: 0,
            optimize: 0,
            cache_enabled: 0,
            log_level: 0,
            reserved: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    #[allow(dead_code)]
    operation: c_char,
}

const BUFFER_SIZE: usize = 256;

/// Calls libc::getenv and returns Some(&CStr) if the variable is set.
/// SAFETY: Caller must not hold the reference across other env modifications.
fn getenv_cstr(name: &str) -> Option<&'static CStr> {
    let cname = CString::new(name).ok()?;
    unsafe {
        let ptr = libc::getenv(cname.as_ptr());
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr))
        }
    }
}

/// Returns true if the C string contains the byte `ch`.
fn cstr_contains_char(s: &CStr, ch: u8) -> bool {
    s.to_bytes().iter().any(|&b| b == ch)
}

/// Mimics C's atoi: skip whitespace, optional sign, then digits.
fn c_atoi(s: &CStr) -> c_int {
    unsafe { libc::atoi(s.as_ptr()) }
}

fn parse_env_numeric(env_name: &str, default_val: c_int) -> c_int {
    let env_value = match getenv_cstr(env_name) {
        Some(v) => v,
        None => return default_val,
    };

    if cstr_contains_char(env_value, b',') {
        eprintln!("Warning: Invalid character in {}", env_name);
        return default_val;
    }

    if cstr_contains_char(env_value, b';') {
        eprintln!("Warning: Semicolon found in {}", env_name);
        return default_val;
    }

    c_atoi(env_value)
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = getenv_cstr("PROG_VERBOSE");
    let debug_env = getenv_cstr("PROG_DEBUG");
    let optimize_env = getenv_cstr("PROG_OPTIMIZE");

    flags.verbose = match verbose_env {
        Some(s) if cstr_contains_char(s, b'1') => 1,
        _ => 0,
    };
    flags.debug = match debug_env {
        Some(s) if cstr_contains_char(s, b'1') => 1,
        _ => 0,
    };
    flags.optimize = if optimize_env.is_some() { 1 } else { 0 };
    flags.cache_enabled = 1;
    flags.log_level = 0o3 & 0x7; // 3-bit field
    flags.reserved = 0;
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let result: c_int;

    let operation_mode: c_int = 0o755;

    if flags.optimize != 0 {
        result = val1.wrapping_add(val2);
    } else {
        // (val1 * flags->log_level) + (val2 / 2)
        let log_level = flags.log_level as c_int;
        result = val1
            .wrapping_mul(log_level)
            .wrapping_add(val2 / 2);
    }

    if flags.debug != 0 {
        println!("Debug: operation_mode = {:o} (octal)", operation_mode);
        println!("Debug: result before adjustment = {}", result);
    }

    result
}

fn apply_bit_operations(value: c_int, flags: &ConfigFlags) -> c_int {
    let mut adjusted = value;

    if flags.verbose != 0 {
        adjusted = adjusted.wrapping_shl(1);
    }

    if flags.cache_enabled != 0 {
        adjusted |= 0x0F;
    }

    adjusted
}

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState {
        flags: ConfigFlags::new(),
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };
    let state_backup: ProcessState;
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut result: c_int;

    init_config_from_env(&mut state.flags);

    let base_offset = parse_env_numeric("PROG_BASE_OFFSET", 0o100);
    let multiplier = parse_env_numeric("PROG_MULTIPLIER", 0o12);

    if state.flags.verbose != 0 {
        println!("Verbose mode enabled");
        println!("Base offset: {} (from octal 0100)", base_offset);
        println!("Multiplier: {} (from octal 012)", multiplier);
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    // memcpy state_backup from state
    state_backup = state;

    if state.flags.debug != 0 {
        println!("Debug: Created state backup using memcpy");
        println!("Debug: Backup base_value = {}", state_backup.base_value);
    }

    result = perform_operation(param1, param2, &state.flags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        // C: param4 >> 2  (signed right shift, arithmetic)
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &state.flags);

    result = result.wrapping_add(base_offset);

    // snprintf(buffer, BUFFER_SIZE, "Result:%d:Complete", result);
    let formatted = format!("Result:{}:Complete", result);
    let bytes = formatted.as_bytes();
    let copy_len = bytes.len().min(BUFFER_SIZE - 1);
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
    buffer[copy_len] = 0;

    // strchr(buffer, ':')
    let colon_pos = buffer.iter().position(|&b| b == b':');
    if let Some(first) = colon_pos {
        if state.flags.verbose != 0 {
            println!("Found colon at position: {}", first);
        }

        // strchr(colon_pos + 1, ':')
        let second_colon = buffer[first + 1..].iter().position(|&b| b == b':');
        if second_colon.is_some() && state.flags.debug != 0 {
            println!("Debug: Result string format validated");
        }
    }

    if result < 0 {
        // memcpy(&state, &state_backup, sizeof(struct ProcessState));
        state = state_backup;
        result = state.base_value; // Use original base value

        if state.flags.verbose != 0 {
            println!("Restored state from backup");
        }
    }

    if state.flags.verbose != 0 {
        println!("Final result: {}", result);
        println!(
            "Configuration - Debug: {}, Optimize: {}, Log Level: {}",
            state.flags.debug, state.flags.optimize, state.flags.log_level
        );
    }

    result
}
