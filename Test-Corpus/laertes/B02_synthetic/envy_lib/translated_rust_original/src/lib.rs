use ::c2rust_bitfields;
extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn snprintf(
        __s: *mut libc::c_char,
        __maxlen: size_t,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn atoi(__nptr: *const libc::c_char) -> libc::c_int;
    fn getenv(__name: *const libc::c_char) -> *mut libc::c_char;
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strchr(__s: *const libc::c_char, __c: libc::c_int)
        -> *mut libc::c_char;
}
pub type size_t = usize;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct ConfigFlags {
    #[bitfield(name = "verbose", ty = "libc::c_uint", bits = "0..=0")]
    #[bitfield(name = "debug", ty = "libc::c_uint", bits = "1..=1")]
    #[bitfield(name = "optimize", ty = "libc::c_uint", bits = "2..=2")]
    #[bitfield(name = "cache_enabled", ty = "libc::c_uint", bits = "3..=3")]
    #[bitfield(name = "log_level", ty = "libc::c_uint", bits = "4..=6")]
    #[bitfield(name = "reserved", ty = "libc::c_uint", bits = "7..=7")]
    pub verbose_debug_optimize_cache_enabled_log_level_reserved: [u8; 1],
    #[bitfield(padding)]
    pub c2rust_padding: [u8; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ProcessState {
    pub flags: ConfigFlags,
    pub base_value: libc::c_int,
    pub multiplier: libc::c_int,
    pub operation: libc::c_char,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const BUFFER_SIZE: libc::c_int = 256 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn parse_env_numeric(
    mut env_name: *const libc::c_char,
    mut default_val: libc::c_int,
) -> libc::c_int {
    let mut env_value: *mut libc::c_char = getenv(env_name);
    if env_value.is_null() {
        return default_val;
    }
    let mut invalid_char: *mut libc::c_char = strchr(env_value, ',' as i32);
    if !invalid_char.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Warning: Invalid character in %s\n\0" as *const u8 as *const libc::c_char,
            env_name,
        );
        return default_val;
    }
    invalid_char = strchr(env_value, ';' as i32);
    if !invalid_char.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Warning: Semicolon found in %s\n\0" as *const u8 as *const libc::c_char,
            env_name,
        );
        return default_val;
    }
    return atoi(env_value);
}
#[no_mangle]
pub unsafe extern "C" fn init_config_from_env(mut flags: *mut ConfigFlags) {
    let mut verbose_env: *mut libc::c_char =
        getenv(b"PROG_VERBOSE\0" as *const u8 as *const libc::c_char);
    let mut debug_env: *mut libc::c_char =
        getenv(b"PROG_DEBUG\0" as *const u8 as *const libc::c_char);
    let mut optimize_env: *mut libc::c_char =
        getenv(b"PROG_OPTIMIZE\0" as *const u8 as *const libc::c_char);
    (*flags).set_verbose(
        (if !verbose_env.is_null() && !strchr(verbose_env, '1' as i32).is_null() {
            1 as libc::c_int
        } else {
            0 as libc::c_int
        }) as libc::c_uint as libc::c_uint,
    );
    (*flags).set_debug(
        (if !debug_env.is_null() && !strchr(debug_env, '1' as i32).is_null() {
            1 as libc::c_int
        } else {
            0 as libc::c_int
        }) as libc::c_uint as libc::c_uint,
    );
    (*flags).set_optimize(
        (if !optimize_env.is_null() {
            1 as libc::c_int
        } else {
            0 as libc::c_int
        }) as libc::c_uint as libc::c_uint,
    );
    (*flags).set_cache_enabled(1 as libc::c_uint as libc::c_uint);
    (*flags).set_log_level(0o3 as libc::c_uint as libc::c_uint);
    (*flags).set_reserved(0 as libc::c_uint as libc::c_uint);
}
#[no_mangle]
pub unsafe extern "C" fn perform_operation(
    mut val1: libc::c_int,
    mut val2: libc::c_int,
    mut flags: *mut ConfigFlags,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut operation_mode: libc::c_int = 0o755 as libc::c_int;
    if (*flags).optimize() != 0 {
        result = val1 + val2;
    } else {
        result = val1 * (*flags).log_level() as libc::c_int + val2 / 2 as libc::c_int;
    }
    if (*flags).debug() != 0 {
        printf(
            b"Debug: operation_mode = %o (octal)\n\0" as *const u8 as *const libc::c_char,
            operation_mode,
        );
        printf(
            b"Debug: result before adjustment = %d\n\0" as *const u8 as *const libc::c_char,
            result,
        );
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn apply_bit_operations(
    mut value: libc::c_int,
    mut flags: *mut ConfigFlags,
) -> libc::c_int {
    let mut adjusted: libc::c_int = value;
    if (*flags).verbose() != 0 {
        adjusted = adjusted << 1 as libc::c_int;
    }
    if (*flags).cache_enabled() != 0 {
        adjusted = adjusted | 0xf as libc::c_int;
    }
    return adjusted;
}
#[no_mangle]
pub unsafe extern "C" fn envy(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    let mut state: ProcessState = ProcessState {
        flags: ConfigFlags {
            verbose_debug_optimize_cache_enabled_log_level_reserved: [0; 1],
            c2rust_padding: [0; 3],
        },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };
    let mut state_backup: ProcessState = ProcessState {
        flags: ConfigFlags {
            verbose_debug_optimize_cache_enabled_log_level_reserved: [0; 1],
            c2rust_padding: [0; 3],
        },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };
    let mut buffer: [libc::c_char; 256] = [0; 256];
    let mut result: libc::c_int = 0 as libc::c_int;
    init_config_from_env(&raw mut state.flags);
    let mut base_offset: libc::c_int = parse_env_numeric(
        b"PROG_BASE_OFFSET\0" as *const u8 as *const libc::c_char,
        0o100 as libc::c_int,
    );
    let mut multiplier: libc::c_int = parse_env_numeric(
        b"PROG_MULTIPLIER\0" as *const u8 as *const libc::c_char,
        0o12 as libc::c_int,
    );
    if state.flags.verbose() != 0 {
        printf(b"Verbose mode enabled\n\0" as *const u8 as *const libc::c_char);
        printf(
            b"Base offset: %d (from octal 0100)\n\0" as *const u8 as *const libc::c_char,
            base_offset,
        );
        printf(
            b"Multiplier: %d (from octal 012)\n\0" as *const u8 as *const libc::c_char,
            multiplier,
        );
    }
    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = '+' as i32 as libc::c_char;
    memcpy(
        &raw mut state_backup as *mut libc::c_void,
        &raw mut state as *const libc::c_void,
        std::mem::size_of::<ProcessState>() as size_t,
    );
    if state.flags.debug() != 0 {
        printf(
            b"Debug: Created state backup using memcpy\n\0" as *const u8
                as *const libc::c_char,
        );
        printf(
            b"Debug: Backup base_value = %d\n\0" as *const u8 as *const libc::c_char,
            state_backup.base_value,
        );
    }
    result = perform_operation(param1, param2, &raw mut state.flags);
    if param3 != 0 as libc::c_int {
        result += param3 * state.multiplier;
    }
    if param4 != 0 as libc::c_int {
        result += param4 >> 2 as libc::c_int;
    }
    result = apply_bit_operations(result, &raw mut state.flags);
    result += base_offset;
    snprintf(
        &raw mut buffer as *mut libc::c_char,
        BUFFER_SIZE as size_t,
        b"Result:%d:Complete\0" as *const u8 as *const libc::c_char,
        result,
    );
    let mut colon_pos: *mut libc::c_char =
        strchr(&raw mut buffer as *mut libc::c_char, ':' as i32);
    if !colon_pos.is_null() {
        if state.flags.verbose() != 0 {
            printf(
                b"Found colon at position: %ld\n\0" as *const u8 as *const libc::c_char,
                colon_pos.offset_from(&raw mut buffer as *mut libc::c_char)
                    as libc::c_long,
            );
        }
        let mut second_colon: *mut libc::c_char = strchr(
            colon_pos.offset(1 as libc::c_int as isize),
            ':' as i32,
        );
        if !second_colon.is_null() && state.flags.debug() as libc::c_int != 0 {
            printf(
                b"Debug: Result string format validated\n\0" as *const u8
                    as *const libc::c_char,
            );
        }
    }
    if result < 0 as libc::c_int {
        memcpy(
            &raw mut state as *mut libc::c_void,
            &raw mut state_backup as *const libc::c_void,
            std::mem::size_of::<ProcessState>() as size_t,
        );
        result = state.base_value;
        if state.flags.verbose() != 0 {
            printf(b"Restored state from backup\n\0" as *const u8 as *const libc::c_char);
        }
    }
    if state.flags.verbose() != 0 {
        printf(
            b"Final result: %d\n\0" as *const u8 as *const libc::c_char,
            result,
        );
        printf(
            b"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n\0" as *const u8
                as *const libc::c_char,
            state.flags.debug() as libc::c_int,
            state.flags.optimize() as libc::c_int,
            state.flags.log_level() as libc::c_int,
        );
    }
    return result;
}
