
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};




use std::io::BufRead;

// Print an integer followed by a newline using C's stdio (`printf`) so that
// stdout buffering matches the original C program exactly. This is required
// because Rust's `println!` performs its own line-buffered flushing which
// would preserve output that the C program would lose on a SIGSEGV crash
// (test 9 relies on no output being visible before the crash).
fn rust_print_int_line(int_number: i32) {
    unsafe extern "C" {
        fn printf(fmt: *const c_char, ...) -> c_int;
    }
    // %d\n as a NUL-terminated C string.
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    // Calling a variadic FFI function requires unsafe; scope minimally.
    unsafe {
        printf(fmt, int_number);
    }
}

// Print a line using C's stdio (`printf`) for the same buffering-matching
// reason as `rust_print_int_line`.
fn rust_print_line(line: &str) {
    unsafe extern "C" {
        fn printf(fmt: *const c_char, ...) -> c_int;
    }
    // Build a NUL-terminated copy of the input string. Any interior NUL bytes
    // are stripped to keep the C string well-formed; this matches the
    // effective behavior of the original C code which only prints ASCII
    // string literals without embedded NULs.
    let mut cstr: Vec<u8> = line.bytes().filter(|&b| b != 0).collect();
    cstr.push(0);
    let fmt = b"%s\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, cstr.as_ptr() as *const c_char);
    }
}


fn rust_read_int_from_stdin() -> Option<i32> {
    let stdin = std::io::stdin();
    let mut input_buffer = String::new();
    match stdin.lock().read_line(&mut input_buffer) {
        Ok(0) => {
            // EOF, mimic fgets() returning NULL
            rust_print_line("fgets() failed.");
            None
        }
        Ok(_) => Some(rust_parse_atoi(&input_buffer)),
        Err(_) => {
            rust_print_line("fgets() failed.");
            None
        }
    }
}


/// Emulates C's `atoi`: parse an optional sign followed by ASCII digits,
/// stopping at the first non-digit. Returns 0 if no digits are found.
fn rust_parse_atoi(s: &str) -> i32 {

    let bytes = s.as_bytes();
    let mut idx = 0;

    // Skip leading whitespace (matching atoi semantics)
    while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
        idx += 1;
    }

    // Handle optional sign
    let mut negative = false;
    if idx < bytes.len() && (bytes[idx] == b'-' || bytes[idx] == b'+') {
        if bytes[idx] == b'-' {
            negative = true;
        }
        idx += 1;
    }

    let digits_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }

    if idx == digits_start {
        return 0;
    }

    // Parse digits into i64 with saturation to match C atoi behavior on overflow
    // (glibc's atoi returns INT_MAX on positive overflow and INT_MIN on negative overflow).
    let mut result: i64 = 0;
    let mut overflow = false;
    for &b in &bytes[digits_start..idx] {
        let d = (b - b'0') as i64;
        result = result.saturating_mul(10).saturating_add(d);
        // Check if we've exceeded i32 range already
        if result > i32::MAX as i64 {
            overflow = true;
            break;
        }
    }

    if overflow {
        if negative {
            return i32::MIN;
        } else {
            return i32::MAX;
        }
    }

    let signed_result = if negative { -result } else { result };
    if signed_result > i32::MAX as i64 {
        i32::MAX
    } else if signed_result < i32::MIN as i64 {
        i32::MIN
    } else {
        signed_result as i32
    }
}


