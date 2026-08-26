use libc::{c_char, c_int};
use std::ffi::CStr;

const BUFFER_SIZE: usize = 256;
const PROG_VERBOSE: &[u8] = b"PROG_VERBOSE\0";
const PROG_DEBUG: &[u8] = b"PROG_DEBUG\0";
const PROG_OPTIMIZE: &[u8] = b"PROG_OPTIMIZE\0";
const PROG_BASE_OFFSET: &[u8] = b"PROG_BASE_OFFSET\0";
const PROG_MULTIPLIER: &[u8] = b"PROG_MULTIPLIER\0";

#[derive(Clone, Copy, Default)]
struct ConfigFlags {
    verbose: c_int,
    debug: c_int,
    optimize: c_int,
    cache_enabled: c_int,
    log_level: c_int,
    reserved: c_int,
}

#[derive(Clone, Copy, Default)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: c_char,
}

fn env_ptr(name: &'static [u8]) -> *const c_char {
    name.as_ptr().cast()
}

fn write_stderr_bytes(bytes: &[u8]) {
    unsafe {
        libc::write(libc::STDERR_FILENO, bytes.as_ptr().cast(), bytes.len());
    }
}

fn print_stdout(args: std::fmt::Arguments<'_>) {
    use std::io::Write;

    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_fmt(args);
}

fn parse_env_numeric(env_name: *const c_char, default_val: c_int) -> c_int {
    unsafe {
        let env_value = libc::getenv(env_name);
        if env_value.is_null() {
            return default_val;
        }

        if !libc::strchr(env_value, b',' as c_int).is_null() {
            let env_name_str = CStr::from_ptr(env_name).to_bytes();
            let mut msg = Vec::with_capacity(b"Warning: Invalid character in \n".len() + env_name_str.len());
            msg.extend_from_slice(b"Warning: Invalid character in ");
            msg.extend_from_slice(env_name_str);
            msg.push(b'\n');
            write_stderr_bytes(&msg);
            return default_val;
        }

        if !libc::strchr(env_value, b';' as c_int).is_null() {
            let env_name_str = CStr::from_ptr(env_name).to_bytes();
            let mut msg = Vec::with_capacity(b"Warning: Semicolon found in \n".len() + env_name_str.len());
            msg.extend_from_slice(b"Warning: Semicolon found in ");
            msg.extend_from_slice(env_name_str);
            msg.push(b'\n');
            write_stderr_bytes(&msg);
            return default_val;
        }

        libc::atoi(env_value)
    }
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    unsafe {
        let verbose_env = libc::getenv(env_ptr(PROG_VERBOSE));
        let debug_env = libc::getenv(env_ptr(PROG_DEBUG));
        let optimize_env = libc::getenv(env_ptr(PROG_OPTIMIZE));

        flags.verbose = if !verbose_env.is_null() && !libc::strchr(verbose_env, b'1' as c_int).is_null() {
            1
        } else {
            0
        };
        flags.debug = if !debug_env.is_null() && !libc::strchr(debug_env, b'1' as c_int).is_null() {
            1
        } else {
            0
        };
        flags.optimize = if !optimize_env.is_null() { 1 } else { 0 };
        flags.cache_enabled = 1;
        flags.log_level = 0o3;
        flags.reserved = 0;
    }
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let operation_mode: c_int = 0o755;

    let result = if flags.optimize != 0 {
        val1.wrapping_add(val2)
    } else {
        val1
            .wrapping_mul(flags.log_level)
            .wrapping_add(val2 / 2)
    };

    if flags.debug != 0 {
        print_stdout(format_args!("Debug: operation_mode = {:o} (octal)\n", operation_mode));
        print_stdout(format_args!("Debug: result before adjustment = {}\n", result));
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
pub extern "C" fn envy(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut state = ProcessState::default();
    let mut buffer = [0_u8; BUFFER_SIZE];
    let mut result: c_int;

    init_config_from_env(&mut state.flags);

    let base_offset = parse_env_numeric(env_ptr(PROG_BASE_OFFSET), 0o100);
    let multiplier = parse_env_numeric(env_ptr(PROG_MULTIPLIER), 0o12);

    if state.flags.verbose != 0 {
        print_stdout(format_args!("Verbose mode enabled\n"));
        print_stdout(format_args!("Base offset: {} (from octal 0100)\n", base_offset));
        print_stdout(format_args!("Multiplier: {} (from octal 012)\n", multiplier));
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    let state_backup = state;

    if state.flags.debug != 0 {
        print_stdout(format_args!("Debug: Created state backup using memcpy\n"));
        print_stdout(format_args!("Debug: Backup base_value = {}\n", state_backup.base_value));
    }

    result = perform_operation(param1, param2, &state.flags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &state.flags);
    result = result.wrapping_add(base_offset);

    let rendered = format!("Result:{}:Complete", result);
    let bytes = rendered.as_bytes();
    let copy_len = bytes.len().min(BUFFER_SIZE.saturating_sub(1));
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);

    if let Some(colon_pos) = buffer[..copy_len].iter().position(|&b| b == b':') {
        if state.flags.verbose != 0 {
            print_stdout(format_args!("Found colon at position: {}\n", colon_pos));
        }

        if buffer[colon_pos + 1..copy_len].contains(&b':') && state.flags.debug != 0 {
            print_stdout(format_args!("Debug: Result string format validated\n"));
        }
    }

    if result < 0 {
        state = state_backup;
        result = state.base_value;

        if state.flags.verbose != 0 {
            print_stdout(format_args!("Restored state from backup\n"));
        }
    }

    if state.flags.verbose != 0 {
        print_stdout(format_args!("Final result: {}\n", result));
        print_stdout(format_args!(
            "Configuration - Debug: {}, Optimize: {}, Log Level: {}\n",
            state.flags.debug, state.flags.optimize, state.flags.log_level
        ));
    }

    result
}
