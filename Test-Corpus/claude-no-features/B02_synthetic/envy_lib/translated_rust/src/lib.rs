// Translation of c_src/src/lib.c to Rust producing byte-identical output.

use std::ffi::OsString;
use std::os::raw::c_int;
use std::os::unix::ffi::OsStrExt;

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
    base_value: c_int,
    multiplier: c_int,
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

const BUFFER_SIZE: usize = 256;

fn get_env_bytes(name: &str) -> Option<Vec<u8>> {
    let val: Option<OsString> = std::env::var_os(name);
    val.map(|v| v.as_bytes().to_vec())
}

/// Mimic C's atoi: skips leading whitespace, optional sign, parses digits.
/// Operates on the input until a non-digit byte is encountered (or NUL/end).
fn c_atoi(bytes: &[u8]) -> i32 {
    let mut i = 0usize;
    // Stop at NUL
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let s = &bytes[..len];

    while i < s.len()
        && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
    {
        i += 1;
    }

    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    let mut result: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i32;
        result = result.wrapping_mul(10);
        if neg {
            result = result.wrapping_sub(d);
        } else {
            result = result.wrapping_add(d);
        }
        i += 1;
    }
    result
}

fn parse_env_numeric(env_name: &str, default_val: i32) -> i32 {
    let env_value = match get_env_bytes(env_name) {
        Some(v) => v,
        None => return default_val,
    };

    if env_value.contains(&b',') {
        eprintln!("Warning: Invalid character in {}", env_name);
        return default_val;
    }

    if env_value.contains(&b';') {
        eprintln!("Warning: Semicolon found in {}", env_name);
        return default_val;
    }

    c_atoi(&env_value)
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = get_env_bytes("PROG_VERBOSE");
    let debug_env = get_env_bytes("PROG_DEBUG");
    let optimize_env = get_env_bytes("PROG_OPTIMIZE");

    flags.verbose = match verbose_env {
        Some(ref v) if v.contains(&b'1') => 1,
        _ => 0,
    };
    flags.debug = match debug_env {
        Some(ref v) if v.contains(&b'1') => 1,
        _ => 0,
    };
    flags.optimize = if optimize_env.is_some() { 1 } else { 0 };
    flags.cache_enabled = 1;
    flags.log_level = 0o3; // 03 octal = 3
    flags.reserved = 0;
}

fn perform_operation(val1: i32, val2: i32, flags: &ConfigFlags) -> i32 {
    let result: i32;

    let operation_mode: i32 = 0o755;

    if flags.optimize != 0 {
        result = val1.wrapping_add(val2);
    } else {
        // (val1 * flags->log_level) + (val2 / 2)
        // log_level is unsigned int bitfield (3 bits). In C, when mixed with int
        // val1, the unsigned int promotes the operation. Use wrapping mul/add
        // to avoid overflow panics; values used in tests should be in range.
        let prod = (val1 as i64).wrapping_mul(flags.log_level as i64) as i32;
        let div = val2 / 2;
        result = prod.wrapping_add(div);
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

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState::default();
    let state_backup;
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut result: i32 = 0;

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

    // memcpy(&state_backup, &state, sizeof(struct ProcessState));
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
        // C: result += param4 >> 2; signed shift
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &state.flags);

    result = result.wrapping_add(base_offset);

    // snprintf(buffer, BUFFER_SIZE, "Result:%d:Complete", result);
    let s = format!("Result:{}:Complete", result);
    let bytes = s.as_bytes();
    let copy_len = bytes.len().min(BUFFER_SIZE - 1);
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
    buffer[copy_len] = 0;

    // char* colon_pos = strchr(buffer, ':');
    let str_len = copy_len;
    let buf_slice = &buffer[..str_len];
    if let Some(first_colon) = buf_slice.iter().position(|&b| b == b':') {
        if state.flags.verbose != 0 {
            println!("Found colon at position: {}", first_colon as i64);
        }

        // strchr(colon_pos + 1, ':')
        let after_first = &buf_slice[first_colon + 1..];
        let second_colon = after_first.iter().position(|&b| b == b':');
        if second_colon.is_some() && state.flags.debug != 0 {
            println!("Debug: Result string format validated");
        }
    }

    if result < 0 {
        // memcpy(&state, &state_backup, sizeof(struct ProcessState));
        state = state_backup;
        result = state.base_value;

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

    result as c_int
}
