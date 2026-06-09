use std::io::{self, Read, Write};

const OP_ADD: i32 = 0x01;
const OP_MULTIPLY: i32 = 0x02;
const OP_XOR: i32 = 0x03;
const OP_SHIFT: i32 = 0x04;
const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const MASK_LOWER: u32 = 0x0000FFFF;

// Suppress dead-code warnings — these constants exist in the C code but are
// not used directly; we keep them to faithfully mirror the original.
#[allow(dead_code)]
const _UNUSED_CONSTS: (i32, i32, i32, i32) = (OP_ADD, OP_MULTIPLY, OP_XOR, OP_SHIFT);

#[derive(Clone, Copy)]
struct ComputeState {
    accumulator: i32,
    operation_count: i32,
    checksum: u32,
}

type OperationFunc = fn(i32, i32) -> i32;

// Static "globals" used by the operations.
const STATIC_MULTIPLIER: i32 = 3;
const STATIC_ADDEND: i32 = 100;
const STATIC_SHIFT_AMOUNT: u32 = 2;

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
    // Match C: (a << static_shift_amount) | (b >> static_shift_amount)
    // Use wrapping_shl/shr to avoid panics on edge cases.
    let left = (a as u32).wrapping_shl(STATIC_SHIFT_AMOUNT) as i32;
    let right = b.wrapping_shr(STATIC_SHIFT_AMOUNT);
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
    if func.is_none() {
        print!(
            "Error: Operation function pointer is NULL for {}\n",
            op_name
        );
        return 0;
    }

    print!("Variable a = {}\n", a);
    print!("Variable b = {}\n", b);

    let result = func.unwrap()(a, b);
    print!("Result of {}: {}\n", op_name, result);

    result
}

fn compute_checksum(values: &[i32]) -> u32 {
    let count = values.len();
    let mut checksum: u32 = 0;

    if count > 0 {
        let copy_count = if count > 4 { 4 } else { count };
        // Mimic memcpy of `int` array bytes (little-endian on typical platforms).
        let mut buffer: Vec<u8> = Vec::with_capacity(4 * copy_count);
        for i in 0..copy_count {
            buffer.extend_from_slice(&values[i].to_le_bytes());
        }

        for i in 0..(4 * copy_count) {
            checksum = checksum.wrapping_shl(1) ^ (buffer[i] as u32);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

fn init_state(state: &mut ComputeState, initial_value: i32) {
    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0x0000,
    };
    *state = template;
    print!(
        "State initialized with accumulator = {}\n",
        state.accumulator
    );
}

fn apply_operation(state: &mut ComputeState, value: i32, func: Option<OperationFunc>) {
    if func.is_none() {
        print!("Error: operation function pointer is NULL in apply_operation\n");
        return;
    }
    state.accumulator = func.unwrap()(state.accumulator, value);
    state.operation_count = state.operation_count.wrapping_add(1);
}

fn checkshift(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    print!("\n=== Starting foo function ===\n");
    print!(
        "Parameters: {}, {}, {}, {}\n",
        param1, param2, param3, param4
    );

    let mut state = ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    };

    init_state(&mut state, param1);

    let params: [i32; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    print!("\n--- Operation 1: Multiply ---\n");
    apply_operation(&mut state, param2, mult_op);

    print!("\n--- Operation 2: Add ---\n");
    apply_operation(&mut state, param3, add_op);

    print!("\n--- Operation 3: XOR ---\n");
    let xor_result = execute_operation(xor_op, state.accumulator, param4, "XOR");

    print!("\n--- Operation 4: Shift ---\n");
    let shift_result = execute_operation(shift_op, xor_result, param2, "SHIFT");

    state.checksum = compute_checksum(&params);
    // %04X uppercase hex, min width 4, zero-padded.
    print!("\nComputed checksum: 0x{:04X}\n", state.checksum);

    // Replicate: (state->accumulator + shift_result) ^ state->checksum
    // The XOR with unsigned int converts the int to unsigned int in C.
    let sum = state.accumulator.wrapping_add(shift_result);
    let final_result = ((sum as u32) ^ state.checksum) as i32;

    print!("\nFinal accumulator: {}\n", state.accumulator);
    print!("Operation count: {}\n", state.operation_count);
    print!("Final result: {}\n", final_result);

    print!("=== Ending foo function ===\n\n");

    final_result
}

fn main() {
    // Read all of stdin and parse whitespace-separated integers (scanf-style).
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        std::process::exit(1);
    }

    let mut iter = input.split_ascii_whitespace();
    let mut nums: [i32; 4] = [0; 4];
    for i in 0..4 {
        match iter.next() {
            Some(tok) => match tok.parse::<i32>() {
                Ok(v) => nums[i] = v,
                Err(_) => {
                    std::process::exit(1);
                }
            },
            None => {
                std::process::exit(1);
            }
        }
    }

    let _ = checkshift(nums[0], nums[1], nums[2], nums[3]);

    // Ensure stdout is flushed before exit.
    let _ = io::stdout().flush();
}
