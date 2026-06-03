// Translation of c_src/src/lib.c to Rust.
//
// Copyright 2025 MIT Lincoln Laboratory
// (See c_src/src/lib.c for the full license header.)

use std::env;
use std::os::raw::c_int;

const BUFFER_SIZE: usize = 256;

#[derive(Clone, Copy, Default)]
struct ConfigFlags {
    verbose: u32,       // 1 bit
    debug: u32,         // 1 bit
    optimize: u32,      // 1 bit
    cache_enabled: u32, // 1 bit
    log_level: u32,     // 3 bits
    reserved: u32,      // 1 bit
}

#[derive(Clone, Copy)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: i32,
    multiplier: i32,
    operation: u8,
}

impl Default for ProcessState {
    fn default() -> Self {
        ProcessState {
            flags: ConfigFlags::default(),
            base_value: 0,
            multiplier: 0,
            operation: 0,
        }
    }
}

fn parse_env_numeric(env_name: &str, default_val: i32) -> i32 {
    let env_value = match env::var(env_name) {
        Ok(v) => v,
        Err(_) => return default_val,
    };

    if env_value.contains(',') {
        eprintln!("Warning: Invalid character in {}", env_name);
        return default_val;
    }

    if env_value.contains(';') {
        eprintln!("Warning: Semicolon found in {}", env_name);
        return default_val;
    }

    // Mimic C's atoi: parse leading optional sign + digits, ignore trailing.
    atoi_like(&env_value)
}

fn atoi_like(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace, like atoi does.
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }

    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }

    let mut result: i32 = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i32;
        result = result.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }

    result.wrapping_mul(sign)
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = env::var("PROG_VERBOSE").ok();
    let debug_env = env::var("PROG_DEBUG").ok();
    let optimize_env = env::var("PROG_OPTIMIZE").ok();

    flags.verbose = match verbose_env {
        Some(ref v) if v.contains('1') => 1,
        _ => 0,
    };
    flags.debug = match debug_env {
        Some(ref v) if v.contains('1') => 1,
        _ => 0,
    };
    flags.optimize = if optimize_env.is_some() { 1 } else { 0 };
    flags.cache_enabled = 1;
    flags.log_level = 0o3;
    flags.reserved = 0;
}

fn perform_operation(val1: i32, val2: i32, flags: &ConfigFlags) -> i32 {
    let result: i32;

    let operation_mode: i32 = 0o755;

    if flags.optimize != 0 {
        result = val1.wrapping_add(val2);
    } else {
        result = val1
            .wrapping_mul(flags.log_level as i32)
            .wrapping_add(val2 / 2);
    }

    if flags.debug != 0 {
        println!("Debug: operation_mode = {:o} (octal)", operation_mode);
        println!("Debug: result before adjustment = {}", result);
    }

    result
}

fn apply_bit_operations(value: i32, flags: &ConfigFlags) -> i32 {
    let mut adjusted = value;

    if flags.verbose != 0 {
        adjusted = adjusted.wrapping_shl(1);
    }

    if flags.cache_enabled != 0 {
        adjusted |= 0x0F;
    }

    adjusted
}

/// Equivalent of `int envy(int a, int b, int c, int d)` from lib.c.
#[no_mangle]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    envy_impl(param1, param2, param3, param4)
}

fn envy_impl(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut state = ProcessState::default();
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut result: i32;

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
    state.operation = b'+';

    // Equivalent to memcpy(&state_backup, &state, sizeof(state)).
    let state_backup: ProcessState = state;

    if state.flags.debug != 0 {
        println!("Debug: Created state backup using memcpy");
        println!("Debug: Backup base_value = {}", state_backup.base_value);
    }

    result = perform_operation(param1, param2, &state.flags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        // C arithmetic right shift on signed int; in Rust, i32 >> behaves as
        // arithmetic shift.
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &state.flags);

    result = result.wrapping_add(base_offset);

    // snprintf(buffer, BUFFER_SIZE, "Result:%d:Complete", result);
    let formatted = format!("Result:{}:Complete", result);
    let bytes = formatted.as_bytes();
    let copy_len = if bytes.len() < BUFFER_SIZE - 1 {
        bytes.len()
    } else {
        BUFFER_SIZE - 1
    };
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
    buffer[copy_len] = 0;

    // Locate the first ':' and (optionally) a second one, mirroring strchr.
    if let Some(first_colon) = buffer.iter().position(|&b| b == b':') {
        if state.flags.verbose != 0 {
            println!("Found colon at position: {}", first_colon);
        }

        let second_colon = buffer[first_colon + 1..].iter().position(|&b| b == b':');
        if second_colon.is_some() && state.flags.debug != 0 {
            println!("Debug: Result string format validated");
        }
    }

    if result < 0 {
        // Equivalent to memcpy(&state, &state_backup, sizeof(state)).
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
