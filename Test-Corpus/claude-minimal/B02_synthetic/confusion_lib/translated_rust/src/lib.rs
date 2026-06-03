// Translation of c_src/src/lib.c to Rust
// Preserves the externally visible `confusion(a, b, c, d) -> int` interface.

use std::ffi::c_int;

// PackedFlags reproduces the bit-field semantics from the C struct.
// Layout in the C type (in declaration order):
//   flag1   : 1
//   flag2   : 1
//   flag3   : 1
//   counter : 5
//   mode    : 3
//   status  : 5
//   reserved: 16
// In the C code only flag1, flag2, flag3, counter, and mode are read after
// being written, so reproducing those semantics faithfully is sufficient.
#[derive(Default, Copy, Clone)]
struct PackedFlags {
    flag1: u32,    // 1 bit
    flag2: u32,    // 1 bit
    flag3: u32,    // 1 bit
    counter: u32,  // 5 bits
    mode: u32,     // 3 bits
    status: u32,   // 5 bits
    reserved: u32, // 16 bits
}

impl PackedFlags {
    fn set_flag1(&mut self, v: u32) {
        self.flag1 = v & 0x1;
    }
    fn set_flag2(&mut self, v: u32) {
        self.flag2 = v & 0x1;
    }
    fn set_flag3(&mut self, v: u32) {
        self.flag3 = v & 0x1;
    }
    fn set_counter(&mut self, v: u32) {
        self.counter = v & 0x1F;
    }
    fn set_mode(&mut self, v: u32) {
        self.mode = v & 0x7;
    }
    fn set_status(&mut self, v: u32) {
        self.status = v & 0x1F;
    }
    fn set_reserved(&mut self, v: u32) {
        self.reserved = v & 0xFFFF;
    }
}

// Replicates the C union, where four fields share the same 4-byte storage.
#[repr(C)]
#[derive(Copy, Clone)]
union TypeConfusion {
    int_val: i32,
    float_val: f32,
    uint_val: u32,
    bytes: [u8; 4],
}

impl Default for TypeConfusion {
    fn default() -> Self {
        TypeConfusion { int_val: 0 }
    }
}

struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: Option<Vec<u8>>, // mirrors C's malloc'd char* buffer
    capacity: i32,
}

fn create_state(initial_val: i32, capacity: i32) -> Option<Box<ProcessState>> {
    let mut state = Box::new(ProcessState {
        flags: PackedFlags::default(),
        data: TypeConfusion::default(),
        buffer: None,
        capacity: 0,
    });

    state.flags.set_flag1(1);
    state.flags.set_flag2(0);
    state.flags.set_flag3(1);
    state.flags.set_counter(0);
    state.flags.set_mode(3);
    state.flags.set_status(15);
    state.flags.set_reserved(0);

    state.data.int_val = initial_val;

    state.capacity = capacity;

    if capacity <= 0 {
        println!("Error: Failed to allocate buffer");
        return None;
    }

    // Build the formatted string the same way snprintf would, then truncate
    // to capacity-1 bytes plus a NUL terminator (matching snprintf semantics).
    let formatted = format!("State:{}:Mode:{}", initial_val, state.flags.mode);
    let max_content = (capacity as usize).saturating_sub(1);
    let truncated = if formatted.len() > max_content {
        &formatted.as_bytes()[..max_content]
    } else {
        formatted.as_bytes()
    };

    let mut buffer = vec![0u8; capacity as usize];
    buffer[..truncated.len()].copy_from_slice(truncated);
    // Ensure NUL terminator (already 0 from initialization, but explicit).
    buffer[truncated.len()] = 0;

    state.buffer = Some(buffer);

    Some(state)
}

fn destroy_state(state: Option<Box<ProcessState>>) {
    // Dropping the Box releases all owned memory, mirroring free().
    drop(state);
}

// Computes the C strlen of the buffer (length up to but not including the
// first NUL byte).
fn buffer_strlen(buffer: &[u8]) -> usize {
    buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len())
}