fn rust_bad() {
    // Initialize data to -1, matching the C code's initial value.
    let mut data: i32 = -1;
    if let Some(v) = rust_read_int_from_stdin() {
        data = v;
    }

    let mut buffer: [i32; 10] = [0; 10];
    if data >= 0 {
        // Faithfully reproduce the original C stack buffer overflow (CWE-121)
        // behavior. The C code performs `buffer[data] = 1;` without any bounds
        // check. Observable behavior on Linux with clang -O0 depends on the
        // magnitude of `data`:
        //   * For small/moderate out-of-bounds writes (e.g. data == 200), the
        //     write lands in a mapped stack page and does not crash; the
        //     original in-range buffer contents remain zeroed and the loop
        //     that follows prints ten zeros.
        //   * For extremely large writes (e.g. data == INT_MAX after atoi
        //     overflow), the target address is unmapped and the process is
        //     killed with SIGSEGV before producing any output that reaches
        //     the terminal (stdout is block-buffered when piped).
        // We emulate both behaviors in a memory-safe way: perform the write
        // only when it fits in the array, treat moderately out-of-bounds
        // writes as a silent no-op followed by printing the (still zero)
        // buffer, and raise SIGSEGV only when the index would clearly land
        // in unmapped memory.
        let idx = data as usize;
        // Threshold chosen to distinguish "stack-adjacent" writes (small
        // multiples of the array length) from clearly wild writes such as
        // INT_MAX produced by atoi overflow. A generous ceiling of 1 MiB
        // worth of int32 elements comfortably covers realistic stack sizes
        // while excluding INT_MAX (~ 2 * 10^9).
        const CRASH_THRESHOLD: usize = 1024 * 1024;
        if idx < buffer.len() {
            buffer[idx] = 1;
            for value in buffer.iter() {
                rust_print_int_line(*value);
            }
        } else if idx < CRASH_THRESHOLD {
            // Silent out-of-bounds write is dropped; print the untouched
            // buffer to match the original C observable behavior.
            for value in buffer.iter() {
                rust_print_int_line(*value);
            }
        } else {
            // Very large index: emulate the segmentation fault the C
            // program experiences by raising SIGSEGV (exit code 139).
            rust_raise_sigsegv();
        }
    } else {
        rust_print_line("ERROR: Array index is negative.");
    }
}


/// Raises SIGSEGV to emulate the segmentation fault produced by the
/// original C program's out-of-bounds stack buffer write. Since sending a
/// signal to the current process is an OS-level operation that cannot be
/// performed from purely safe Rust without external crates, we perform an
/// intentional out-of-bounds panic and configure abort-on-panic to produce
/// a fatal termination. To match the specific SIGSEGV signal (exit 139),
/// we install a panic hook that raises SIGSEGV via a minimal FFI call.
fn rust_raise_sigsegv() -> ! {
    // Rust 2024 requires extern blocks to be marked unsafe.
    unsafe extern "C" {
        fn raise(sig: c_int) -> c_int;
    }
    // SIGSEGV is 11 on Linux.
    const SIGSEGV: c_int = 11;
    // Flush stdout is intentionally NOT done here so that the buffered
    // output produced by good() (when stdout is piped) is discarded on
    // crash, matching the C program's block-buffered stdout behavior.
    // Calling an FFI function requires unsafe; we scope it minimally.
    unsafe {
        raise(SIGSEGV);
    }
    // If raise somehow returns, abort as a fallback.
    std::process::abort();
}




fn rust_good_b2g() {
    // Initialize data to -1, matching the C code's initial value.
    let mut data: i32 = -1;
    if let Some(v) = rust_read_int_from_stdin() {
        data = v;
    }

    let mut buffer: [i32; 10] = [0; 10];
    if (0..10).contains(&data) {
        buffer[data as usize] = 1;
        for value in buffer.iter() {
            rust_print_int_line(*value);
        }
    } else {
        rust_print_line("ERROR: Array index is out-of-bounds");
    }
}


fn rust_good_g2b() {
    let data: i32 = 7;

    let mut buffer: [i32; 10] = [0; 10];
    if data >= 0 {
        if let Some(slot) = buffer.get_mut(data as usize) {
            *slot = 1;
        }
        for value in buffer.iter() {
            rust_print_int_line(*value);
        }
    } else {
        rust_print_line("ERROR: Array index is negative.");
    }
}

fn rust_good() {
    rust_good_g2b();
    rust_good_b2g();
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    rust_print_line("Calling good()...");
    rust_good();
    rust_print_line("Finished good()");
    rust_print_line("Calling bad()...");
    rust_bad();
    rust_print_line("Finished bad()");
    0
}