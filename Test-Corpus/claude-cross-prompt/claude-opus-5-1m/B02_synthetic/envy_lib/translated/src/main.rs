// Translation of c_src/src/lib.c to Rust.
// Produces byte-identical output for the same inputs as the C version
// when wrapped with a main that reads four ints from stdin.

use std::env;
use std::io::{self, Read, Write};

#[derive(Clone, Copy, Default)]
struct ConfigFlags {
    verbose: u8,      // 1 bit
    debug: u8,        // 1 bit
    optimize: u8,     // 1 bit
    cache_enabled: u8,// 1 bit
    log_level: u8,    // 3 bits
    reserved: u8,     // 1 bit
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

const BUFFER_SIZE: usize = 256;

/// Replicate C's atoi: skip leading whitespace, optional sign, then consume
/// digits as long as possible. Stops on first non-digit. Returns 0 if none.
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Skip leading whitespace as defined by C's isspace
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
            i += 1;
        } else {
            break;
        }
    }
    let mut sign: i64 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut value: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    (value.wrapping_mul(sign)) as i32
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

    c_atoi(&env_value)
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = env::var("PROG_VERBOSE").ok();
    let debug_env = env::var("PROG_DEBUG").ok();
    let optimize_env = env::var("PROG_OPTIMIZE").ok();

    flags.verbose = match &verbose_env {
        Some(v) if v.contains('1') => 1,
        _ => 0,
    };
    flags.debug = match &debug_env {
        Some(v) if v.contains('1') => 1,
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
        // C signed division truncates toward zero
        let m = val1.wrapping_mul(flags.log_level as i32);
        let d = val2 / 2; // Rust i32 / i32 also truncates toward zero
        result = m.wrapping_add(d);
    }

    if flags.debug != 0 {
        // %o prints the value in octal without leading 0
        println!("Debug: operation_mode = {:o} (octal)", operation_mode);
        println!("Debug: result before adjustment = {}", result);
    }

    result
}

fn apply_bit_operations(value: i32, flags: &ConfigFlags) -> i32 {
    let mut adjusted = value;

    if flags.verbose != 0 {
        // C left-shift of signed int with overflow is UB but commonly wraps;
        // mirror as wrapping shift on the unsigned representation.
        adjusted = ((adjusted as u32).wrapping_shl(1)) as i32;
    }

    if flags.cache_enabled != 0 {
        adjusted |= 0x0F;
    }

    adjusted
}

fn envy(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut state = ProcessState::default();
    let state_backup: ProcessState;
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

    state_backup = state; // memcpy equivalent

    if state.flags.debug != 0 {
        println!("Debug: Created state backup using memcpy");
        println!("Debug: Backup base_value = {}", state_backup.base_value);
    }

    result = perform_operation(param1, param2, &state.flags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        // C arithmetic right shift on signed int (typically arithmetic).
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &state.flags);

    result = result.wrapping_add(base_offset);

    // snprintf into buffer: "Result:%d:Complete"
    let formatted = format!("Result:{}:Complete", result);
    let bytes = formatted.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), BUFFER_SIZE - 1);
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
    buffer[copy_len] = 0;

    // Find first ':' in buffer (up to NUL)
    let mut nul_idx = 0usize;
    while nul_idx < BUFFER_SIZE && buffer[nul_idx] != 0 {
        nul_idx += 1;
    }
    let mut colon_pos: Option<usize> = None;
    for i in 0..nul_idx {
        if buffer[i] == b':' {
            colon_pos = Some(i);
            break;
        }
    }
    if let Some(cp) = colon_pos {
        if state.flags.verbose != 0 {
            println!("Found colon at position: {}", cp);
        }

        // Find second ':' starting from cp + 1
        let mut second_colon: Option<usize> = None;
        for i in (cp + 1)..nul_idx {
            if buffer[i] == b':' {
                second_colon = Some(i);
                break;
            }
        }
        if second_colon.is_some() && state.flags.debug != 0 {
            println!("Debug: Result string format validated");
        }
    }

    if result < 0 {
        state = state_backup; // memcpy restore
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

    result
}

/// Read all of stdin and parse four whitespace-separated integers in
/// scanf("%d", ...) style: skip leading whitespace, optional sign, digits.
fn read_four_ints() -> Option<(i32, i32, i32, i32)> {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return None;
    }
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut vals = [0i32; 4];

    for slot in 0..4 {
        // Skip whitespace
        while i < bytes.len() {
            let c = bytes[i];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
                i += 1;
            } else {
                break;
            }
        }
        if i >= bytes.len() {
            return None;
        }
        let mut sign: i64 = 1;
        if bytes[i] == b'+' || bytes[i] == b'-' {
            if bytes[i] == b'-' {
                sign = -1;
            }
            i += 1;
        }
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            return None;
        }
        let mut value: i64 = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            value = value.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
            i += 1;
        }
        vals[slot] = (value.wrapping_mul(sign)) as i32;
    }
    Some((vals[0], vals[1], vals[2], vals[3]))
}

fn main() {
    let (a, b, c, d) = match read_four_ints() {
        Some(t) => t,
        None => {
            // No input or insufficient input: nothing to do
            return;
        }
    };
    let result = envy(a, b, c, d);
    // Match C printf("%d\n", result)
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", result);
}
