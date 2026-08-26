use std::os::raw::c_int;

const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const MASK_LOWER: u32 = 0x0000FFFF;

#[derive(Clone, Copy)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: u32,
}

type OperationFunc = fn(c_int, c_int) -> c_int;

static STATIC_MULTIPLIER: c_int = 3;
static STATIC_ADDEND: c_int = 100;
static STATIC_SHIFT_AMOUNT: c_int = 2;

fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

fn shift_with_static(a: c_int, b: c_int) -> c_int {
    (a << STATIC_SHIFT_AMOUNT) | (b >> STATIC_SHIFT_AMOUNT)
}

fn get_operation(opcode: c_int) -> Option<OperationFunc> {
    match opcode {
        0 => Some(multiply_with_static),
        1 => Some(add_with_static),
        2 => Some(xor_operation),
        3 => Some(shift_with_static),
        _ => None,
    }
}

fn execute_operation(func: Option<OperationFunc>, a: c_int, b: c_int, op_name: &str) -> c_int {
    let Some(f) = func else {
        println!("Error: Operation function pointer is NULL for {}", op_name);
        return 0;
    };

    println!("Variable a = {}", a);
    println!("Variable b = {}", b);

    let result = f(a, b);
    println!("Result of {}: {}", op_name, result);

    result
}

fn compute_checksum(values: &[c_int]) -> u32 {
    let mut checksum: u32 = 0;

    if !values.is_empty() {
        let copy_count = if values.len() > 4 { 4 } else { values.len() };
        let mut buffer = Vec::with_capacity(std::mem::size_of::<c_int>() * copy_count);
        for &val in &values[..copy_count] {
            buffer.extend_from_slice(&val.to_ne_bytes());
        }

        for &byte in &buffer {
            checksum = (checksum << 1) ^ (byte as u32);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

fn init_state(state: &mut ComputeState, initial_value: c_int) {
    state.accumulator = initial_value;
    state.operation_count = 0;
    state.checksum = 0x0000;

    println!("State initialized with accumulator = {}", state.accumulator);
}

fn apply_operation(state: &mut ComputeState, value: c_int, func: Option<OperationFunc>) {
    let Some(f) = func else {
        println!("Error: operation function pointer is NULL in apply_operation");
        return;
    };

    state.accumulator = f(state.accumulator, value);
    state.operation_count += 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn checkshift(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    println!("\n=== Starting foo function ===");
    println!("Parameters: {}, {}, {}, {}", param1, param2, param3, param4);

    let mut state = ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    };

    init_state(&mut state, param1);

    let params = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    println!("\n--- Operation 1: Multiply ---");
    apply_operation(&mut state, param2, mult_op);

    println!("\n--- Operation 2: Add ---");
    apply_operation(&mut state, param3, add_op);

    println!("\n--- Operation 3: XOR ---");
    let xor_result = execute_operation(xor_op, state.accumulator, param4, "XOR");

    println!("\n--- Operation 4: Shift ---");
    let shift_result = execute_operation(shift_op, xor_result, param2, "SHIFT");

    state.checksum = compute_checksum(&params);
    println!("\nComputed checksum: 0x{:04X}", state.checksum);

    let final_result = state.accumulator.wrapping_add(shift_result) ^ (state.checksum as c_int);

    println!("\nFinal accumulator: {}", state.accumulator);
    println!("Operation count: {}", state.operation_count);
    println!("Final result: {}", final_result);

    println!("=== Ending foo function ===\n");

    final_result
}
