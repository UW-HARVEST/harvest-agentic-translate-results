// Rust translation of the C library. Uses libc printf/snprintf to produce
// byte-identical output compared to the original C implementation.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uint;
use std::os::raw::c_void;
use std::ptr;

// Mirrors the C bit-field struct. Each field is stored independently and
// masked to its bit-width on assignment so results match the original
// behavior.
#[derive(Clone, Copy, Default)]
struct PackedFlags {
    flag1: u32,    // 1 bit
    flag2: u32,    // 1 bit
    flag3: u32,    // 1 bit
    counter: u32,  // 5 bits
    mode: u32,     // 3 bits
    status: u32,   // 5 bits
    reserved: u32, // 16 bits
}

// Mirrors the C union. We store the raw 4 bytes and reinterpret on access.
#[derive(Clone, Copy)]
struct TypeConfusion {
    bytes: [u8; 4],
}

impl TypeConfusion {
    fn new() -> Self {
        TypeConfusion { bytes: [0u8; 4] }
    }

    fn set_int(&mut self, v: c_int) {
        self.bytes = v.to_ne_bytes();
    }

    fn get_int(&self) -> c_int {
        c_int::from_ne_bytes(self.bytes)
    }

    fn get_float(&self) -> f32 {
        f32::from_ne_bytes(self.bytes)
    }

    fn get_uint(&self) -> c_uint {
        c_uint::from_ne_bytes(self.bytes)
    }
}

#[repr(C)]
struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

// Helper: print "Debug: <var-name> = <value>\n"
fn debug_var(name: &[u8], val: c_int) {
    // Build a NUL-terminated format string: "Debug: <name> = %d\n\0"
    let mut buf: Vec<u8> = Vec::with_capacity(name.len() + 16);
    buf.extend_from_slice(b"Debug: ");
    buf.extend_from_slice(name);
    buf.extend_from_slice(b" = %d\n\0");
    unsafe {
        printf(buf.as_ptr() as *const c_char, val);
    }
}

// Helper: print "Operation: <op-name> with value <val>\n"
fn log_operation(op: &[u8], val: c_int) {
    let mut buf: Vec<u8> = Vec::with_capacity(op.len() + 32);
    buf.extend_from_slice(b"Operation: ");
    buf.extend_from_slice(op);
    buf.extend_from_slice(b" with value %d\n\0");
    unsafe {
        printf(buf.as_ptr() as *const c_char, val);
    }
}

unsafe fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = malloc(std::mem::size_of::<ProcessState>()) as *mut ProcessState;

    if state.is_null() {
        printf(b"Error: Failed to allocate memory for state\n\0".as_ptr() as *const c_char);
        return ptr::null_mut();
    }

    // Initialize the structure (the C code only sets the listed fields, but
    // we initialize the whole struct here for safety).
    ptr::write(
        state,
        ProcessState {
            flags: PackedFlags::default(),
            data: TypeConfusion::new(),
            buffer: ptr::null_mut(),
            capacity: 0,
        },
    );

    (*state).flags.flag1 = 1 & 0x1;
    (*state).flags.flag2 = 0 & 0x1;
    (*state).flags.flag3 = 1 & 0x1;
    (*state).flags.counter = 0 & 0x1F;
    (*state).flags.mode = 3 & 0x7;
    (*state).flags.status = 15 & 0x1F;
    (*state).flags.reserved = 0 & 0xFFFF;

    (*state).data.set_int(initial_val);

    (*state).capacity = capacity;
    (*state).buffer = malloc(capacity as usize) as *mut c_char;

    if (*state).buffer.is_null() {
        printf(b"Error: Failed to allocate buffer\n\0".as_ptr() as *const c_char);
        free(state as *mut c_void);
        return ptr::null_mut();
    }

    snprintf(
        (*state).buffer,
        capacity as usize,
        b"State:%d:Mode:%d\0".as_ptr() as *const c_char,
        initial_val,
        (*state).flags.mode as c_int,
    );

    state
}

