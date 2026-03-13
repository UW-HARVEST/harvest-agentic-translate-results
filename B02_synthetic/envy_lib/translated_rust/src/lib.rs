use std::ffi::{c_int, CStr, CString};
use std::os::raw::c_char;

struct ConfigFlags {
    verbose: bool,
    debug: bool,
    optimize: bool,
    cache_enabled: bool,
    log_level: u8,
}

#[allow(dead_code)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    _operation: c_char,
}

fn getenv(name: &str) -> Option<CString> {
    let c_name = CString::new(name).ok()?;
    unsafe {
        let ptr = libc_getenv(c_name.as_ptr());
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr).to_owned())
        }
    }
}

extern "C" {
    #[link_name = "getenv"]
    fn libc_getenv(name: *const c_char) -> *const c_char;
}

fn parse_env_numeric(env_name: &str, default_val: c_int) -> c_int {
    let env_value = match getenv(env_name) {
        Some(v) => v,
        None => return default_val,
    };

    let bytes = env_value.as_bytes();

    if bytes.contains(&b',') {
        eprint!("Warning: Invalid character in {}\n", env_name);
        return default_val;
    }

    if bytes.contains(&b';') {
        eprint!("Warning: Semicolon found in {}\n", env_name);
        return default_val;
    }

    // Replicate atoi behavior: skip leading whitespace, optional sign, then digits
    let s = env_value.to_str().unwrap_or("");
    let s = s.trim_start();
    atoi(s)
}

fn atoi(s: &str) -> c_int {
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('+') => { chars.next(); false }
        Some('-') => { chars.next(); true }
        _ => false,
    };
    let mut val: c_int = 0;
    for c in chars {
        if c.is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add((c as u8 - b'0') as c_int);
        } else {
            break;
        }
    }
    if neg { val.wrapping_neg() } else { val }
}

fn init_config_from_env() -> ConfigFlags {
    let verbose_env = getenv("PROG_VERBOSE");
    let debug_env = getenv("PROG_DEBUG");
    let optimize_env = getenv("PROG_OPTIMIZE");

    let verbose = match &verbose_env {
        Some(v) => v.as_bytes().contains(&b'1'),
        None => false,
    };
    let debug = match &debug_env {
        Some(v) => v.as_bytes().contains(&b'1'),
        None => false,
    };
    let optimize = optimize_env.is_some();

    ConfigFlags {
        verbose,
        debug,
        optimize,
        cache_enabled: true,
        log_level: 3, // octal 03 = 3
    }
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let result;
    let operation_mode: c_int = 0o755;

    if flags.optimize {
        result = val1.wrapping_add(val2);
    } else {
        result = (val1.wrapping_mul(flags.log_level as c_int)).wrapping_add(val2 / 2);
    }

    if flags.debug {
        print!("Debug: operation_mode = {:o} (octal)\n", operation_mode);
        print!("Debug: result before adjustment = {}\n", result);
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
    let flags = init_config_from_env();

    let base_offset = parse_env_numeric("PROG_BASE_OFFSET", 0o100); // octal 0100 = 64
    let multiplier = parse_env_numeric("PROG_MULTIPLIER", 0o12);    // octal 012 = 10

    if flags.verbose {
        print!("Verbose mode enabled\n");
        print!("Base offset: {} (from octal 0100)\n", base_offset);
        print!("Multiplier: {} (from octal 012)\n", multiplier);
    }

    let base_value = param1;

    // Save backup of flags for potential restore
    let backup_verbose = flags.verbose;
    let _backup_debug = flags.debug;
    let _backup_optimize = flags.optimize;

    if flags.debug {
        print!("Debug: Created state backup using memcpy\n");
        print!("Debug: Backup base_value = {}\n", base_value);
    }

    let mut result = perform_operation(param1, param2, &flags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(multiplier));
    }

    if param4 != 0 {
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &flags);

    result = result.wrapping_add(base_offset);

    let buffer = format!("Result:{}:Complete", result);

    if let Some(colon_pos) = buffer.find(':') {
        if flags.verbose {
            print!("Found colon at position: {}\n", colon_pos);
        }

        if let Some(_) = buffer[colon_pos + 1..].find(':') {
            if flags.debug {
                print!("Debug: Result string format validated\n");
            }
        }
    }

    if result < 0 {
        // Restore from backup
        result = base_value;

        if backup_verbose {
            print!("Restored state from backup\n");
        }
    }

    if flags.verbose {
        print!("Final result: {}\n", result);
        print!(
            "Configuration - Debug: {}, Optimize: {}, Log Level: {}\n",
            flags.debug as c_int,
            flags.optimize as c_int,
            flags.log_level
        );
    }

    result
}
