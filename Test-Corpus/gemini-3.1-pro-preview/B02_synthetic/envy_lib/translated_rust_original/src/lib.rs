use std::os::raw::c_int;

#[derive(Clone, Copy)]
struct ConfigFlags {
    verbose: bool,
    debug: bool,
    optimize: bool,
    cache_enabled: bool,
    log_level: u8,
    reserved: u8,
}

#[derive(Clone, Copy)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: u8,
}

fn parse_env_numeric(env_name: &str, default_val: c_int) -> c_int {
    if let Ok(env_value) = std::env::var(env_name) {
        if env_value.contains(',') {
            eprintln!("Warning: Invalid character in {}", env_name);
            return default_val;
        }
        if env_value.contains(';') {
            eprintln!("Warning: Semicolon found in {}", env_name);
            return default_val;
        }
        
        let s = env_value.trim_start();
        let mut end = 0;
        for (i, c) in s.char_indices() {
            if i == 0 && (c == '-' || c == '+') {
                end += c.len_utf8();
                continue;
            }
            if c.is_ascii_digit() {
                end += c.len_utf8();
            } else {
                break;
            }
        }
        s[..end].parse::<c_int>().unwrap_or(0)
    } else {
        default_val
    }
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = std::env::var("PROG_VERBOSE").ok();
    let debug_env = std::env::var("PROG_DEBUG").ok();
    let optimize_env = std::env::var("PROG_OPTIMIZE").ok();

    flags.verbose = verbose_env.map_or(false, |v| v.contains('1'));
    flags.debug = debug_env.map_or(false, |v| v.contains('1'));
    flags.optimize = optimize_env.is_some();
    flags.cache_enabled = true;
    flags.log_level = 0o3;
    flags.reserved = 0;
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let operation_mode = 0o755;

    let result = if flags.optimize {
        val1 + val2
    } else {
        (val1 * (flags.log_level as c_int)) + (val2 / 2)
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
        adjusted = adjusted << 1;
    }

    if flags.cache_enabled {
        adjusted = adjusted | 0x0F;
    }

    adjusted
}

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState {
        flags: ConfigFlags {
            verbose: false,
            debug: false,
            optimize: false,
            cache_enabled: false,
            log_level: 0,
            reserved: 0,
        },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };

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

    let mut result = perform_operation(param1, param2, &state.flags);

    if param3 != 0 {
        result += param3 * state.multiplier;
    }

    if param4 != 0 {
        result += param4 >> 2;
    }

    result = apply_bit_operations(result, &state.flags);

    result += base_offset;

    let buffer = format!("Result:{}:Complete", result);

    if let Some(colon_pos) = buffer.find(':') {
        if state.flags.verbose {
            println!("Found colon at position: {}", colon_pos);
        }

        if buffer[colon_pos + 1..].find(':').is_some() && state.flags.debug {
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
        println!("Configuration - Debug: {}, Optimize: {}, Log Level: {}",
                 state.flags.debug as u8, state.flags.optimize as u8, state.flags.log_level);
    }

    result
}
