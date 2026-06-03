// Translated to Rust from c_src/src/lib.c
// Reproduces the original C library behavior, including identical stdout output.

use std::ffi::c_int;

#[derive(Default, Copy, Clone)]
struct PackedFlags {
    flag1: u32,    // 1 bit
    flag2: u32,    // 1 bit
    flag3: u32,    // 1 bit
    counter: u32,  // 5 bits
    mode: u32,     // 3 bits
    status: u32,   // 5 bits
    #[allow(dead_code)]
    reserved: u32, // 16 bits
}

#[repr(C)]
#[derive(Copy, Clone)]
union TypeConfusion {
    int_val: i32,
    float_val: f32,
    uint_val: u32,
    bytes: [i8; 4],
}

impl Default for TypeConfusion {
    fn default() -> Self {
        TypeConfusion { int_val: 0 }
    }
}

struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: Option<Vec<u8>>, // null-terminated buffer (capacity bytes total)
    capacity: i32,
}

/// Mimic snprintf: write up to `capacity` bytes (including the terminating NUL)
/// of `s` into a buffer of length `capacity`.  Returns the resulting buffer.
fn snprintf_buffer(capacity: i32, s: &str) -> Vec<u8> {
    let cap = if capacity < 0 { 0 } else { capacity as usize };
    let mut buf = vec![0u8; cap];
    if cap == 0 {
        return buf;
    }
    let bytes = s.as_bytes();
    let n = std::cmp::min(bytes.len(), cap - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0; // NUL terminator
    buf
}

/// Length of the C string in `buf` (bytes up to but not including the NUL).
fn cstrlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn create_state(initial_val: i32, capacity: i32) -> Option<Box<ProcessState>> {
    let mut state = Box::new(ProcessState {
        flags: PackedFlags::default(),
        data: TypeConfusion::default(),
        buffer: None,
        capacity: 0,
    });

    // Initial bit-field values.
    state.flags.flag1 = 1 & 0x1;
    state.flags.flag2 = 0 & 0x1;
    state.flags.flag3 = 1 & 0x1;
    state.flags.counter = 0 & 0x1F;
    state.flags.mode = 3 & 0x7;
    state.flags.status = 15 & 0x1F;
    state.flags.reserved = 0 & 0xFFFF;

    state.data = TypeConfusion { int_val: initial_val };

    state.capacity = capacity;

    // Allocate buffer of `capacity` bytes (matches the C malloc).
    // If capacity is non-positive we still go through the snprintf path,
    // matching C's behavior with a zero-length buffer.
    let formatted = format!("State:{}:Mode:{}", initial_val, state.flags.mode);
    state.buffer = Some(snprintf_buffer(capacity, &formatted));

    Some(state)
}

fn destroy_state(_state: Box<ProcessState>) {
    // Box drop releases memory; matches free() of state and buffer.
}

fn process_buffer(state: &mut ProcessState, target: u8) -> i32 {
    let buf = match state.buffer.as_ref() {
        Some(b) => b,
        None => {
            println!("Error: Null pointer in process_buffer");
            return -1;
        }
    };

    let mut count: i32 = 0;
    let len = cstrlen(buf);
    let mut idx: usize = 0;
    let mut remaining: usize = len;

    while remaining > 0 {
        let slice = &buf[idx..idx + remaining];
        match slice.iter().position(|&b| b == target) {
            None => break,
            Some(pos) => {
                count += 1;
                println!("Operation: memchr_found with value {}", count);
                remaining -= pos + 1;
                idx += pos + 1;
            }
        }
    }

    count
}

fn update_flags(state: &mut ProcessState, param: i32) {
    state.flags.counter = (state.flags.counter.wrapping_add(1)) & 0x1F;
    state.flags.flag1 = ((param & 1) as u32) & 0x1;
    state.flags.flag2 = (((param & 2) >> 1) as u32) & 0x1;
    state.flags.flag3 = (((param & 4) >> 2) as u32) & 0x1;
    state.flags.mode = (((param >> 3) & 0x7) as u32) & 0x7;

    println!("Debug: state->flags.counter = {}", state.flags.counter);
    println!(
        "Bit fields - flag1:{} flag2:{} flag3:{} mode:{}",
        state.flags.flag1, state.flags.flag2, state.flags.flag3, state.flags.mode
    );
}

fn confuse_types(state: &mut ProcessState, operation: i32) -> i32 {
    let mut result: i32 = 0;

    match operation {
        0 => {
            state.data = TypeConfusion { int_val: 1078530011 };
            // SAFETY: just wrote int_val.
            let v = unsafe { state.data.int_val };
            println!("Set as int: {}", v);
        }
        1 => {
            // SAFETY: union access; reproduces C's type-confusion read.
            let f = unsafe { state.data.float_val };
            println!("Read as float: {}", format_c_double(f as f64));
            result = (f * 100.0) as i32;
        }
        2 => {
            // SAFETY: union access; reproduces C's type-confusion read.
            let u = unsafe { state.data.uint_val };
            println!("Read as uint: {}", u);
            result = (u & 0xFF) as i32;
        }
        3 => {
            // SAFETY: union access; reproduces C's type-confusion read.
            let b = unsafe { state.data.bytes };
            println!(
                "Read as bytes: [{}, {}, {}, {}]",
                b[0] as i32, b[1] as i32, b[2] as i32, b[3] as i32
            );
            result = (b[0] as i32).wrapping_add(b[1] as i32);
        }
        _ => {}
    }

    result
}

/// Format a double the way C's `printf("%f", x)` does: 6 digits after
/// the decimal point with banker's-style rounding.  Rust's `{:.6}` on
/// `f64` already matches glibc's `%f` for the values produced by this
/// program.
fn format_c_double(x: f64) -> String {
    format!("{:.6}", x)
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    println!("Debug: param1 = {}", param1);
    println!("Debug: param2 = {}", param2);
    println!("Debug: param3 = {}", param3);
    println!("Debug: param4 = {}", param4);

    let mut result: i32 = 0;

    let state_opt = create_state(param1, 128);
    let mut state = match state_opt {
        Some(s) => s,
        None => return -1,
    };

    update_flags(&mut state, param2);

    let search_char = ((b'0' as i32).wrapping_add(param3 % 10)) as u8;
    let found_count = process_buffer(&mut state, search_char);
    result = result.wrapping_add(found_count.wrapping_mul(10));

    let confusion_result = confuse_types(&mut state, param4 % 4);
    result = result.wrapping_add(confusion_result);

    result = result.wrapping_add((state.flags.counter as i32).wrapping_mul(5));
    result = result.wrapping_add((state.flags.mode as i32).wrapping_mul(3));

    println!("Final result: {}", result);

    destroy_state(state);

    result
}
