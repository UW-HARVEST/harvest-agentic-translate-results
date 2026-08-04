// Rust translation of c_src/src/lib.c
// Reads 4 integers from stdin (scanf %d %d %d %d) and calls confusion().

use std::io::{self, Read, Write};

#[derive(Clone, Copy, Default)]
struct PackedFlags {
    // Stored as raw u32 values, masked when assigned.
    flag1: u32,   // 1 bit
    flag2: u32,   // 1 bit
    flag3: u32,   // 1 bit
    counter: u32, // 5 bits
    mode: u32,    // 3 bits
    status: u32,  // 5 bits
    #[allow(dead_code)]
    reserved: u32, // 16 bits
}

#[derive(Clone, Copy)]
struct TypeConfusion {
    bytes: [u8; 4],
}

impl TypeConfusion {
    fn new() -> Self {
        Self { bytes: [0; 4] }
    }

    fn set_int(&mut self, v: i32) {
        self.bytes = v.to_le_bytes();
    }

    fn int_val(&self) -> i32 {
        i32::from_le_bytes(self.bytes)
    }

    fn float_val(&self) -> f32 {
        f32::from_le_bytes(self.bytes)
    }

    fn uint_val(&self) -> u32 {
        u32::from_le_bytes(self.bytes)
    }
}

struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: Vec<u8>, // C-string with null terminator
    #[allow(dead_code)]
    capacity: i32,
}

/// snprintf-like helper. Writes formatted string into buffer of given capacity.
/// Behaves like C snprintf: capacity-1 chars max plus null terminator.
fn snprintf_state(capacity: usize, initial_val: i32, mode: u32) -> Vec<u8> {
    if capacity == 0 {
        return Vec::new();
    }
    let formatted = format!("State:{}:Mode:{}", initial_val, mode);
    let bytes = formatted.as_bytes();
    let max_chars = capacity - 1;
    let take = bytes.len().min(max_chars);
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(&bytes[..take]);
    out.push(0);
    out
}

/// Returns length until first null byte, mimicking strlen.
fn cstr_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn create_state<W: Write>(out: &mut W, initial_val: i32, capacity: i32) -> Option<ProcessState> {
    // malloc never fails in our translation, but we still construct state similarly.
    let mut flags = PackedFlags::default();
    flags.flag1 = 1 & 0x1;
    flags.flag2 = 0 & 0x1;
    flags.flag3 = 1 & 0x1;
    flags.counter = 0 & 0x1F;
    flags.mode = 3 & 0x7;
    flags.status = 15 & 0x1F;
    flags.reserved = 0 & 0xFFFF;

    let mut data = TypeConfusion::new();
    data.set_int(initial_val);

    if capacity < 0 {
        // Extremely defensive; C would call malloc with negative size.
        let _ = out.write_all(b"Error: Failed to allocate buffer\n");
        return None;
    }

    let cap_usize = capacity as usize;
    let buffer = snprintf_state(cap_usize, initial_val, flags.mode);

    Some(ProcessState {
        flags,
        data,
        buffer,
        capacity,
    })
}

fn process_buffer<W: Write>(out: &mut W, state: &ProcessState, target: u8) -> i32 {
    // state and buffer are guaranteed non-null in our translation.
    let mut count: i32 = 0;
    let total_len = cstr_len(&state.buffer);
    let mut idx: usize = 0;
    let mut remaining = total_len;

    while remaining > 0 {
        let slice = &state.buffer[idx..idx + remaining];
        let pos = slice.iter().position(|&b| b == target);
        match pos {
            None => break,
            Some(p) => {
                count += 1;
                let _ = writeln!(out, "Operation: memchr_found with value {}", count);
                remaining -= p + 1;
                idx += p + 1;
            }
        }
    }

    count
}

fn update_flags<W: Write>(out: &mut W, state: &mut ProcessState, param: i32) {
    state.flags.counter = (state.flags.counter.wrapping_add(1)) & 0x1F;
    // C bit ops on signed int; we replicate with masks then truncate to bit-field widths.
    let p_u = param as u32;
    state.flags.flag1 = (p_u & 1) & 0x1;
    state.flags.flag2 = ((p_u & 2) >> 1) & 0x1;
    state.flags.flag3 = ((p_u & 4) >> 2) & 0x1;
    // (param >> 3) & 0x7 - in C this is signed shift; for our purposes treat as
    // signed arithmetic shift then mask. The mask of 0x7 makes the signedness
    // irrelevant for the result.
    let shifted = (param >> 3) as u32;
    state.flags.mode = (shifted & 0x7) & 0x7;

    // DEBUG_VAR(state->flags.counter) -> "Debug: state->flags.counter = %d\n"
    let _ = writeln!(out, "Debug: state->flags.counter = {}", state.flags.counter);
    let _ = writeln!(
        out,
        "Bit fields - flag1:{} flag2:{} flag3:{} mode:{}",
        state.flags.flag1, state.flags.flag2, state.flags.flag3, state.flags.mode
    );
}

