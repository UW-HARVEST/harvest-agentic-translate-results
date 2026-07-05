



use std::ptr;

use std::ffi::CStr;

use ::c2rust_bitfields;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memchr(
        __s: *const ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct PackedFlags {
    #[bitfield(name = "flag1", ty = "::core::ffi::c_uint", bits = "0..=0")]
    #[bitfield(name = "flag2", ty = "::core::ffi::c_uint", bits = "1..=1")]
    #[bitfield(name = "flag3", ty = "::core::ffi::c_uint", bits = "2..=2")]
    #[bitfield(name = "counter", ty = "::core::ffi::c_uint", bits = "3..=7")]
    #[bitfield(name = "mode", ty = "::core::ffi::c_uint", bits = "8..=10")]
    #[bitfield(name = "status", ty = "::core::ffi::c_uint", bits = "11..=15")]
    #[bitfield(name = "reserved", ty = "::core::ffi::c_uint", bits = "16..=31")]
    pub flag1_flag2_flag3_counter_mode_status_reserved: [u8; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union TypeConfusion {
    pub int_val: ::core::ffi::c_int,
    pub float_val: ::core::ffi::c_float,
    pub uint_val: ::core::ffi::c_uint,
    pub bytes: [::core::ffi::c_char; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ProcessState {
    pub flags: PackedFlags,
    pub data: TypeConfusion,
    pub buffer: *mut ::core::ffi::c_char,
    pub capacity: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn create_state(
    initial_val: ::core::ffi::c_int,
    capacity: ::core::ffi::c_int,
) -> *mut ProcessState {
    if capacity <= 0 {
        eprintln!("Error: Failed to allocate buffer");
        return ptr::null_mut();
    }

    let capacity_usize = capacity as usize;

    let mut state = Box::new(ProcessState {
        flags: unsafe { ::core::mem::zeroed() },
        data: unsafe { ::core::mem::zeroed() },
        capacity,
        buffer: ptr::null_mut(),
    });

    state.flags.set_flag1(1);
    state.flags.set_flag2(0);
    state.flags.set_flag3(1);
    state.flags.set_counter(0);
    state.flags.set_mode(3);
    state.flags.set_status(15);
    state.flags.set_reserved(0);

    state.data.int_val = initial_val;

    let text = format!("State:{}:Mode:{}", initial_val, state.flags.mode());
    let bytes = text.as_bytes();

    let mut buffer = vec![0u8; capacity_usize];
    let copy_len = bytes.len().min(capacity_usize.saturating_sub(1));
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);

    let boxed_slice = buffer.into_boxed_slice();
    state.buffer = Box::into_raw(boxed_slice) as *mut ::core::ffi::c_char;

    Box::into_raw(state)
}

#[no_mangle]
pub fn destroy_state(state: Option<Box<ProcessState>>) {
    drop(state);
}

#[no_mangle]
pub fn process_buffer(state: &ProcessState, target: ::core::ffi::c_char) -> ::core::ffi::c_int {
    if state.buffer.is_null() {
        eprintln!("Error: Null pointer in process_buffer");
        return -1;
    }

    let buffer = unsafe { ::std::ffi::CStr::from_ptr(state.buffer) };
    let bytes = buffer.to_bytes();
    let target_byte = target as u8;

    let mut count: ::core::ffi::c_int = 0;
    let mut start = 0usize;

    while start < bytes.len() {
        if let Some(pos) = bytes[start..].iter().position(|&b| b == target_byte) {
            count += 1;
            println!("Operation: memchr_found with value {}", count);
            start += pos + 1;
        } else {
            break;
        }
    }

    count
}

#[no_mangle]
pub fn update_flags(state: Option<&mut ProcessState>, param: i32) {
    if let Some(state) = state {
        let next_counter = ((state.flags.counter() as i32 + 1) & 0x1f) as u32;
        state.flags.set_counter(next_counter);
        state.flags.set_flag1((param & 1) as u32);
        state.flags.set_flag2(((param & 2) >> 1) as u32);
        state.flags.set_flag3(((param & 4) >> 2) as u32);
        state.flags.set_mode(((param >> 3) & 0x7) as u32);

        println!("Debug: state->flags.counter = {}", state.flags.counter());
        println!(
            "Bit fields - flag1:{} flag2:{} flag3:{} mode:{}",
            state.flags.flag1(),
            state.flags.flag2(),
            state.flags.flag3(),
            state.flags.mode()
        );
    }
}

#[no_mangle]
pub fn confuse_types(state: Option<&mut ProcessState>, operation: i32) -> i32 {
    let state = match state {
        Some(state) => state,
        None => return 0,
    };

    let mut result = 0;

    match operation {
        0 => {
            unsafe {
                state.data.int_val = 1078530011;
                let int_val = state.data.int_val;
                println!("Set as int: {}", int_val);
            }
        }
        1 => {
            unsafe {
                let float_val = state.data.float_val;
                println!("Read as float: {}", float_val);
                result = (float_val * 100.0) as i32;
            }
        }
        2 => {
            unsafe {
                let uint_val = state.data.uint_val;
                println!("Read as uint: {}", uint_val);
                result = (uint_val & 0xff) as i32;
            }
        }
        3 => {
            unsafe {
                let bytes = state.data.bytes;
                println!(
                    "Read as bytes: [{}, {}, {}, {}]",
                    bytes[0], bytes[1], bytes[2], bytes[3]
                );
                result = bytes[0] as i32 + bytes[1] as i32;
            }
        }
        _ => {}
    }

    result
}

#[no_mangle]
pub fn confusion(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    println!("Debug: param1 = {}", param1);
    println!("Debug: param2 = {}", param2);
    println!("Debug: param3 = {}", param3);
    println!("Debug: param4 = {}", param4);

    let mut result = 0;
    let state = create_state(param1, 128);

    if state.is_null() {
        return -1;
    }

    let mut state = Some(unsafe { Box::from_raw(state) });

    if let Some(ref mut state_ref) = state {
        update_flags(Some(state_ref.as_mut()), param2);

        let search_char = (b'0' + param3.rem_euclid(10) as u8) as ::core::ffi::c_char;
        let found_count = process_buffer(state_ref.as_ref(), search_char);
        result += found_count * 10;

        let confusion_result = confuse_types(Some(state_ref.as_mut()), param4.rem_euclid(4));
        result += confusion_result;

        result += state_ref.flags.counter() as i32 * 5;
        result += state_ref.flags.mode() as i32 * 3;
    }

    println!("Final result: {}", result);
    destroy_state(state);
    result
}