fn process_buffer(state: Option<&ProcessState>, target: u8) -> i32 {
    let state = match state {
        Some(s) => s,
        None => {
            println!("Error: Null pointer in process_buffer");
            return -1;
        }
    };

    let buffer = match &state.buffer {
        Some(b) => b,
        None => {
            println!("Error: Null pointer in process_buffer");
            return -1;
        }
    };

    let mut count: i32 = 0;
    let mut start: usize = 0;
    let mut remaining = buffer_strlen(buffer);

    while remaining > 0 {
        let slice = &buffer[start..start + remaining];
        match slice.iter().position(|&b| b == target) {
            Some(idx) => {
                count += 1;
                println!("Operation: memchr_found with value {}", count);
                let consumed = idx + 1;
                remaining -= consumed;
                start += consumed;
            }
            None => break,
        }
    }

    count
}

fn update_flags(state: Option<&mut ProcessState>, param: i32) {
    let state = match state {
        Some(s) => s,
        None => return,
    };

    let new_counter = (state.flags.counter + 1) & 0x1F;
    state.flags.set_counter(new_counter);
    state.flags.set_flag1((param & 1) as u32);
    state.flags.set_flag2(((param & 2) >> 1) as u32);
    state.flags.set_flag3(((param & 4) >> 2) as u32);
    state.flags.set_mode(((param >> 3) & 0x7) as u32);

    println!("Debug: state->flags.counter = {}", state.flags.counter);
    println!(
        "Bit fields - flag1:{} flag2:{} flag3:{} mode:{}",
        state.flags.flag1, state.flags.flag2, state.flags.flag3, state.flags.mode
    );
}

fn confuse_types(state: Option<&mut ProcessState>, operation: i32) -> i32 {
    let state = match state {
        Some(s) => s,
        None => return 0,
    };

    let mut result: i32 = 0;

    // SAFETY: each branch reads only the union field that is well-defined for
    // the chosen operation. The case `operation == 0` writes int_val first and
    // does not read any other field afterward. Reading other variants relies on
    // the same in-memory representation as the original C union.
    unsafe {
        match operation {
            0 => {
                state.data.int_val = 1_078_530_011;
                println!("Set as int: {}", state.data.int_val);
            }
            1 => {
                let f = state.data.float_val;
                // Match C's printf %f default of 6 decimal places.
                println!("Read as float: {:.6}", f);
                result = (f * 100.0) as i32;
            }
            2 => {
                let u = state.data.uint_val;
                println!("Read as uint: {}", u);
                result = (u & 0xFF) as i32;
            }
            3 => {
                // C reads the bytes as `char` (signed on most platforms),
                // promoted to int via printf("%d"). Mirror that by treating
                // the bytes as i8.
                let b = state.data.bytes;
                let b0 = b[0] as i8 as i32;
                let b1 = b[1] as i8 as i32;
                let b2 = b[2] as i8 as i32;
                let b3 = b[3] as i8 as i32;
                println!("Read as bytes: [{}, {}, {}, {}]", b0, b1, b2, b3);
                result = b0 + b1;
            }
            _ => {}
        }
    }

    result
}

#[no_mangle]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    println!("Debug: param1 = {}", param1);
    println!("Debug: param2 = {}", param2);
    println!("Debug: param3 = {}", param3);
    println!("Debug: param4 = {}", param4);

    let mut result: i32 = 0;

    let mut state = match create_state(param1, 128) {
        Some(s) => s,
        None => return -1,
    };

    update_flags(Some(&mut state), param2);

    // Reproduce C's `'0' + (param3 % 10)` (signed remainder semantics).
    let search_char = (b'0' as i32 + (param3 % 10)) as u8;
    let found_count = process_buffer(Some(&state), search_char);
    result += found_count * 10;

    let confusion_result = confuse_types(Some(&mut state), param4 % 4);
    result += confusion_result;

    result += (state.flags.counter as i32) * 5;
    result += (state.flags.mode as i32) * 3;

    println!("Final result: {}", result);

    destroy_state(Some(state));

    result as c_int
}