/// Format a float the same way as C printf("%f", v) (default precision 6).
fn c_printf_f(v: f32) -> String {
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    // C's %f promotes float to double; precision 6.
    let v_d = v as f64;
    format!("{:.*}", 6, v_d)
}

fn confuse_types<W: Write>(out: &mut W, state: &mut ProcessState, operation: i32) -> i32 {
    let mut result: i32 = 0;
    match operation {
        0 => {
            state.data.set_int(1078530011);
            let _ = writeln!(out, "Set as int: {}", state.data.int_val());
        }
        1 => {
            let f = state.data.float_val();
            let _ = writeln!(out, "Read as float: {}", c_printf_f(f));
            // (int)(float * 100) - C cast truncates toward zero.
            let prod = f * 100.0_f32;
            // Saturating cast to i32 mimics typical x86 behavior closely; for
            // representative inputs (near pi) this is plain truncation.
            result = prod as i32;
        }
        2 => {
            let u = state.data.uint_val();
            let _ = writeln!(out, "Read as uint: {}", u);
            result = (u & 0xFF) as i32;
        }
        3 => {
            // bytes are signed char in C printf %d.
            let b: [i8; 4] = [
                state.data.bytes[0] as i8,
                state.data.bytes[1] as i8,
                state.data.bytes[2] as i8,
                state.data.bytes[3] as i8,
            ];
            let _ = writeln!(
                out,
                "Read as bytes: [{}, {}, {}, {}]",
                b[0] as i32, b[1] as i32, b[2] as i32, b[3] as i32
            );
            result = (b[0] as i32) + (b[1] as i32);
        }
        _ => {}
    }
    result
}

fn confusion<W: Write>(out: &mut W, param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let _ = writeln!(out, "Debug: param1 = {}", param1);
    let _ = writeln!(out, "Debug: param2 = {}", param2);
    let _ = writeln!(out, "Debug: param3 = {}", param3);
    let _ = writeln!(out, "Debug: param4 = {}", param4);

    let mut result: i32 = 0;

    let state_opt = create_state(out, param1, 128);
    let mut state = match state_opt {
        Some(s) => s,
        None => return -1,
    };

    update_flags(out, &mut state, param2);

    // search_char = '0' + (param3 % 10) — C truncates % toward zero, so
    // negative inputs yield negative remainders.
    let rem = param3 % 10; // matches C's truncated modulus
    let search_char_val: i32 = (b'0' as i32).wrapping_add(rem);
    // C narrows to char (signed/unsigned per platform); memchr treats as unsigned char.
    let search_byte: u8 = (search_char_val as i32 & 0xFF) as u8;

    let found_count = process_buffer(out, &state, search_byte);
    result = result.wrapping_add(found_count.wrapping_mul(10));

    // operation = param4 % 4 (truncated modulus). In C, switch only matches 0..3,
    // so negative remainders fall through with no output and result += 0.
    let operation = param4 % 4;
    let confusion_result = confuse_types(out, &mut state, operation);
    result = result.wrapping_add(confusion_result);

    // counter is 5-bit unsigned bit field; multiply as int.
    result = result.wrapping_add((state.flags.counter as i32).wrapping_mul(5));
    result = result.wrapping_add((state.flags.mode as i32).wrapping_mul(3));

    let _ = writeln!(out, "Final result: {}", result);

    // destroy_state — Rust drops automatically.
    drop(state);

    result
}

/// Read all of stdin and parse 4 whitespace-separated decimal integers
/// (matching scanf("%d %d %d %d", ...) which reads across newlines).
fn read_four_ints() -> Option<(i32, i32, i32, i32)> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    let mut it = buf.split_ascii_whitespace();
    let parse_next = |it: &mut std::str::SplitAsciiWhitespace| -> Option<i32> {
        let s = it.next()?;
        // C's scanf %d: optional sign, digits. Stops at first non-digit.
        // split_ascii_whitespace gives whole tokens; we accept as long as a
        // valid prefix parses. We mimic strtol-ish behavior.
        // Try full parse first; if that fails, try a prefix.
        if let Ok(v) = s.parse::<i32>() {
            return Some(v);
        }
        // Take longest valid integer prefix
        let bytes = s.as_bytes();
        let mut end = 0;
        if !bytes.is_empty() && (bytes[0] == b'-' || bytes[0] == b'+') {
            end = 1;
        }
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == 0 || (end == 1 && (bytes[0] == b'-' || bytes[0] == b'+')) {
            return None;
        }
        s[..end].parse::<i32>().ok()
    };
    let a = parse_next(&mut it)?;
    let b = parse_next(&mut it)?;
    let c = parse_next(&mut it)?;
    let d = parse_next(&mut it)?;
    Some((a, b, c, d))
}

fn main() {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let (a, b, c, d) = match read_four_ints() {
        Some(t) => t,
        None => {
            // No clean fallback — exit silently. The original C would have
            // similarly produced nothing if scanf failed, but here we exit 1.
            std::process::exit(1);
        }
    };

    let _ = confusion(&mut out, a, b, c, d);
}