unsafe fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        if !(*state).buffer.is_null() {
            free((*state).buffer as *mut c_void);
        }
        free(state as *mut c_void);
    }
}

unsafe fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || (*state).buffer.is_null() {
        printf(b"Error: Null pointer in process_buffer\n\0".as_ptr() as *const c_char);
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr_cur: *mut c_char = (*state).buffer;
    let mut remaining: usize = strlen((*state).buffer);

    while remaining > 0 {
        let found = memchr(
            ptr_cur as *const c_void,
            target as c_int,
            remaining,
        ) as *mut c_char;

        if found.is_null() {
            break;
        }

        count += 1;
        log_operation(b"memchr_found", count);

        let diff = found as isize - ptr_cur as isize;
        remaining -= (diff + 1) as usize;
        ptr_cur = found.offset(1);
    }

    count
}

unsafe fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    (*state).flags.counter = ((*state).flags.counter.wrapping_add(1)) & 0x1F;
    (*state).flags.flag1 = (param & 1) as u32 & 0x1;
    (*state).flags.flag2 = ((param & 2) >> 1) as u32 & 0x1;
    (*state).flags.flag3 = ((param & 4) >> 2) as u32 & 0x1;
    (*state).flags.mode = ((param >> 3) & 0x7) as u32 & 0x7;

    debug_var(b"state->flags.counter", (*state).flags.counter as c_int);
    printf(
        b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0".as_ptr() as *const c_char,
        (*state).flags.flag1 as c_int,
        (*state).flags.flag2 as c_int,
        (*state).flags.flag3 as c_int,
        (*state).flags.mode as c_int,
    );
}

unsafe fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    match operation {
        0 => {
            (*state).data.set_int(1078530011);
            printf(
                b"Set as int: %d\n\0".as_ptr() as *const c_char,
                (*state).data.get_int(),
            );
        }
        1 => {
            // float gets promoted to double by varargs
            let f = (*state).data.get_float();
            printf(
                b"Read as float: %f\n\0".as_ptr() as *const c_char,
                f as f64,
            );
            result = (f * 100.0) as c_int;
        }
        2 => {
            printf(
                b"Read as uint: %u\n\0".as_ptr() as *const c_char,
                (*state).data.get_uint(),
            );
            result = ((*state).data.get_uint() & 0xFF) as c_int;
        }
        3 => {
            // bytes are signed char in C on most platforms targeted here
            // (we mirror the printf which prints them as %d). char in C may
            // be signed or unsigned depending on platform; on x86_64 Linux
            // it is signed, matching i8.
            let b0 = (*state).data.bytes[0] as i8 as c_int;
            let b1 = (*state).data.bytes[1] as i8 as c_int;
            let b2 = (*state).data.bytes[2] as i8 as c_int;
            let b3 = (*state).data.bytes[3] as i8 as c_int;
            printf(
                b"Read as bytes: [%d, %d, %d, %d]\n\0".as_ptr() as *const c_char,
                b0,
                b1,
                b2,
                b3,
            );
            result = b0 + b1;
        }
        _ => {}
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        debug_var(b"param1", param1);
        debug_var(b"param2", param2);
        debug_var(b"param3", param3);
        debug_var(b"param4", param4);

        let mut result: c_int = 0;

        let state = create_state(param1, 128);

        if state.is_null() {
            return -1;
        }

        update_flags(state, param2);

        // search_char = '0' + (param3 % 10)
        // In C, param3 % 10 has the sign of param3, and char arithmetic
        // produces an int that is then truncated to char.
        let search_char = (b'0' as c_int + (param3 % 10)) as c_char;
        let found_count = process_buffer(state, search_char);
        result += found_count * 10;

        let confusion_result = confuse_types(state, param4 % 4);
        result += confusion_result;

        result += (*state).flags.counter as c_int * 5;
        result += (*state).flags.mode as c_int * 3;

        printf(
            b"Final result: %d\n\0".as_ptr() as *const c_char,
            result,
        );

        destroy_state(state);

        result
    }
}
