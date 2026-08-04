use std::os::raw::c_int;
use std::sync::OnceLock;

type OperationFunc = fn(c_int, c_int) -> c_int;

const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const MASK_LOWER: u32 = 0x0000FFFF;

#[repr(C)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: u32,
}

static STATIC_MULTIPLIER: c_int = 3;
static STATIC_ADDEND: c_int = 100;
static STATIC_SHIFT_AMOUNT: u32 = 2;

fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    (a * b) * STATIC_MULTIPLIER
}

fn add_with_static(a: c_int, b: c_int) -> c_int {
    (a + b) + STATIC_ADDEND
}

fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

fn shift_with_static(a: c_int, b: c_int) -> c_int {
    let left = ((a as u32) << STATIC_SHIFT_AMOUNT) as c_int;
    let right = ((b as u32) >> STATIC_SHIFT_AMOUNT) as c_int;
    left | right
}

fn get_operation(opcode: c_int) -> Option<OperationFunc> {
    static OPS: OnceLock<[Option<OperationFunc>; 4]> = OnceLock::new();
    let ops = OPS.get_or_init(|| {
        [
            Some(multiply_with_static),
            Some(add_with_static),
            Some(xor_operation),
            Some(shift_with_static),
        ]
    });

    if (0..4).contains(&opcode) {
        ops[opcode as usize]
    } else {
        None
    }
}

fn execute_operation(func: Option<OperationFunc>, a: c_int, b: c_int, op_name: &str) -> c_int {
    let Some(func) = func else {
        println!("Error: Operation function pointer is NULL for {}", op_name);
        return 0;
    };

    println!("Variable a = {}", a);
    println!("Variable b = {}", b);

    let result = func(a, b);
    println!("Result of {}: {}", op_name, result);
    result
}

fn compute_checksum(values: Option<&[c_int]>, count: c_int) -> u32 {
    let mut checksum: u32 = 0;

    if let Some(values) = values {
        if count > 0 {
            let copy_count = usize::min(count as usize, usize::min(4, values.len()));
            let mut buffer = [0u8; std::mem::size_of::<c_int>() * 4];

            for (i, value) in values.iter().take(copy_count).enumerate() {
                let bytes = value.to_ne_bytes();
                let start = i * std::mem::size_of::<c_int>();
                let end = start + std::mem::size_of::<c_int>();
                buffer[start..end].copy_from_slice(&bytes);
            }

            for byte in &buffer[..std::mem::size_of::<c_int>() * copy_count] {
                checksum = (checksum << 1) ^ (*byte as u32);
            }

            checksum ^= MAGIC_NUMBER;
        }
    }

    checksum & MASK_LOWER
}

fn init_state(state: Option<&mut ComputeState>, initial_value: c_int) {
    let Some(state) = state else {
        println!("Error: state pointer is NULL in init_state");
        return;
    };

    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0x0000,
    };

    *state = template;
    println!("State initialized with accumulator = {}", state.accumulator);
}

fn apply_operation(state: Option<&mut ComputeState>, value: c_int, func: Option<OperationFunc>) {
    let Some(state) = state else {
        println!("Error: state pointer is NULL in apply_operation");
        return;
    };

    let Some(func) = func else {
        println!("Error: operation function pointer is NULL in apply_operation");
        return;
    };

    state.accumulator = func(state.accumulator, value);
    state.operation_count += 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn checkshift(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    println!("\n=== Starting foo function ===");
    println!("Parameters: {}, {}, {}, {}", param1, param2, param3, param4);

    let mut state = Box::new(ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    });

    init_state(Some(&mut state), param1);

    let params = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    println!("\n--- Operation 1: Multiply ---");
    apply_operation(Some(&mut state), param2, mult_op);

    println!("\n--- Operation 2: Add ---");
    apply_operation(Some(&mut state), param3, add_op);

    println!("\n--- Operation 3: XOR ---");
    let xor_result = execute_operation(xor_op, state.accumulator, param4, "XOR");

    println!("\n--- Operation 4: Shift ---");
    let shift_result = execute_operation(shift_op, xor_result, param2, "SHIFT");

    state.checksum = compute_checksum(Some(&params), 4);
    println!("\nComputed checksum: 0x{:04X}", state.checksum);

    let final_result = (state.accumulator + shift_result) ^ (state.checksum as c_int);

    println!("\nFinal accumulator: {}", state.accumulator);
    println!("Operation count: {}", state.operation_count);
    println!("Final result: {}", final_result);

    drop(state);

    println!("=== Ending foo function ===\n");

    final_result
}
