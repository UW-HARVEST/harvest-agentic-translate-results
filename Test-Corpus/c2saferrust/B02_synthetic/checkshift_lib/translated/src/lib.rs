







extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ComputeState {
    pub accumulator: ::core::ffi::c_int,
    pub operation_count: ::core::ffi::c_int,
    pub checksum: ::core::ffi::c_uint,
}
pub type operation_func =
    Option<unsafe extern "C" fn(::core::ffi::c_int, ::core::ffi::c_int) -> ::core::ffi::c_int>;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const MAGIC_NUMBER: ::core::ffi::c_uint = 0xdeadbeef as ::core::ffi::c_uint;
pub const MASK_LOWER: ::core::ffi::c_int = 0xffff as ::core::ffi::c_int;
static mut static_multiplier: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
static mut static_addend: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
static mut static_shift_amount: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn multiply_with_static(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a * b * static_multiplier;
}
#[no_mangle]
pub unsafe extern "C" fn add_with_static(a: ::core::ffi::c_int, b: ::core::ffi::c_int) -> ::core::ffi::c_int {
    a + b + static_addend
}

#[no_mangle]
pub unsafe extern "C" fn xor_operation(a: ::core::ffi::c_int, b: ::core::ffi::c_int) -> ::core::ffi::c_int {
    a ^ b ^ 0xabcd
}

#[no_mangle]
pub unsafe extern "C" fn shift_with_static(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a << static_shift_amount | b >> static_shift_amount;
}
#[no_mangle]
pub fn get_operation(opcode: i32) -> operation_func {
    let ops: [operation_func; 4] = [
        Some(multiply_with_static),
        Some(add_with_static),
        Some(xor_operation),
        Some(shift_with_static),
    ];

    if (0..4).contains(&opcode) {
        ops[opcode as usize]
    } else {
        None
    }
}

#[no_mangle]
pub fn execute_operation(
    func: operation_func,
    a: ::core::ffi::c_int,
    b: ::core::ffi::c_int,
    op_name: &str,
) -> ::core::ffi::c_int {
    match func {
        Some(func) => {
            println!("Variable a = {}", a);
            println!("Variable b = {}", b);

            let result = unsafe { func(a, b) };
            println!("Result of {}: {}", op_name, result);
            result
        }
        None => {
            println!("Error: Operation function pointer is NULL for {}", op_name);
            0
        }
    }
}

#[no_mangle]
pub fn compute_checksum(values: &[::core::ffi::c_int]) -> ::core::ffi::c_uint {
    let mut checksum: ::core::ffi::c_uint = 0;
    let mut buffer: [u8; 16] = [0; 16];

    if !values.is_empty() {
        let copy_count = values.len().min(4);
        let int_size = ::core::mem::size_of::<::core::ffi::c_int>();
        let byte_count = int_size * copy_count;

        for (i, value) in values.iter().take(copy_count).enumerate() {
            let bytes = value.to_ne_bytes();
            let start = i * int_size;
            buffer[start..start + int_size].copy_from_slice(&bytes);
        }

        let mut i = 0;
        while i < byte_count {
            checksum = (checksum << 1) ^ buffer[i] as ::core::ffi::c_uint;
            i += 1;
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER as ::core::ffi::c_uint
}

#[no_mangle]
pub fn init_state(state: Option<&mut ComputeState>, initial_value: ::core::ffi::c_int) {
    if let Some(state) = state {
        *state = ComputeState {
            accumulator: initial_value,
            operation_count: 0,
            checksum: 0,
        };
        println!("State initialized with accumulator = {}", state.accumulator);
    } else {
        println!("Error: state pointer is NULL in init_state");
    }
}

#[no_mangle]
pub fn apply_operation(
    state: Option<&mut ComputeState>,
    value: ::core::ffi::c_int,
    func: operation_func,
) {
    if state.is_none() {
        eprintln!("Error: state pointer is NULL in apply_operation");
        return;
    }
    if func.is_none() {
        eprintln!("Error: operation function pointer is NULL in apply_operation");
        return;
    }

    let state = state.unwrap();
    let func = func.unwrap();

    state.accumulator = unsafe { func(state.accumulator, value) };
    state.operation_count += 1;
}

#[no_mangle]
pub fn checkshift(
    param1: ::core::ffi::c_int,
    param2: ::core::ffi::c_int,
    param3: ::core::ffi::c_int,
    param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    println!("\n=== Starting foo function ===");
    println!("Parameters: {}, {}, {}, {}", param1, param2, param3, param4);

    let mut state = Box::new(ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    });

    init_state(Some(&mut *state), param1);

    let params = [param1, param2, param3, param4];

    let mult_op: operation_func = get_operation(0 as ::core::ffi::c_int);
    let add_op: operation_func = get_operation(1 as ::core::ffi::c_int);
    let xor_op: operation_func = get_operation(2 as ::core::ffi::c_int);
    let shift_op: operation_func = get_operation(3 as ::core::ffi::c_int);

    println!("\n--- Operation 1: Multiply ---");
    apply_operation(Some(&mut *state), param2, mult_op);

    println!("\n--- Operation 2: Add ---");
    apply_operation(Some(&mut *state), param3, add_op);

    println!("\n--- Operation 3: XOR ---");
    let xor_result: ::core::ffi::c_int =
        execute_operation(xor_op, state.accumulator, param4, "XOR");

    println!("\n--- Operation 4: Shift ---");
    let shift_result: ::core::ffi::c_int =
        execute_operation(shift_op, xor_result, param2, "SHIFT");

    state.checksum = compute_checksum(&params);

    println!("\nComputed checksum: 0x{:04X}", state.checksum);

    let final_result: ::core::ffi::c_int =
        (((state.accumulator + shift_result) as ::core::ffi::c_uint) ^ state.checksum)
            as ::core::ffi::c_int;

    println!("\nFinal accumulator: {}", state.accumulator);
    println!("Operation count: {}", state.operation_count);
    println!("Final result: {}", final_result);
    println!("=== Ending foo function ===\n");

    final_result
}

