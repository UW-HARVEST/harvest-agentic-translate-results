// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::os::raw::c_int;

const _OP_ADD: i32 = 0x01;
const _OP_MULTIPLY: i32 = 0x02;
const _OP_XOR: i32 = 0x03;
const _OP_SHIFT: i32 = 0x04;
const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const MASK_LOWER: u32 = 0x0000FFFF;

#[derive(Copy, Clone)]
struct ComputeState {
    accumulator: i32,
    operation_count: i32,
    checksum: u32,
}

type OperationFunc = fn(i32, i32) -> i32;

static STATIC_MULTIPLIER: i32 = 3;
static STATIC_ADDEND: i32 = 100;
static STATIC_SHIFT_AMOUNT: i32 = 2;

fn multiply_with_static(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

fn add_with_static(a: i32, b: i32) -> i32 {
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

fn xor_operation(a: i32, b: i32) -> i32 {
    a ^ b ^ 0xABCD
}

fn shift_with_static(a: i32, b: i32) -> i32 {
    let left = (a as u32).wrapping_shl(STATIC_SHIFT_AMOUNT as u32) as i32;
    // C's `b >> static_shift_amount` for signed int is implementation-defined
    // but typically arithmetic shift. Use signed shift to match common behavior.
    let right = b >> (STATIC_SHIFT_AMOUNT as u32);
    left | right
}

fn get_operation(opcode: i32) -> Option<OperationFunc> {
    let ops: [OperationFunc; 4] = [
        multiply_with_static,
        add_with_static,
        xor_operation,
        shift_with_static,
    ];

    if opcode >= 0 && opcode < 4 {
        Some(ops[opcode as usize])
    } else {
        None
    }
}

fn execute_operation(
    func: Option<OperationFunc>,
    a: i32,
    b: i32,
    op_name: &str,
) -> i32 {
    let f = match func {
        Some(f) => f,
        None => {
            println!("Error: Operation function pointer is NULL for {}", op_name);
            return 0;
        }
    };

    println!("Variable a = {}", a);
    println!("Variable b = {}", b);

    let result = f(a, b);
    println!("Result of {}: {}", op_name, result);

    result
}

fn compute_checksum(values: &[i32]) -> u32 {
    let mut checksum: u32 = 0;
    let count = values.len();

    if count > 0 {
        let copy_count = if count > 4 { 4 } else { count };
        // Replicate the memcpy of `sizeof(int) * copy_count` bytes.
        let int_size = std::mem::size_of::<i32>();
        let total_bytes = int_size * copy_count;
        let mut buffer: [u8; 16] = [0; 16];
        for i in 0..copy_count {
            let bytes = values[i].to_ne_bytes();
            for j in 0..int_size {
                buffer[i * int_size + j] = bytes[j];
            }
        }

        for i in 0..total_bytes {
            checksum = checksum.wrapping_shl(1) ^ (buffer[i] as u32);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

fn init_state(state: &mut ComputeState, initial_value: i32) {
    state.accumulator = initial_value;
    state.operation_count = 0;
    state.checksum = 0;
    println!(
        "State initialized with accumulator = {}",
        state.accumulator
    );
}

fn apply_operation(
    state: &mut ComputeState,
    value: i32,
    func: Option<OperationFunc>,
) {
    let f = match func {
        Some(f) => f,
        None => {
            println!(
                "Error: operation function pointer is NULL in apply_operation"
            );
            return;
        }
    };

    state.accumulator = f(state.accumulator, value);
    state.operation_count += 1;
}

#[no_mangle]
pub extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let param1 = param1 as i32;
    let param2 = param2 as i32;
    let param3 = param3 as i32;
    let param4 = param4 as i32;

    println!("\n=== Starting foo function ===");
    println!(
        "Parameters: {}, {}, {}, {}",
        param1, param2, param3, param4
    );

    // Allocate state on the heap (mirroring malloc).
    let mut state_box: Box<ComputeState> = Box::new(ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    });

    init_state(&mut state_box, param1);

    let params: [i32; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    println!("\n--- Operation 1: Multiply ---");
    apply_operation(&mut state_box, param2, mult_op);

    println!("\n--- Operation 2: Add ---");
    apply_operation(&mut state_box, param3, add_op);

    println!("\n--- Operation 3: XOR ---");
    let xor_result =
        execute_operation(xor_op, state_box.accumulator, param4, "XOR");

    println!("\n--- Operation 4: Shift ---");
    let shift_result =
        execute_operation(shift_op, xor_result, param2, "SHIFT");

    state_box.checksum = compute_checksum(&params);
    println!("\nComputed checksum: 0x{:04X}", state_box.checksum);

    let final_result = (state_box
        .accumulator
        .wrapping_add(shift_result))
        ^ (state_box.checksum as i32);

    println!("\nFinal accumulator: {}", state_box.accumulator);
    println!("Operation count: {}", state_box.operation_count);
    println!("Final result: {}", final_result);

    // Box is dropped at end of scope (mirrors free()).
    drop(state_box);

    println!("=== Ending foo function ===\n");

    final_result as c_int
}
