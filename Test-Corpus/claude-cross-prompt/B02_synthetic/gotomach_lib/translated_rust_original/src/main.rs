// Rust translation of c_src/src/lib.c
// Produces byte-identical output for the same inputs.

use std::io::{self, Read, Write};

type OperationFn = fn(i32, i32, *mut core::ffi::c_void) -> i32;

struct ProcessorState {
    results: Vec<i32>,
    capacity: usize,
    count: usize,
    operation: OperationFn,
    status: u8,
}

fn is_valid_state(state: &ProcessorState) -> bool {
    if state.status != 0 {
        return state.count < state.capacity;
    }
    false
}

fn check_char_flag(flag: u8) -> bool {
    flag != 0
}

fn process_value(value: i32, _unused_param: i32, _unused_context: *mut core::ffi::c_void) -> i32 {
    value.wrapping_add(10)
}

fn double_value(value: i32, _unused_param: i32, _unused_context: *mut core::ffi::c_void) -> i32 {
    value.wrapping_mul(2)
}

fn triple_value(value: i32, _unused_param: i32, _unused_context: *mut core::ffi::c_void) -> i32 {
    value.wrapping_mul(3)
}

fn init_processor(capacity: usize, op: OperationFn) -> Option<ProcessorState> {
    // The C code allocates `capacity * sizeof(int)`. We mirror this with a vec
    // sized to capacity, but tracked count starts at 0.
    let results = vec![0i32; capacity];
    Some(ProcessorState {
        results,
        capacity,
        count: 0,
        operation: op,
        status: 1,
    })
}

fn log_msg(level: &str, msg: &str) {
    // Mirrors `printf("[" #level "] " msg "\n")` in C.
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "[{}] {}\n", level, msg);
}

const UINT16_MAX: i32 = 65535;

fn gotomach(iterations: i32, seed: i32, mode: i32, threshold: i32) -> i32 {
    let mut state: Option<ProcessorState> = None;
    let mut temp_buffer: Option<Vec<i32>> = None;
    let mut result: i32 = 0;
    let selected_op: OperationFn;

    log_msg("INFO", "Starting gotomach function");

    // Use a closure-style cleanup via labeled block; emulate `goto cleanup`.
    'cleanup: {
        if iterations < 0 || iterations > UINT16_MAX {
            log_msg("ERROR", "Invalid iteration count");
            result = -1;
            break 'cleanup;
        }

        if seed < 0 || seed > UINT16_MAX {
            log_msg("ERROR", "Invalid seed value");
            result = -2;
            break 'cleanup;
        }

        selected_op = match mode {
            0 => process_value,
            1 => double_value,
            2 => triple_value,
            _ => {
                log_msg("WARNING", "Invalid mode, using default");
                process_value
            }
        };

        state = init_processor(iterations as usize, selected_op);
        if state.is_none() {
            log_msg("ERROR", "Failed to initialize processor");
            result = -3;
            break 'cleanup;
        }

        // allocate temp buffer
        temp_buffer = Some(vec![0i32; iterations as usize]);
        if temp_buffer.is_none() {
            log_msg("ERROR", "Failed to allocate temporary buffer");
            result = -4;
            break 'cleanup;
        }

        {
            let s = state.as_ref().unwrap();
            if !check_char_flag(s.status) {
                log_msg("ERROR", "Invalid state status");
                result = -5;
                break 'cleanup;
            }
        }

        let mut current_value: i32 = seed;
        let s = state.as_mut().unwrap();
        let tb = temp_buffer.as_mut().unwrap();

        let mut early_break = false;
        for i in 0..iterations {
            if !is_valid_state(s) {
                log_msg("ERROR", "State became invalid during processing");
                result = -6;
                early_break = true;
                break;
            }

            let computed = (s.operation)(current_value, 0, core::ptr::null_mut());
            tb[i as usize] = computed;

            if computed < threshold {
                let idx = s.count;
                s.results[idx] = computed;
                s.count += 1;
            }

            current_value = computed.rem_euclid_c(1000);

            if s.count >= UINT16_MAX as usize {
                log_msg("WARNING", "Reached maximum count");
                break;
            }
        }

        if early_break {
            break 'cleanup;
        }

        result = 0;
        for i in 0..s.count {
            result = result.wrapping_add(s.results[i]);
        }

        log_msg("INFO", "Processing completed successfully");
    }

    // cleanup label
    drop(temp_buffer);
    drop(state);

    result
}

// C uses `%` which is truncated division remainder.
trait CMod {
    fn rem_euclid_c(self, rhs: i32) -> i32;
}

impl CMod for i32 {
    fn rem_euclid_c(self, rhs: i32) -> i32 {
        // C's `%` operator: truncated division remainder. Rust's `%` matches.
        self % rhs
    }
}

fn read_all_stdin() -> String {
    let mut buf = String::new();
    let _ = io::stdin().read_to_string(&mut buf);
    buf
}

fn parse_four_ints(input: &str) -> (i32, i32, i32, i32) {
    // Mimic scanf("%d %d %d %d") behavior: skip whitespace, parse signed ints.
    let mut iter = input.split_ascii_whitespace();
    let a = iter.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let b = iter.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let c = iter.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let d = iter.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    (a, b, c, d)
}

fn main() {
    let input = read_all_stdin();
    let (iterations, seed, mode, threshold) = parse_four_ints(&input);
    let result = gotomach(iterations, seed, mode, threshold);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", result);
}
