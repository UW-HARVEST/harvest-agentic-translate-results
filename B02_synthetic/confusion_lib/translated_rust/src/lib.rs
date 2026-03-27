// Rust translation of c_src/src/lib.c — byte-identical output.

use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn snprintf(buf: *mut u8, size: usize, fmt: *const u8, ...) -> c_int;
    fn memchr(s: *const u8, c: c_int, n: usize) -> *mut u8;
    fn strlen(s: *const u8) -> usize;
}

// ---------- Bitfield helpers (PackedFlags stored as u32) ----------
// Layout (LSB first, matching typical C unsigned-int bitfield on little-endian):
//   flag1    : 1  (bit 0)
//   flag2    : 1  (bit 1)
//   flag3    : 1  (bit 2)
//   counter  : 5  (bits 3-7)
//   mode     : 3  (bits 8-10)
//   status   : 5  (bits 11-15)
//   reserved : 16 (bits 16-31)

#[repr(C)]
struct PackedFlags(u32);

impl PackedFlags {
    fn get_flag1(&self) -> u32 { self.0 & 1 }
    fn set_flag1(&mut self, v: u32) { self.0 = (self.0 & !1) | (v & 1); }
    fn get_flag2(&self) -> u32 { (self.0 >> 1) & 1 }
    fn set_flag2(&mut self, v: u32) { self.0 = (self.0 & !(1 << 1)) | ((v & 1) << 1); }
    fn get_flag3(&self) -> u32 { (self.0 >> 2) & 1 }
    fn set_flag3(&mut self, v: u32) { self.0 = (self.0 & !(1 << 2)) | ((v & 1) << 2); }
    fn get_counter(&self) -> u32 { (self.0 >> 3) & 0x1F }
    fn set_counter(&mut self, v: u32) { self.0 = (self.0 & !(0x1F << 3)) | ((v & 0x1F) << 3); }
    fn get_mode(&self) -> u32 { (self.0 >> 8) & 0x7 }
    fn set_mode(&mut self, v: u32) { self.0 = (self.0 & !(0x7 << 8)) | ((v & 0x7) << 8); }
    fn set_status(&mut self, v: u32) { self.0 = (self.0 & !(0x1F << 11)) | ((v & 0x1F) << 11); }
    fn set_reserved(&mut self, v: u32) { self.0 = (self.0 & !(0xFFFF << 16)) | ((v & 0xFFFF) << 16); }
}

// ---------- TypeConfusion union ----------
#[repr(C)]
union TypeConfusion {
    int_val: i32,
    float_val: f32,
    uint_val: u32,
    bytes: [i8; 4],
}

// ---------- ProcessState ----------
#[repr(C)]
struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut u8,
    capacity: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = malloc(std::mem::size_of::<ProcessState>()) as *mut ProcessState;
    if state.is_null() {
        printf(b"Error: Failed to allocate memory for state\n\0".as_ptr());
        return std::ptr::null_mut();
    }

    (*state).flags = PackedFlags(0);
    (*state).flags.set_flag1(1);
    (*state).flags.set_flag2(0);
    (*state).flags.set_flag3(1);
    (*state).flags.set_counter(0);
    (*state).flags.set_mode(3);
    (*state).flags.set_status(15);
    (*state).flags.set_reserved(0);

    (*state).data.int_val = initial_val;
    (*state).capacity = capacity;

    let buf = malloc(capacity as usize);
    (*state).buffer = buf;
    if buf.is_null() {
        printf(b"Error: Failed to allocate buffer\n\0".as_ptr());
        free(state as *mut u8);
        return std::ptr::null_mut();
    }

    snprintf(
        buf,
        capacity as usize,
        b"State:%d:Mode:%d\0".as_ptr(),
        initial_val,
        (*state).flags.get_mode() as c_int,
    );

    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        if !(*state).buffer.is_null() {
            free((*state).buffer);
        }
        free(state as *mut u8);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: u8) -> c_int {
    if state.is_null() || (*state).buffer.is_null() {
        printf(b"Error: Null pointer in process_buffer\n\0".as_ptr());
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr = (*state).buffer;
    let mut remaining = strlen(ptr);

    while remaining > 0 {
        let found = memchr(ptr, target as c_int, remaining);
        if found.is_null() {
            break;
        }
        count += 1;
        // LOG_OPERATION(memchr_found, count) expands to:
        // printf("Operation: memchr_found with value %d\n", count)
        printf(
            b"Operation: memchr_found with value %d\n\0".as_ptr(),
            count,
        );
        let offset = (found as usize) - (ptr as usize) + 1;
        remaining -= offset;
        ptr = found.add(1);
    }

    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    let new_counter = ((*state).flags.get_counter() + 1) & 0x1F;
    (*state).flags.set_counter(new_counter);
    (*state).flags.set_flag1((param as u32) & 1);
    (*state).flags.set_flag2(((param as u32) & 2) >> 1);
    (*state).flags.set_flag3(((param as u32) & 4) >> 2);
    (*state).flags.set_mode(((param as u32) >> 3) & 0x7);

    // DEBUG_VAR(state->flags.counter) => printf("Debug: state->flags.counter = %d\n", ...)
    printf(
        b"Debug: state->flags.counter = %d\n\0".as_ptr(),
        (*state).flags.get_counter() as c_int,
    );
    printf(
        b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0".as_ptr(),
        (*state).flags.get_flag1() as c_int,
        (*state).flags.get_flag2() as c_int,
        (*state).flags.get_flag3() as c_int,
        (*state).flags.get_mode() as c_int,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    match operation {
        0 => {
            (*state).data.int_val = 1078530011;
            printf(
                b"Set as int: %d\n\0".as_ptr(),
                (*state).data.int_val,
            );
        }
        1 => {
            printf(
                b"Read as float: %f\n\0".as_ptr(),
                (*state).data.float_val as f64,
            );
            result = ((*state).data.float_val * 100.0).to_int_unchecked::<c_int>();
        }
        2 => {
            printf(
                b"Read as uint: %u\n\0".as_ptr(),
                (*state).data.uint_val,
            );
            result = ((*state).data.uint_val & 0xFF) as c_int;
        }
        3 => {
            printf(
                b"Read as bytes: [%d, %d, %d, %d]\n\0".as_ptr(),
                (*state).data.bytes[0] as c_int,
                (*state).data.bytes[1] as c_int,
                (*state).data.bytes[2] as c_int,
                (*state).data.bytes[3] as c_int,
            );
            result = ((*state).data.bytes[0] as c_int) + ((*state).data.bytes[1] as c_int);
        }
        _ => {}
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    // DEBUG_VAR(param1) => printf("Debug: param1 = %d\n", param1)
    printf(b"Debug: param1 = %d\n\0".as_ptr(), param1);
    printf(b"Debug: param2 = %d\n\0".as_ptr(), param2);
    printf(b"Debug: param3 = %d\n\0".as_ptr(), param3);
    printf(b"Debug: param4 = %d\n\0".as_ptr(), param4);

    let mut result: c_int = 0;

    let state = create_state(param1, 128);
    if state.is_null() {
        return -1;
    }

    update_flags(state, param2);

    let search_char = (b'0' as c_int + (param3 % 10)) as u8;
    let found_count = process_buffer(state, search_char);
    result += found_count * 10;

    let confusion_result = confuse_types(state, param4 % 4);
    result += confusion_result;

    result += (*state).flags.get_counter() as c_int * 5;
    result += (*state).flags.get_mode() as c_int * 3;

    printf(b"Final result: %d\n\0".as_ptr(), result);

    destroy_state(state);

    result
}
