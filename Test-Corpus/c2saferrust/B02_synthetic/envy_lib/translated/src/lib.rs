



use std::env;

use ::c2rust_bitfields;
extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn atoi(__nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
}
pub type size_t = usize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct ConfigFlags {
    #[bitfield(name = "verbose", ty = "::core::ffi::c_uint", bits = "0..=0")]
    #[bitfield(name = "debug", ty = "::core::ffi::c_uint", bits = "1..=1")]
    #[bitfield(name = "optimize", ty = "::core::ffi::c_uint", bits = "2..=2")]
    #[bitfield(name = "cache_enabled", ty = "::core::ffi::c_uint", bits = "3..=3")]
    #[bitfield(name = "log_level", ty = "::core::ffi::c_uint", bits = "4..=6")]
    #[bitfield(name = "reserved", ty = "::core::ffi::c_uint", bits = "7..=7")]
    pub verbose_debug_optimize_cache_enabled_log_level_reserved: [u8; 1],
    #[bitfield(padding)]
    pub c2rust_padding: [u8; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ProcessState {
    pub flags: ConfigFlags,
    pub base_value: ::core::ffi::c_int,
    pub multiplier: ::core::ffi::c_int,
    pub operation: ::core::ffi::c_char,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const BUFFER_SIZE: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
#[no_mangle]
pub fn parse_env_numeric(env_name: &str, default_val: ::core::ffi::c_int) -> ::core::ffi::c_int {
    match env::var(env_name) {
        Ok(env_value) => {
            if env_value.contains(',') {
                eprintln!("Warning: Invalid character in {}", env_name);
                default_val
            } else if env_value.contains(';') {
                eprintln!("Warning: Semicolon found in {}", env_name);
                default_val
            } else {
                env_value
                    .trim()
                    .parse::<::core::ffi::c_int>()
                    .unwrap_or(0)
            }
        }
        Err(_) => default_val,
    }
}

#[no_mangle]
pub fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose = match env::var("PROG_VERBOSE") {
        Ok(v) => v.contains('1'),
        Err(_) => false,
    };
    let debug = match env::var("PROG_DEBUG") {
        Ok(v) => v.contains('1'),
        Err(_) => false,
    };
    let optimize = env::var("PROG_OPTIMIZE").is_ok();

    flags.set_verbose(verbose as u32);
    flags.set_debug(debug as u32);
    flags.set_optimize(optimize as u32);
    flags.set_cache_enabled(1u32);
    flags.set_log_level(0o3u32);
    flags.set_reserved(0u32);
}

#[no_mangle]
pub fn perform_operation(val1: i32, val2: i32, flags: &ConfigFlags) -> i32 {
    let mut result: i32;
    let operation_mode: i32 = 0o755;

    if flags.optimize() != 0 {
        result = val1 + val2;
    } else {
        result = val1 * flags.log_level() as i32 + val2 / 2;
    }

    if flags.debug() != 0 {
        println!("Debug: operation_mode = {:o} (octal)", operation_mode);
        println!("Debug: result before adjustment = {}", result);
    }

    result
}

#[no_mangle]
pub fn apply_bit_operations(mut value: i32, flags: &ConfigFlags) -> i32 {
    if flags.verbose() != 0 {
        value <<= 1;
    }
    if flags.cache_enabled() != 0 {
        value |= 0xf;
    }
    value
}

#[no_mangle]
pub fn envy(
    param1: ::core::ffi::c_int,
    param2: ::core::ffi::c_int,
    param3: ::core::ffi::c_int,
    param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut state = ProcessState {
        flags: ConfigFlags {
            verbose_debug_optimize_cache_enabled_log_level_reserved: [0; 1],
            c2rust_padding: [0; 3],
        },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };
    let mut state_backup = ProcessState {
        flags: ConfigFlags {
            verbose_debug_optimize_cache_enabled_log_level_reserved: [0; 1],
            c2rust_padding: [0; 3],
        },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };
    let mut result: ::core::ffi::c_int = 0;

    init_config_from_env(&mut state.flags);

    let base_offset = parse_env_numeric("PROG_BASE_OFFSET", 0o100);
    let multiplier = parse_env_numeric("PROG_MULTIPLIER", 0o12);

    if state.flags.verbose() != 0 {
        println!("Verbose mode enabled");
        println!("Base offset: {} (from octal 0100)", base_offset);
        println!("Multiplier: {} (from octal 012)", multiplier);
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = '+' as ::core::ffi::c_char;

    state_backup = state;

    if state.flags.debug() != 0 {
        println!("Debug: Created state backup using memcpy");
        println!("Debug: Backup base_value = {}", state_backup.base_value);
    }

    result = perform_operation(param1, param2, &mut state.flags);

    if param3 != 0 {
        result += param3 * state.multiplier;
    }

    if param4 != 0 {
        result += param4 >> 2;
    }

    result = apply_bit_operations(result, &mut state.flags);
    result += base_offset;

    let buffer = format!("Result:{}:Complete", result);

    if let Some(colon_pos) = buffer.find(':') {
        if state.flags.verbose() != 0 {
            println!("Found colon at position: {}", colon_pos);
        }

        if buffer[colon_pos + 1..].contains(':') && state.flags.debug() != 0 {
            println!("Debug: Result string format validated");
        }
    }

    if result < 0 {
        state = state_backup;
        result = state.base_value;

        if state.flags.verbose() != 0 {
            println!("Restored state from backup");
        }
    }

    if state.flags.verbose() != 0 {
        println!("Final result: {}", result);
        println!(
            "Configuration - Debug: {}, Optimize: {}, Log Level: {}",
            state.flags.debug() as ::core::ffi::c_int,
            state.flags.optimize() as ::core::ffi::c_int,
            state.flags.log_level() as ::core::ffi::c_int,
        );
    }

    result
}

