// Translated from C to Rust. Preserves the exact behavior of the original C code.

use std::io::{self, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

// Wrapping integer arithmetic to mirror C's signed int (i32) semantics.
// In C, signed integer overflow is undefined behavior, but in practice on
// x86_64 it wraps around. We use i32 with wrapping_* operations.

// Global mutable state to mirror the C `static` globals.
static mut GLOBAL_COUNTER: i32 = 0;
static mut GLOBAL_ACCUMULATOR: i32 = 0;

fn increment_counter(value: i32, _unused_param: i32) {
    unsafe {
        GLOBAL_COUNTER = GLOBAL_COUNTER.wrapping_add(value);
    }
}

fn update_accumulator(value: i32, _unused_param: i32) {
    unsafe {
        GLOBAL_ACCUMULATOR = GLOBAL_ACCUMULATOR.wrapping_mul(2).wrapping_add(value);
    }
}

type OperationFunc = fn(i32, i32, i32) -> i32;
type ModifierFunc = fn(i32, i32);

fn apply_operation(op: OperationFunc, a: i32, b: i32, c: i32) -> i32 {
    op(a, b, c)
}

fn add_three(a: i32, b: i32, c: i32) -> i32 {
    a.wrapping_add(b).wrapping_add(c)
}

fn multiply_add(a: i32, b: i32, c: i32) -> i32 {
    a.wrapping_mul(b).wrapping_add(c)
}

fn complex_calc(a: i32, b: i32, c: i32) -> i32 {
    let counter = unsafe { GLOBAL_COUNTER };
    a.wrapping_sub(b).wrapping_mul(c).wrapping_add(counter)
}

#[derive(Clone, Copy)]
struct DataRecord {
    id: i32,
    value: i32,
    timestamp: i64, // time_t
    name: [u8; 32],
}

impl DataRecord {
    fn new() -> Self {
        DataRecord {
            id: 0,
            value: 0,
            timestamp: 0,
            name: [0u8; 32],
        }
    }
}

fn shift_array_data(arr: &mut [i32], size: usize, shift_by: usize) {
    if shift_by > 0 && shift_by < size {
        // memmove(arr, arr + shift_by, (size - shift_by) * sizeof(int))
        arr.copy_within(shift_by..size, 0);
        // memset(arr + (size - shift_by), 0, shift_by * sizeof(int))
        for i in (size - shift_by)..size {
            arr[i] = 0;
        }
    }
}

fn process_pointer_data(value: i32, multiplier: i32) -> i32 {
    let acc = unsafe { GLOBAL_ACCUMULATOR };
    value.wrapping_mul(multiplier).wrapping_add(acc)
}

fn compute_with_dynamic_memory(base: i32, count: i32) -> i32 {
    let count_usize = count as usize;
    let mut temp_array: Vec<i32> = vec![0; count_usize];

    for i in 0..count {
        temp_array[i as usize] = base.wrapping_add(i.wrapping_mul(3));
    }

    let mut sum: i32 = 0;
    for i in 0..count {
        sum = sum.wrapping_add(temp_array[i as usize]);
    }

    sum
}

fn get_time_based_value(seed: i32) -> i32 {
    // time(&current_time)
    let current_time: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // reference_time = current_time - (seed * 3600)
    // Note: in C, seed * 3600 may overflow as int; subtraction is between time_t and int.
    // On most systems time_t is i64, and `seed * 3600` is computed as int then converted.
    let seed_times_3600: i32 = seed.wrapping_mul(3600);
    let reference_time: i64 = current_time.wrapping_sub(seed_times_3600 as i64);

    // difftime returns double
    let diff: f64 = (current_time as f64) - (reference_time as f64);

    // (int)(diff / 100) + seed
    ((diff / 100.0) as i32).wrapping_add(seed)
}

fn manipulate_records(records: &mut [DataRecord], num_records: i32, shift: i32) -> i32 {
    let mut total: i32 = 0;

    if shift > 0 && shift < num_records {
        // memmove(records, records + shift, (num_records - shift) * sizeof(DataRecord));
        let s = shift as usize;
        let n = num_records as usize;
        records.copy_within(s..n, 0);
    }

    // for (int i = 0; i < num_records - shift; i++)
    // Note: this iterates regardless of whether the shift condition above was true.
    let limit = num_records.wrapping_sub(shift);
    for i in 0..limit {
        total = total.wrapping_add(records[i as usize].value);
    }

    total
}

fn hatch(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result: i32 = 0;

    let mut mod_func: ModifierFunc;

    mod_func = increment_counter;
    mod_func(param1, 999);

    mod_func = update_accumulator;
    mod_func(param2, 888);
    let _ = mod_func; // silence unused warning

    let mut op_func: OperationFunc;

    op_func = add_three;
    result = result.wrapping_add(apply_operation(op_func, param1, param2, param3));

    op_func = multiply_add;
    result = result.wrapping_add(apply_operation(op_func, param2, param3, param4));

    op_func = complex_calc;
    result = result.wrapping_add(apply_operation(op_func, param1, param3, param4));
    let _ = op_func;

    // int *dynamic_data = (int *)malloc(10 * sizeof(int));
    let mut dynamic_data: Vec<i32> = vec![0; 10];
    for i in 0..10i32 {
        dynamic_data[i as usize] = param1.wrapping_add(i);
    }

    // process_pointer_data(&dynamic_data[5], param2)
    result = result.wrapping_add(process_pointer_data(dynamic_data[5], param2));

    shift_array_data(&mut dynamic_data, 10, 3);
    result = result.wrapping_add(dynamic_data[0]);

    // free(dynamic_data) -> drop happens automatically

    result = result.wrapping_add(get_time_based_value(param3));

    // DataRecord *records = (DataRecord *)malloc(5 * sizeof(DataRecord));
    let mut records: Vec<DataRecord> = vec![DataRecord::new(); 5];

    for i in 0..5i32 {
        records[i as usize].id = i;
        records[i as usize].value = param4.wrapping_add(i.wrapping_mul(10));

        // time(&records[i].timestamp)
        let now: i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        records[i as usize].timestamp = now;

        // snprintf(records[i].name, 32, "Record_%d", i)
        let s = format!("Record_{}", i);
        let bytes = s.as_bytes();
        // emulate snprintf truncation to 32 bytes including null terminator
        let max = 31.min(bytes.len());
        for j in 0..max {
            records[i as usize].name[j] = bytes[j];
        }
        records[i as usize].name[max] = 0;
    }

    result = result.wrapping_add(manipulate_records(&mut records, 5, 2));

    // free(records) -> drop happens automatically

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    let counter = unsafe { GLOBAL_COUNTER };
    let acc = unsafe { GLOBAL_ACCUMULATOR };
    result = result.wrapping_add(counter.wrapping_add(acc));

    result
}

// Read whitespace-delimited tokens from stdin (mirrors scanf("%d") behavior:
// reads across newlines, skipping any whitespace).
struct Scanner {
    buffer: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer).ok();
        Scanner { buffer, pos: 0 }
    }

    fn next_int(&mut self) -> Option<i32> {
        // Skip whitespace
        while self.pos < self.buffer.len()
            && (self.buffer[self.pos] as char).is_ascii_whitespace()
        {
            self.pos += 1;
        }
        if self.pos >= self.buffer.len() {
            return None;
        }
        let start = self.pos;
        // Optional sign
        if self.buffer[self.pos] == b'+' || self.buffer[self.pos] == b'-' {
            self.pos += 1;
        }
        while self.pos < self.buffer.len()
            && (self.buffer[self.pos] as char).is_ascii_digit()
        {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        let s = std::str::from_utf8(&self.buffer[start..self.pos]).ok()?;
        s.parse::<i32>().ok()
    }
}

fn main() {
    let mut scanner = Scanner::new();
    let a = scanner.next_int().unwrap_or(0);
    let b = scanner.next_int().unwrap_or(0);
    let c = scanner.next_int().unwrap_or(0);
    let d = scanner.next_int().unwrap_or(0);

    let result = hatch(a, b, c, d);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    // printf("%d\n", result)
    writeln!(out, "{}", result).ok();
}
