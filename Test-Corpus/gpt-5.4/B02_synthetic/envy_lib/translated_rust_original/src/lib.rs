use std::env;
use std::ffi::CString;
use std::os::raw::c_int;

#[derive(Clone, Copy, Default)]
struct ConfigFlags {
    verbose: bool,
    debug: bool,
    optimize: bool,
    cache_enabled: bool,
    log_level: u8,
    reserved: bool,
}

#[derive(Clone, Copy, Default)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: u8,
}

const BUFFER_SIZE: usize = 256;

fn parse_env_numeric(env_name: &str, default_val: c_int) -> c_int {
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

    env_value.parse::<c_int>().unwrap_or(0)
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = env::var("PROG_VERBOSE").ok();
    let debug_env = env::var("PROG_DEBUG").ok();
    let optimize_env = env::var("PROG_OPTIMIZE").ok();

    flags.verbose = verbose_env.as_deref().is_some_and(|v| v.contains('1'));
    flags.debug = debug_env.as_deref().is_some_and(|v| v.contains('1'));
    flags.optimize = optimize_env.is_some();
    flags.cache_enabled = true;
    flags.log_level = 0o3;
    flags.reserved = false;
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let operation_mode = 0o755;

    let result = if flags.optimize {
        val1 + val2
    } else {
        (val1 * flags.log_level as c_int) + (val2 / 2)
    };

    if flags.debug {
        println!("Debug: operation_mode = {:o} (octal)", operation_mode);
        println!("Debug: result before adjustment = {}", result);
    }

    result
}

fn apply_bit_operations(value: c_int, flags: &ConfigFlags) -> c_int {
    let mut adjusted = value;

    if flags.verbose {
        adjusted <<= 1;
    }

    if flags.cache_enabled {
        adjusted |= 0x0F;
    }

    adjusted
}

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState::default();
    let mut result: c_int;

    init_config_from_env(&mut state.flags);

    let base_offset = parse_env_numeric("PROG_BASE_OFFSET", 0o100);
    let multiplier = parse_env_numeric("PROG_MULTIPLIER", 0o12);

    if state.flags.verbose {
        println!("Verbose mode enabled");
        println!("Base offset: {} (from octal 0100)", base_offset);
        println!("Multiplier: {} (from octal 012)", multiplier);
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+';

    let state_backup = state;

    if state.flags.debug {
        println!("Debug: Created state backup using memcpy");
        println!("Debug: Backup base_value = {}", state_backup.base_value);
    }

    result = perform_operation(param1, param2, &state.flags);

    if param3 != 0 {
        result += param3 * state.multiplier;
    }

    if param4 != 0 {
        result += param4 >> 2;
    }

    result = apply_bit_operations(result, &state.flags);
    result += base_offset;

    let formatted = format!("Result:{}:Complete", result);
    let buffer_string = if formatted.len() >= BUFFER_SIZE {
        let mut bytes = formatted.into_bytes();
        bytes.truncate(BUFFER_SIZE.saturating_sub(1));
        CString::new(bytes)
            .ok()
            .and_then(|s| s.into_string().ok())
            .unwrap_or_default()
    } else {
        formatted
    };

    if let Some(colon_pos) = buffer_string.find(':') {
        if state.flags.verbose {
            println!("Found colon at position: {}", colon_pos);
        }

        if buffer_string[colon_pos + 1..].contains(':') && state.flags.debug {
            println!("Debug: Result string format validated");
        }
    }

    if result < 0 {
        state = state_backup;
        result = state.base_value;

        if state.flags.verbose {
            println!("Restored state from backup");
        }
    }

    if state.flags.verbose {
        println!("Final result: {}", result);
        println!(
            "Configuration - Debug: {}, Optimize: {}, Log Level: {}",
            state.flags.debug as c_int,
            state.flags.optimize as c_int,
            state.flags.log_level
        );
    }

    result
}
