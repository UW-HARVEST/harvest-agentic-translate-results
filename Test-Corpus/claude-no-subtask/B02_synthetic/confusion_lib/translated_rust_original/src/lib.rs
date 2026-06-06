// Translation of c_src/src/lib.c to Rust.
//
// Behavior must match the C version byte-for-byte on stdout, including
// printf-style number formatting and the order of writes.

use std::ffi::c_int;
use std::io::Write;

// PackedFlags only exposes counter (5 bits), mode (3 bits), and the boolean
// flags as observable values; we only need to model what the caller reads.
#[derive(Default, Clone, Copy)]
struct PackedFlags {
    flag1: u32,    // 1 bit
    flag2: u32,    // 1 bit
    flag3: u32,    // 1 bit
    counter: u32,  // 5 bits
    mode: u32,     // 3 bits
    #[allow(dead_code)]
    status: u32,   // 5 bits
    #[allow(dead_code)]
    reserved: u32, // 16 bits
}

// TypeConfusion mirrors the C union: we keep four bytes and reinterpret as
// needed.
#[derive(Default, Clone, Copy)]
struct TypeConfusion {
    bytes: [u8; 4],
}

impl TypeConfusion {
    fn set_int(&mut self, v: i32) {
        self.bytes = v.to_ne_bytes();
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
    fn signed_byte(&self, i: usize) -> i8 {
        self.bytes[i] as i8
    }
}

struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: Vec<u8>, // C allocation of `capacity` bytes; we keep raw bytes.
    buffer_len: usize, // bytes written by snprintf, not including NUL
    #[allow(dead_code)]
    capacity: i32,
}

// Helpers that print to stdout in raw byte form so output is byte identical
// regardless of locale or stdout buffering settings.
fn write_stdout(bytes: &[u8]) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(bytes);
    // C's printf is line-buffered when attached to a terminal but fully
    // buffered otherwise; flushing on every call matches the typical output
    // captured by harness scripts.
    let _ = handle.flush();
}

// Format an i32 the way printf("%d", ...) does.
fn fmt_d(v: i32) -> String {
    format!("{}", v)
}

// Format a u32 the way printf("%u", ...) does.
fn fmt_u(v: u32) -> String {
    format!("{}", v)
}

// Format a float the way printf("%f", ...) does: 6 fractional digits.
fn fmt_f(v: f32) -> String {
    // glibc's %f prints with 6 digits after the decimal point and uses
    // round-half-to-even on ties. f64 formatting via Rust's "{:.6}" does the
    // same rounding, which is consistent with glibc for the values produced
    // here.
    format!("{:.6}", v as f64)
}

// snprintf-equivalent that writes "State:<int>:Mode:<int>" into a buffer of
// at most `capacity` bytes (including NUL terminator). Returns number of
// bytes that would have been written excluding NUL (matches C semantics, but
// we only need the actual written length).
fn snprintf_state(buf: &mut Vec<u8>, capacity: usize, initial_val: i32, mode: u32) -> usize {
    if capacity == 0 {
        return 0;
    }
    let formatted = format!("State:{}:Mode:{}", initial_val, mode);
    let max = capacity.saturating_sub(1); // leave room for NUL
    let to_copy = std::cmp::min(formatted.len(), max);
    // Buffer size is `capacity`; zero it like fresh malloc'd memory after
    // snprintf would have written its NUL.
    buf.clear();
    buf.resize(capacity, 0);
    buf[..to_copy].copy_from_slice(&formatted.as_bytes()[..to_copy]);
    // ensure NUL after the copied prefix
    buf[to_copy] = 0;
    to_copy
}

fn create_state(initial_val: i32, capacity: i32) -> Option<Box<ProcessState>> {
    // The C code calls malloc and only returns NULL on allocation failure,
    // which we cannot easily simulate. We emulate the success path.
    let mut flags = PackedFlags::default();
    flags.flag1 = 1;
    flags.flag2 = 0;
    flags.flag3 = 1;
    flags.counter = 0;
    flags.mode = 3;
    flags.status = 15;
    flags.reserved = 0;

    let mut data = TypeConfusion::default();
    data.set_int(initial_val);

    // Allocate buffer of `capacity` bytes. If capacity <= 0, treat as a
    // failure path matching what C would essentially produce on a 0-length
    // malloc (implementation defined; we keep the success path).
    if capacity < 0 {
        return None;
    }
    let mut buffer: Vec<u8> = Vec::new();
    let cap_usize = capacity as usize;
    let written = snprintf_state(&mut buffer, cap_usize, initial_val, flags.mode);

    Some(Box::new(ProcessState {
        flags,
        data,
        buffer,
        buffer_len: written,
        capacity,
    }))
}

