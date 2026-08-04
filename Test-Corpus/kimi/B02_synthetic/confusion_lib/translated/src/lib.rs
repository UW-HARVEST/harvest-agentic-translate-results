use std::ffi::{c_char, c_int, CStr};
use std::os::raw::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct PackedFlags {
    bits: u32,
}

impl PackedFlags {
    fn flag1(&self) -> u32 {
        self.bits & 0x1
    }

    fn set_flag1(&mut self, val: u32) {
        self.bits = (self.bits & !0x1) | (val & 0x1);
    }

    fn flag2(&self) -> u32 {
        (self.bits >> 1) & 0x1
    }

    fn set_flag2(&mut self, val: u32) {
        self.bits = (self.bits & !0x2) | ((val & 0x1) << 1);
    }

    fn flag3(&self) -> u32 {
        (self.bits >> 2) & 0x1
    }

    fn set_flag3(&mut self, val: u32) {
        self.bits = (self.bits & !0x4) | ((val & 0x1) << 2);
    }

    fn counter(&self) -> u32 {
        (self.bits >> 3) & 0x1F
    }

    fn set_counter(&mut self, val: u32) {
        self.bits = (self.bits & !0xF8) | ((val & 0x1F) << 3);
    }

    fn mode(&self) -> u32 {
        (self.bits >> 8) & 0x7
    }

    fn set_mode(&mut self, val: u32) {
        self.bits = (self.bits & !0x700) | ((val & 0x7) << 8);
    }

    fn status(&self) -> u32 {
        (self.bits >> 11) & 0x1F
    }

    fn set_status(&mut self, val: u32) {
        self.bits = (self.bits & !0xF800) | ((val & 0x1F) << 11);
    }

    fn reserved(&self) -> u32 {
        self.bits >> 16
    }

    fn set_reserved(&mut self, val: u32) {
        self.bits = (self.bits & 0xFFFF) | (val << 16);
    }
}

#[repr(C)]
union TypeConfusion {
    int_val: i32,
    float_val: f32,
    uint_val: u32,
    bytes: [u8; 4],
}

impl Clone for TypeConfusion {
    fn clone(&self) -> Self {
        unsafe {
            Self { uint_val: self.uint_val }
        }
    }
}

impl Copy for TypeConfusion {}

struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: Vec<u8>,
    capacity: usize,
}

fn create_state(initial_val: i32, capacity: i32) -> Option<Box<ProcessState>> {
    if capacity <= 0 {
        eprintln!("Error: Invalid capacity");
        return None;
    }

    let cap = capacity as usize;
    let mut buffer = vec![0u8; cap];
    
    let formatted = format!("State:{}:Mode:{}", initial_val, 3);
    let bytes = formatted.as_bytes();
    let len = bytes.len().min(cap - 1);
    buffer[..len].copy_from_slice(&bytes[..len]);
    buffer[len] = 0;

    let mut flags = PackedFlags { bits: 0 };
    flags.set_flag1(1);
    flags.set_flag2(0);
    flags.set_flag3(1);
    flags.set_counter(0);
    flags.set_mode(3);
    flags.set_status(15);
    flags.set_reserved(0);

    Some(Box::new(ProcessState {
        flags,
        data: TypeConfusion { int_val: initial_val },
        buffer,
        capacity: cap,
    }))
}

fn process_buffer(state: &ProcessState, target: u8) -> i32 {
    let c_str = match CStr::from_bytes_until_nul(&state.buffer) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: Invalid buffer in process_buffer");
            return -1;
        }
    };

    let bytes = c_str.to_bytes();
    bytes.iter().filter(|&&b| b == target).count() as i32
}

fn update_flags(state: &mut ProcessState, param: i32) {
    let new_counter = (state.flags.counter() + 1) & 0x1F;
    state.flags.set_counter(new_counter);
    state.flags.set_flag1((param as u32) & 1);
    state.flags.set_flag2(((param as u32) & 2) >> 1);
    state.flags.set_flag3(((param as u32) & 4) >> 2);
    state.flags.set_mode(((param as u32) >> 3) & 0x7);

    eprintln!("Debug: state->flags.counter = {}", state.flags.counter());
    eprintln!(
        "Bit fields - flag1:{} flag2:{} flag3:{} mode:{}",
        state.flags.flag1(),
        state.flags.flag2(),
        state.flags.flag3(),
        state.flags.mode()
    );
}

fn confuse_types(state: &mut ProcessState, operation: i32) -> i32 {
    let mut result = 0;

    match operation {
        0 => {
            unsafe {
                state.data.int_val = 1078530011;
                eprintln!("Set as int: {}", state.data.int_val);
            }
        }
        1 => {
            unsafe {
                eprintln!("Read as float: {}", state.data.float_val);
                result = (state.data.float_val * 100.0) as i32;
            }
        }
        2 => {
            unsafe {
                eprintln!("Read as uint: {}", state.data.uint_val);
                result = (state.data.uint_val & 0xFF) as i32;
            }
        }
        3 => {
            unsafe {
                eprintln!(
                    "Read as bytes: [{}, {}, {}, {}]",
                    state.data.bytes[0],
                    state.data.bytes[1],
                    state.data.bytes[2],
                    state.data.bytes[3]
                );
                result = (state.data.bytes[0] as i32) + (state.data.bytes[1] as i32);
            }
        }
        _ => {}
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    eprintln!("Debug: param1 = {}", param1);
    eprintln!("Debug: param2 = {}", param2);
    eprintln!("Debug: param3 = {}", param3);
    eprintln!("Debug: param4 = {}", param4);

    let mut result = 0;

    let mut state = match create_state(param1, 128) {
        Some(s) => s,
        None => return -1,
    };

    update_flags(&mut state, param2);

    let search_char = b'0' + ((param3 % 10).abs() as u8 % 10);
    let found_count = process_buffer(&state, search_char);
    result += found_count * 10;

    let confusion_result = confuse_types(&mut state, (param4 % 4).abs());
    result += confusion_result;

    result += (state.flags.counter() * 5) as i32;
    result += (state.flags.mode() * 3) as i32;

    eprintln!("Final result: {}", result);

    result
}
