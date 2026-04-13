use std::os::raw::c_int;
use std::sync::Mutex;

const OP_ADD: u8 = 0x01;
const OP_MULTIPLY: u8 = 0x02;
const OP_XOR: u8 = 0x03;
const OP_SHIFT: u8 = 0x04;
const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const MASK_LOWER: u32 = 0x0000FFFF;

static STATIC_MULTIPLIER: Mutex<i32> = Mutex::new(3);
static STATIC_ADDEND: Mutex<i32> = Mutex::new(100);
static STATIC_SHIFT_AMOUNT: Mutex<i32> = Mutex::new(2);

type OperationFunc = fn(i32, i32) -> i32;

struct ComputeState {
    accumulator: i32,
    operation_count: i32,
    checksum: u32,
}

fn multiply_with_static(a: i32, b: i32) -> i32 {
    (a * b) * *STATIC_MULTIPLIER.lock().unwrap()
}

fn add_with_static(a: i32, b: i32) -> i32 {
    (a + b) + *STATIC_ADDEND.lock().unwrap()
}

fn xor_operation(a: i32, b: i32) -> i32 {
    a ^ b ^ 0xABCD
}

fn shift_with_static(a: i32, b: i32) -> i32 {
    let shift = *STATIC_SHIFT_AMOUNT.lock().unwrap();
    (a << shift) | (b >> shift)
}

fn get_operation(opcode: i32) -> Option<OperationFunc> {
    static mut OPS: [Option<OperationFunc>; 4] = [None, None, None, None];
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| {
        unsafe {
            OPS[0] = Some(multiply_with_static);
            OPS[1] = Some(add_with_static);
            OPS[2] = Some(xor_operation);
            OPS[3] = Some(shift_with_static);
        }
    });

    if opcode >= 0 && opcode < 4 {
        unsafe { OPS[opcode as usize] }
    } else {
        None
    }
}

fn execute_operation(func: Option<OperationFunc>, a: i32, b: i32, op_name: &str) -> i32 {
    let func = match func {
        Some(f) => f,
        None => {
            println!("Error: Operation function pointer is NULL for {}", op_name);
            return 0;
        }
    };

    println!("Variable a = {}", a);
    println!("Variable b = {}", b);

    let result = func(a, b);
    println!("Result of {}: {}", op_name, result);

    result
}

fn compute_checksum(values: &[i32]) -> u32 {
    let mut checksum: u32 = 0;

    if !values.is_empty() {
        let copy_count = values.len().min(4);
        let buffer: Vec<u8> = values[..copy_count]
            .iter()
            .flat_map(|&v| v.to_ne_bytes())
            .collect();

        for &byte in &buffer {
            checksum = (checksum << 1) ^ (byte as u32);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

fn init_state(state: &mut ComputeState, initial_value: i32) {
    state.accumulator = initial_value;
    state.operation_count = 0;
    state.checksum = 0x0000;

    println!("State initialized with accumulator = {}", state.accumulator);
}

fn apply_operation(state: &mut ComputeState, value: i32, func: Option<OperationFunc>) {
    let func = match func {
        Some(f) => f,
        None => {
            println!("Error: operation function pointer is NULL in apply_operation");
            return;
        }
    };

    state.accumulator = func(state.accumulator, value);
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

    let final_result = (state.accumulator + shift_result) ^ (state.checksum as i32);

    println!("\nFinal accumulator: {}", state.accumulator);
    println!("Operation count: {}", state.operation_count);
    println!("Final result: {}", final_result);

    println!("=== Ending foo function ===\n");

    final_result
}