fn update_flags(state: &mut ProcessState, param: i32) {
    state.flags.counter = (state.flags.counter.wrapping_add(1)) & 0x1F;
    state.flags.flag1 = (param & 1) as u32 & 0x1;
    state.flags.flag2 = ((param & 2) >> 1) as u32 & 0x1;
    state.flags.flag3 = ((param & 4) >> 2) as u32 & 0x1;
    state.flags.mode = ((param >> 3) & 0x7) as u32;

    // DEBUG_VAR(state->flags.counter)
    let s = format!(
        "Debug: state->flags.counter = {}\n",
        fmt_d(state.flags.counter as i32)
    );
    write_stdout(s.as_bytes());

    let s = format!(
        "Bit fields - flag1:{} flag2:{} flag3:{} mode:{}\n",
        state.flags.flag1, state.flags.flag2, state.flags.flag3, state.flags.mode
    );
    write_stdout(s.as_bytes());
}

fn process_buffer(state: &ProcessState, target: u8) -> i32 {
    // strlen(state->buffer): C's strlen stops at the first NUL.
    let remaining_len = {
        // find first NUL in buffer up to buffer's allocated size
        let mut n = 0usize;
        while n < state.buffer.len() && state.buffer[n] != 0 {
            n += 1;
        }
        n
    };

    let mut count: i32 = 0;
    let mut idx = 0usize;
    let mut remaining = remaining_len;

    while remaining > 0 {
        // memchr in [idx..idx+remaining]
        let slice = &state.buffer[idx..idx + remaining];
        match slice.iter().position(|&b| b == target) {
            None => break,
            Some(pos) => {
                count += 1;
                let s = format!(
                    "Operation: memchr_found with value {}\n",
                    fmt_d(count)
                );
                write_stdout(s.as_bytes());

                // remaining -= (found - ptr + 1)
                let consumed = pos + 1;
                remaining -= consumed;
                idx += consumed;
            }
        }
    }

    // Silence unused field warnings in some build configurations.
    let _ = state.buffer_len;
    count
}

fn confuse_types(state: &mut ProcessState, operation: i32) -> i32 {
    let mut result: i32 = 0;
    match operation {
        0 => {
            state.data.set_int(1078530011);
            let s = format!("Set as int: {}\n", fmt_d(state.data.int_val()));
            write_stdout(s.as_bytes());
        }
        1 => {
            let f = state.data.float_val();
            let s = format!("Read as float: {}\n", fmt_f(f));
            write_stdout(s.as_bytes());
            // result = (int)(state->data.float_val * 100);
            // C casts float-to-int truncate toward zero.
            let prod = (f as f64) * 100.0_f64;
            // Match C cast: out-of-range is UB; clamp to i32 range to avoid
            // panics from `as i32` on NaN -- though for our reproduced inputs
            // it won't matter.
            result = prod as i32;
        }
        2 => {
            let u = state.data.uint_val();
            let s = format!("Read as uint: {}\n", fmt_u(u));
            write_stdout(s.as_bytes());
            result = (u & 0xFF) as i32;
        }
        3 => {
            let b0 = state.data.signed_byte(0) as i32;
            let b1 = state.data.signed_byte(1) as i32;
            let b2 = state.data.signed_byte(2) as i32;
            let b3 = state.data.signed_byte(3) as i32;
            let s = format!(
                "Read as bytes: [{}, {}, {}, {}]\n",
                fmt_d(b0),
                fmt_d(b1),
                fmt_d(b2),
                fmt_d(b3)
            );
            write_stdout(s.as_bytes());
            // Sum is performed in `int` in C; matches i32 arithmetic with
            // wrapping semantics.
            result = b0.wrapping_add(b1);
        }
        _ => {}
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    // DEBUG_VAR(param1..param4)
    write_stdout(format!("Debug: param1 = {}\n", fmt_d(param1)).as_bytes());
    write_stdout(format!("Debug: param2 = {}\n", fmt_d(param2)).as_bytes());
    write_stdout(format!("Debug: param3 = {}\n", fmt_d(param3)).as_bytes());
    write_stdout(format!("Debug: param4 = {}\n", fmt_d(param4)).as_bytes());

    let mut result: i32 = 0;

    let state = create_state(param1, 128);
    let mut state = match state {
        Some(s) => s,
        None => return -1,
    };

    update_flags(&mut state, param2);

    // search_char = '0' + (param3 % 10)
    // C's % can be negative for negative dividends; preserve that.
    let modv = param3 % 10;
    let search_char_i = (b'0' as i32).wrapping_add(modv);
    // C assigns to a `char`; truncate to u8.
    let search_char = (search_char_i & 0xFF) as u8;

    let found_count = process_buffer(&state, search_char);
    result = result.wrapping_add(found_count.wrapping_mul(10));

    let op = param4 % 4;
    let confusion_result = confuse_types(&mut state, op);
    result = result.wrapping_add(confusion_result);

    result = result.wrapping_add((state.flags.counter as i32).wrapping_mul(5));
    result = result.wrapping_add((state.flags.mode as i32).wrapping_mul(3));

    write_stdout(format!("Final result: {}\n", fmt_d(result)).as_bytes());

    // destroy_state: dropping `state` (Box) frees memory.
    drop(state);

    result
}
