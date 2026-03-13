use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn snprintf(buf: *mut u8, size: usize, fmt: *const u8, ...) -> c_int;
    fn strlen(s: *const u8) -> usize;
    fn memchr(s: *const u8, c: c_int, n: usize) -> *mut u8;
}

// PackedFlags as a u32 bitfield (matches C struct layout on little-endian):
//   flag1    : 1 bit  (bit 0)
//   flag2    : 1 bit  (bit 1)
//   flag3    : 1 bit  (bit 2)
//   counter  : 5 bits (bits 3-7)
//   mode     : 3 bits (bits 8-10)
//   status   : 5 bits (bits 11-15)
//   reserved : 16 bits (bits 16-31)
#[repr(C)]
#[derive(Clone, Copy)]
struct PackedFlags {
    bits: u32,
}

impl PackedFlags {
    fn flag1(&self) -> u32 { self.bits & 1 }
    fn set_flag1(&mut self, v: u32) { self.bits = (self.bits & !1) | (v & 1); }

    fn flag2(&self) -> u32 { (self.bits >> 1) & 1 }
    fn set_flag2(&mut self, v: u32) { self.bits = (self.bits & !(1 << 1)) | ((v & 1) << 1); }

    fn flag3(&self) -> u32 { (self.bits >> 2) & 1 }
    fn set_flag3(&mut self, v: u32) { self.bits = (self.bits & !(1 << 2)) | ((v & 1) << 2); }

    fn counter(&self) -> u32 { (self.bits >> 3) & 0x1F }
    fn set_counter(&mut self, v: u32) { self.bits = (self.bits & !(0x1F << 3)) | ((v & 0x1F) << 3); }

    fn mode(&self) -> u32 { (self.bits >> 8) & 0x7 }
    fn set_mode(&mut self, v: u32) { self.bits = (self.bits & !(0x7 << 8)) | ((v & 0x7) << 8); }

    fn status(&self) -> u32 { (self.bits >> 11) & 0x1F }
    fn set_status(&mut self, v: u32) { self.bits = (self.bits & !(0x1F << 11)) | ((v & 0x1F) << 11); }
}

#[repr(C)]
union TypeConfusion {
    int_val: i32,
    float_val: f32,
    uint_val: u32,
    bytes: [i8; 4],
}

#[repr(C)]
struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut u8,
    capacity: c_int,
}

unsafe fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = malloc(std::mem::size_of::<ProcessState>()) as *mut ProcessState;

    if state.is_null() {
        printf(b"Error: Failed to allocate memory for state\n\0".as_ptr());
        return std::ptr::null_mut();
    }

    (*state).flags = PackedFlags { bits: 0 };
    (*state).flags.set_flag1(1);
    (*state).flags.set_flag2(0);
    (*state).flags.set_flag3(1);
    (*state).flags.set_counter(0);
    (*state).flags.set_mode(3);
    (*state).flags.set_status(15);

    (*state).data.int_val = initial_val;

    (*state).capacity = capacity;
    (*state).buffer = malloc(capacity as usize);

    if (*state).buffer.is_null() {
        printf(b"Error: Failed to allocate buffer\n\0".as_ptr());
        free(state as *mut u8);
        return std::ptr::null_mut();
    }

    snprintf(
        (*state).buffer,
        capacity as usize,
        b"State:%d:Mode:%d\0".as_ptr(),
        initial_val,
        (*state).flags.mode() as c_int,
    );

    state
}

unsafe fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        if !(*state).buffer.is_null() {
            free((*state).buffer);
        }
        free(state as *mut u8);
    }
}

unsafe fn process_buffer(state: *mut ProcessState, target: u8) -> c_int {
    if state.is_null() || (*state).buffer.is_null() {
        printf(b"Error: Null pointer in process_buffer\n\0".as_ptr());
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr = (*state).buffer;
    let mut remaining = strlen((*state).buffer);

    while remaining > 0 {
        let found = memchr(ptr, target as c_int, remaining);

        if found.is_null() {
            break;
        }

        count += 1;
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

unsafe fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    (*state).flags.set_counter(((*state).flags.counter().wrapping_add(1)) & 0x1F);
    (*state).flags.set_flag1((param & 1) as u32);
    (*state).flags.set_flag2(((param & 2) >> 1) as u32);
    (*state).flags.set_flag3(((param & 4) >> 2) as u32);
    (*state).flags.set_mode(((param >> 3) & 0x7) as u32);

    printf(
        b"Debug: state->flags.counter = %d\n\0".as_ptr(),
        (*state).flags.counter() as c_int,
    );
    printf(
        b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0".as_ptr(),
        (*state).flags.flag1() as c_int,
        (*state).flags.flag2() as c_int,
        (*state).flags.flag3() as c_int,
        (*state).flags.mode() as c_int,
    );
}

unsafe fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
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
            result = ((*state).data.float_val * 100.0) as c_int;
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
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
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

        result += (*state).flags.counter() as c_int * 5;
        result += (*state).flags.mode() as c_int * 3;

        printf(b"Final result: %d\n\0".as_ptr(), result);

        destroy_state(state);

        result
    }
}
