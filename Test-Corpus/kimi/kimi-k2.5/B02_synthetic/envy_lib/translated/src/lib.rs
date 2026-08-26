use std::env;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

#[repr(C)]
struct ConfigFlags {
    bits: u8,
}

impl ConfigFlags {
    fn new() -> Self {
        ConfigFlags { bits: 0 }
    }

    fn verbose(&self) -> bool {
        (self.bits & 0x01) != 0
    }

    fn set_verbose(&mut self, value: bool) {
        if value {
            self.bits |= 0x01;
        } else {
            self.bits &= !0x01;
        }
    }

    fn debug(&self) -> bool {
        (self.bits & 0x02) != 0
    }

    fn set_debug(&mut self, value: bool) {
        if value {
            self.bits |= 0x02;
        } else {
            self.bits &= !0x02;
        }
    }

    fn optimize(&self) -> bool {
        (self.bits & 0x04) != 0
    }

    fn set_optimize(&mut self, value: bool) {
        if value {
            self.bits |= 0x04;
        } else {
            self.bits &= !0x04;
        }
    }

    fn cache_enabled(&self) -> bool {
        (self.bits & 0x08) != 0
    }

    fn set_cache_enabled(&mut self, value: bool) {
        if value {
            self.bits |= 0x08;
        } else {
            self.bits &= !0x08;
        }
    }

    fn log_level(&self) -> u8 {
        (self.bits >> 4) & 0x07
    }

    fn set_log_level(&mut self, value: u8) {
        self.bits = (self.bits & 0x8F) | ((value & 0x07) << 4);
    }
}

struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: char,
}

impl Clone for ProcessState {
    fn clone(&self) -> Self {
        ProcessState {
            flags: ConfigFlags { bits: self.flags.bits },
            base_value: self.base_value,
            multiplier: self.multiplier,
            operation: self.operation,
        }
    }
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

    env_value.parse::<c_int>().unwrap_or(default_val)
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = env::var("PROG_VERBOSE").ok();
    let debug_env = env::var("PROG_DEBUG").ok();
    let optimize_env = env::var("PROG_OPTIMIZE").ok();

    flags.set_verbose(verbose_env.as_ref().map_or(false, |v| v.contains('1')));
    flags.set_debug(debug_env.as_ref().map_or(false, |v| v.contains('1')));
    flags.set_optimize(optimize_env.is_some());
    flags.set_cache_enabled(true);
    flags.set_log_level(3);
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let operation_mode: c_int = 0o755;

    let mut result = if flags.optimize() {
        val1 + val2
    } else {
        (val1 * flags.log_level() as c_int) + (val2 / 2)
    };

    if flags.debug() {
        println!("Debug: operation_mode = {:o} (octal)", operation_mode);
        println!("Debug: result before adjustment = {}", result);
    }

    result
}

fn apply_bit_operations(value: c_int, flags: &ConfigFlags) -> c_int {
    let mut adjusted = value;

    if flags.verbose() {
        adjusted = adjusted << 1;
    }

    if flags.cache_enabled() {
        adjusted = adjusted | 0x0F;
    }

    adjusted
}

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState {
        flags: ConfigFlags::new(),
        base_value: 0,
        multiplier: 0,
        operation: '+',
    };

    init_config_from_env(&mut state.flags);

    let base_offset = parse_env_numeric("PROG_BASE_OFFSET", 0o100);
    let multiplier = parse_env_numeric("PROG_MULTIPLIER", 0o12);

    if state.flags.verbose() {
        println!("Verbose mode enabled");
        println!("Base offset: {} (from octal 0100)", base_offset);
        println!("Multiplier: {} (from octal 012)", multiplier);
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = '+';

    let state_backup = state.clone();

    if state.flags.debug() {
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
        if state.flags.verbose() {
            println!("Found colon at position: {}", colon_pos);
        }

        if buffer[colon_pos + 1..].find(':').is_some() && state.flags.debug() {
            println!("Debug: Result string format validated");
        }
    }

    if result < 0 {
        state.base_value = state_backup.base_value;
        state.multiplier = state_backup.multiplier;
        state.operation = state_backup.operation;
        state.flags.bits = state_backup.flags.bits;
        result = state.base_value;

        if state.flags.verbose() {
            println!("Restored state from backup");
        }
    }

    if state.flags.verbose() {
        println!("Final result: {}", result);
        println!(
            "Configuration - Debug: {}, Optimize: {}, Log Level: {}",
            state.flags.debug() as c_int,
            state.flags.optimize() as c_int,
            state.flags.log_level()
        );
    }

    result
}
