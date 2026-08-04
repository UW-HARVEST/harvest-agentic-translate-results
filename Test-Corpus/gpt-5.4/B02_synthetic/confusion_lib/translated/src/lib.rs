use std::ffi::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct PackedFlags {
    bits: u32,
}

impl PackedFlags {
    fn new() -> Self {
        Self { bits: 0 }
    }

    fn flag1(&self) -> u32 {
        self.bits & 0x1
    }

    fn flag2(&self) -> u32 {
        (self.bits >> 1) & 0x1
    }

    fn flag3(&self) -> u32 {
        (self.bits >> 2) & 0x1
    }

    fn counter(&self) -> u32 {
        (self.bits >> 3) & 0x1f
    }

    fn mode(&self) -> u32 {
        (self.bits >> 8) & 0x7
    }

    fn set_flag1(&mut self, value: u32) {
        self.bits = (self.bits & !0x1) | (value & 0x1);
    }

    fn set_flag2(&mut self, value: u32) {
        self.bits = (self.bits & !(0x1 << 1)) | ((value & 0x1) << 1);
    }

    fn set_flag3(&mut self, value: u32) {
        self.bits = (self.bits & !(0x1 << 2)) | ((value & 0x1) << 2);
    }

    fn set_counter(&mut self, value: u32) {
        self.bits = (self.bits & !(0x1f << 3)) | ((value & 0x1f) << 3);
    }

    fn set_mode(&mut self, value: u32) {
        self.bits = (self.bits & !(0x7 << 8)) | ((value & 0x7) << 8);
    }

    fn set_status(&mut self, value: u32) {
        self.bits = (self.bits & !(0x1f << 11)) | ((value & 0x1f) << 11);
    }

    fn set_reserved(&mut self, value: u32) {
        self.bits = (self.bits & !(0xffff << 16)) | ((value & 0xffff) << 16);
    }
}

#[derive(Clone)]
struct TypeConfusion {
    bytes: [u8; 4],
}

impl TypeConfusion {
    fn from_int(value: i32) -> Self {
        Self {
            bytes: value.to_ne_bytes(),
        }
    }

    fn set_int(&mut self, value: i32) {
        self.bytes = value.to_ne_bytes();
    }

    fn int_val(&self) -> i32 {
        i32::from_ne_bytes(self.bytes)
    }

    fn float_val(&self) -> f32 {
        f32::from_ne_bytes(self.bytes)
    }

    fn uint_val(&self) -> u32 {
        u32::from_ne_bytes(self.bytes)
    }

    fn bytes(&self) -> [u8; 4] {
        self.bytes
    }
}

struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: Vec<u8>,
    capacity: usize,
}

fn create_state(initial_val: i32, capacity: i32) -> Option<ProcessState> {
    if capacity <= 0 {
        println!("Error: Failed to allocate buffer");
        return None;
    }

    let mut flags = PackedFlags::new();
    flags.set_flag1(1);
    flags.set_flag2(0);
    flags.set_flag3(1);
    flags.set_counter(0);
    flags.set_mode(3);
    flags.set_status(15);
    flags.set_reserved(0);

    let text = format!("State:{}:Mode:{}", initial_val, flags.mode());
    let cap = capacity as usize;
    let mut buffer = vec![0u8; cap];
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(cap.saturating_sub(1));
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);

    Some(ProcessState {
        flags,
        data: TypeConfusion::from_int(initial_val),
        buffer,
        capacity: cap,
    })
}

fn process_buffer(state: &ProcessState, target: u8) -> i32 {
    let len = state
        .buffer
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(state.buffer.len());
    let mut count = 0;
    let mut start = 0;
    let slice = &state.buffer[..len];

    while start < slice.len() {
        if let Some(pos) = slice[start..].iter().position(|&b| b == target) {
            count += 1;
            println!("Operation: memchr_found with value {}", count);
            start += pos + 1;
        } else {
            break;
        }
    }

    count
}

fn update_flags(state: &mut ProcessState, param: i32) {
    let counter = (state.flags.counter() + 1) & 0x1f;
    state.flags.set_counter(counter);
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

fn confuse_types(state: &mut ProcessState, operation: i32) -> i32 {
    let mut result = 0;

    match operation {
        0 => {
            state.data.set_int(1078530011);
            println!("Set as int: {}", state.data.int_val());
        }
        1 => {
            println!("Read as float: {}", state.data.float_val());
            result = (state.data.float_val() * 100.0) as i32;
        }
        2 => {
            println!("Read as uint: {}", state.data.uint_val());
            result = (state.data.uint_val() & 0xff) as i32;
        }
        3 => {
            let bytes = state.data.bytes();
            let b0 = bytes[0] as i8 as i32;
            let b1 = bytes[1] as i8 as i32;
            let b2 = bytes[2] as i8 as i32;
            let b3 = bytes[3] as i8 as i32;
            println!("Read as bytes: [{}, {}, {}, {}]", b0, b1, b2, b3);
            result = b0 + b1;
        }
        _ => {}
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    println!("Debug: param1 = {}", param1);
    println!("Debug: param2 = {}", param2);
    println!("Debug: param3 = {}", param3);
    println!("Debug: param4 = {}", param4);

    let mut result = 0i32;

    let Some(mut state) = create_state(param1, 128) else {
        return -1;
    };

    update_flags(&mut state, param2);

    let digit = param3.rem_euclid(10) as u8;
    let search_char = b'0' + digit;
    let found_count = process_buffer(&state, search_char);
    result += found_count * 10;

    let confusion_result = confuse_types(&mut state, param4.rem_euclid(4));
    result += confusion_result;

    result += state.flags.counter() as i32 * 5;
    result += state.flags.mode() as i32 * 3;

    let _ = state.capacity;

    println!("Final result: {}", result);

    result as c_int
}
