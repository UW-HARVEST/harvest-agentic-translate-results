use std::ffi::c_int;

const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const MASK_LOWER: u32 = 0x0000FFFF;

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
    (a.wrapping_mul(b)).wrapping_mul(STATIC_MULTIPLIER)
}

fn add_with_static(a: c_int, b: c_int) -> c_int {
    (a.wrapping_add(b)).wrapping_add(STATIC_ADDEND)
}

fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCDu32 as c_int
}

fn shift_with_static(a: c_int, b: c_int) -> c_int {
    (a << STATIC_SHIFT_AMOUNT) | ((b as u32 >> STATIC_SHIFT_AMOUNT) as c_int)
}

fn get_operation(opcode: c_int) -> Option<OperationFunc> {
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
    a: c_int,
    b: c_int,
    op_name: &str,
) -> c_int {
    match func {
        None => {
            print!("Error: Operation function pointer is NULL for {}\n", op_name);
            0
        }
        Some(f) => {
            print!("Variable a = {}\n", a);
            print!("Variable b = {}\n", b);
            let result = f(a, b);
            print!("Result of {}: {}\n", op_name, result);
            result
        }
    }
}

fn compute_checksum(values: &[c_int; 4], count: c_int) -> u32 {
    let mut checksum: u32 = 0;

    if count > 0 {
        let copy_count = if count > 4 { 4 } else { count as usize };
        // Reinterpret the int array as bytes, matching C memcpy behavior
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                values.as_ptr() as *const u8,
                std::mem::size_of::<c_int>() * copy_count,
            )
        };

        for &byte in bytes {
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
    print!("State initialized with accumulator = {}\n", state.accumulator);
}

fn apply_operation(state: &mut ComputeState, value: c_int, func: OperationFunc) {
    state.accumulator = func(state.accumulator, value);
    state.operation_count += 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn checkshift(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    print!("\n=== Starting foo function ===\n");
    print!("Parameters: {}, {}, {}, {}\n", param1, param2, param3, param4);

    let mut state = ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    };

    init_state(&mut state, param1);

    let params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0).unwrap();
    let add_op = get_operation(1).unwrap();
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

    state.checksum = compute_checksum(&params, 4);
    print!("\nComputed checksum: 0x{:04X}\n", state.checksum);

    let final_result = (state.accumulator.wrapping_add(shift_result)) ^ (state.checksum as c_int);

    print!("\nFinal accumulator: {}\n", state.accumulator);
    print!("Operation count: {}\n", state.operation_count);
    print!("Final result: {}\n", final_result);

    print!("=== Ending foo function ===\n\n");

    final_result
}
