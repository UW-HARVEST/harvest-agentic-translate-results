use std::os::raw::{c_char, c_int, c_uint};

struct PackedFlags {
    flag1: u8,
    flag2: u8,
    flag3: u8,
    counter: u8,
    mode: u8,
    _status: u8,
    _reserved: u16,
}

#[repr(C)]
union TypeConfusion {
    int_val: c_int,
    float_val: f32,
    uint_val: c_uint,
    bytes: [c_char; 4],
}

struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: String,
    _capacity: c_int,
}

fn create_state(initial_val: c_int, capacity: c_int) -> Option<ProcessState> {
    let flags = PackedFlags {
        flag1: 1,
        flag2: 0,
        flag3: 1,
        counter: 0,
        mode: 3,
        _status: 15,
        _reserved: 0,
    };

    let data = TypeConfusion { int_val: initial_val };

    let buffer = format!("State:{}:Mode:{}", initial_val, flags.mode);

    Some(ProcessState {
        flags,
        data,
        buffer,
        _capacity: capacity,
    })
}

fn process_buffer(state: &ProcessState, target: u8) -> c_int {
    let mut count = 0;
    for &b in state.buffer.as_bytes() {
        if b == target {
            count += 1;
            println!("Operation: \"memchr_found\" with value {}", count);
        }
    }
    count
}

fn update_flags(state: &mut ProcessState, param: c_int) {
    state.flags.counter = (state.flags.counter + 1) & 0x1F;
    state.flags.flag1 = (param & 1) as u8;
    state.flags.flag2 = ((param & 2) >> 1) as u8;
    state.flags.flag3 = ((param & 4) >> 2) as u8;
    state.flags.mode = ((param >> 3) & 0x7) as u8;

    println!("Debug: \"state->flags.counter\" = {}", state.flags.counter);
    println!("Bit fields - flag1:{} flag2:{} flag3:{} mode:{}",
             state.flags.flag1, state.flags.flag2,
             state.flags.flag3, state.flags.mode);
}

fn confuse_types(state: &mut ProcessState, operation: c_int) -> c_int {
    let mut result = 0;
    unsafe {
        match operation {
            0 => {
                state.data.int_val = 1078530011;
                println!("Set as int: {}", state.data.int_val);
            }
            1 => {
                println!("Read as float: {:.6}", state.data.float_val);
                result = (state.data.float_val * 100.0) as c_int;
            }
            2 => {
                println!("Read as uint: {}", state.data.uint_val);
                result = (state.data.uint_val & 0xFF) as c_int;
            }
            3 => {
                println!("Read as bytes: [{}, {}, {}, {}]",
                         state.data.bytes[0], state.data.bytes[1],
                         state.data.bytes[2], state.data.bytes[3]);
                result = (state.data.bytes[0] as c_int) + (state.data.bytes[1] as c_int);
            }
            _ => {}
        }
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    println!("Debug: \"param1\" = {}", param1);
    println!("Debug: \"param2\" = {}", param2);
    println!("Debug: \"param3\" = {}", param3);
    println!("Debug: \"param4\" = {}", param4);

    let mut result = 0;

    let mut state = match create_state(param1, 128) {
        Some(s) => s,
        None => return -1,
    };

    update_flags(&mut state, param2);

    let search_char = (b'0' as c_int + (param3 % 10)) as u8;
    let found_count = process_buffer(&state, search_char);
    result += found_count * 10;

    let confusion_result = confuse_types(&mut state, param4 % 4);
    result += confusion_result;

    result += (state.flags.counter as c_int) * 5;
    result += (state.flags.mode as c_int) * 3;

    println!("Final result: {}", result);

    result
}
