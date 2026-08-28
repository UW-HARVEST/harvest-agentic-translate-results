use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_long, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicUsize, Ordering};

type ModifierFn = unsafe extern "C" fn(c_int, c_int);
type OperationFn = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type ApplyFn = unsafe extern "C" fn(OperationFn, c_int, c_int, c_int) -> c_int;
type ShiftFn = unsafe extern "C" fn(*mut c_int, c_int, c_int);
type PointerFn = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type DynamicFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type TimeFn = unsafe extern "C" fn(c_int) -> c_int;
type RecordsFn = unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int;
type HatchFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: c_long,
    name: [c_char; 32],
}

#[derive(Clone, Copy)]
struct Api {
    increment_counter: ModifierFn,
    update_accumulator: ModifierFn,
    apply_operation: ApplyFn,
    add_three: OperationFn,
    multiply_add: OperationFn,
    complex_calc: OperationFn,
    shift_array_data: ShiftFn,
    process_pointer_data: PointerFn,
    compute_with_dynamic_memory: DynamicFn,
    get_time_based_value: TimeFn,
    manipulate_records: RecordsFn,
    hatch: HatchFn,
}

impl Api {
    unsafe fn load(library: &Library) -> Self {
        unsafe {
            Self {
                increment_counter: *library.get(b"increment_counter\0").unwrap(),
                update_accumulator: *library.get(b"update_accumulator\0").unwrap(),
                apply_operation: *library.get(b"apply_operation\0").unwrap(),
                add_three: *library.get(b"add_three\0").unwrap(),
                multiply_add: *library.get(b"multiply_add\0").unwrap(),
                complex_calc: *library.get(b"complex_calc\0").unwrap(),
                shift_array_data: *library.get(b"shift_array_data\0").unwrap(),
                process_pointer_data: *library.get(b"process_pointer_data\0").unwrap(),
                compute_with_dynamic_memory: *library
                    .get(b"compute_with_dynamic_memory\0")
                    .unwrap(),
                get_time_based_value: *library.get(b"get_time_based_value\0").unwrap(),
                manipulate_records: *library.get(b"manipulate_records\0").unwrap(),
                hatch: *library.get(b"hatch\0").unwrap(),
            }
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
    _c_library: Library,
    _rust_library: Library,
}

static COPY_ID: AtomicUsize = AtomicUsize::new(0);

fn shared_objects() -> (PathBuf, PathBuf) {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        crate_dir.join("../c_src/build/libharvest-work-I5OVhX.so"),
        crate_dir.join("target/release/libhatch_lib.so"),
    )
}

impl Pair {
    fn fresh() -> Self {
        let (c_source, rust_source) = shared_objects();
        assert!(c_source.is_file(), "missing C shared object: {c_source:?}");
        assert!(
            rust_source.is_file(),
            "missing Rust shared object: {rust_source:?}"
        );

        let id = COPY_ID.fetch_add(1, Ordering::Relaxed);
        let directory =
            env::temp_dir().join(format!("hatch-differential-{}-{id}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let c_copy = directory.join("c.so");
        let rust_copy = directory.join("rust.so");
        fs::copy(c_source, &c_copy).unwrap();
        fs::copy(rust_source, &rust_copy).unwrap();

        let c_library = unsafe { Library::new(&c_copy).unwrap() };
        let rust_library = unsafe { Library::new(&rust_copy).unwrap() };
        let c = unsafe { Api::load(&c_library) };
        let rust = unsafe { Api::load(&rust_library) };

        // Linux keeps mapped objects alive after unlink, while each unique path
        // gives this test a fresh copy of the C and Rust global state.
        fs::remove_file(c_copy).unwrap();
        fs::remove_file(rust_copy).unwrap();
        fs::remove_dir(directory).unwrap();

        Self {
            c,
            rust,
            _c_library: c_library,
            _rust_library: rust_library,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn i32(&mut self, minimum: i32, maximum: i32) -> i32 {
        let width = (maximum as i64 - minimum as i64 + 1) as u64;
        minimum + (self.next_u32() as u64 % width) as i32
    }

    fn any_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

fn assert_scalar(context: &str, c: c_int, rust: c_int) {
    assert_eq!(
        c.to_ne_bytes(),
        rust.to_ne_bytes(),
        "{context}: C returned {c}, Rust returned {rust}"
    );
}

unsafe fn bytes<T>(values: &[T]) -> &[u8] {
    unsafe { slice::from_raw_parts(values.as_ptr().cast(), size_of_val(values)) }
}

fn random_records(rng: &mut Rng, count: usize) -> Vec<DataRecord> {
    (0..count)
        .map(|index| {
            let mut name = [0; 32];
            for byte in &mut name {
                *byte = rng.next_u32() as c_char;
            }
            DataRecord {
                id: index as c_int,
                value: rng.i32(-100_000, 100_000),
                timestamp: rng.any_i32() as c_long,
                name,
            }
        })
        .collect()
}

unsafe fn compare_shift(pair: &Pair, source: &[c_int], size: c_int, shift: c_int) {
    let mut c_values = source.to_vec();
    let mut rust_values = source.to_vec();
    unsafe {
        (pair.c.shift_array_data)(c_values.as_mut_ptr(), size, shift);
        (pair.rust.shift_array_data)(rust_values.as_mut_ptr(), size, shift);
        assert_eq!(
            bytes(&c_values),
            bytes(&rust_values),
            "shift_array_data size={size}, shift={shift}"
        );
    }
}

unsafe fn compare_records(
    pair: &Pair,
    source: &[DataRecord],
    num_records: c_int,
    shift: c_int,
) -> c_int {
    let mut c_records = source.to_vec();
    let mut rust_records = source.to_vec();
    let c_result =
        unsafe { (pair.c.manipulate_records)(c_records.as_mut_ptr(), num_records, shift) };
    let rust_result =
        unsafe { (pair.rust.manipulate_records)(rust_records.as_mut_ptr(), num_records, shift) };
    assert_scalar("manipulate_records result", c_result, rust_result);
    unsafe {
        assert_eq!(
            bytes(&c_records),
            bytes(&rust_records),
            "manipulate_records bytes num_records={num_records}, shift={shift}"
        );
    }
    c_result
}

#[test]
fn differential_surface() {
    unsafe {
        verify_hatch();
        verify_arithmetic_callbacks_and_state();
        verify_array_shapes_and_range_rejections();
        verify_dynamic_memory_and_time();
        verify_record_shapes_and_range_rejections();
        verify_generic_boundaries();
        verify_unchecked_pointer_outcomes();
    }
}

unsafe fn verify_hatch() {
    let mut rng = Rng::new(0x68a4_9d13_f10c_72be);

    // CONFIGS row 27: many genuinely fresh first calls.
    for case in 0..64 {
        let pair = Pair::fresh();
        let arguments = (
            rng.i32(-1_000, 1_000),
            rng.i32(-1_000, 1_000),
            rng.i32(-100_000, 100_000),
            rng.i32(-1_000, 1_000),
        );
        let c = unsafe { (pair.c.hatch)(arguments.0, arguments.1, arguments.2, arguments.3) };
        let rust = unsafe { (pair.rust.hatch)(arguments.0, arguments.1, arguments.2, arguments.3) };
        assert_scalar(&format!("hatch fresh case {case}"), c, rust);
    }

    // CONFIGS row 28: repeated calls exercise both accumulating globals.
    let pair = Pair::fresh();
    for case in 0..256 {
        let arguments = (
            rng.i32(-1_000, 1_000),
            rng.i32(-1_000, 1_000),
            rng.i32(-100_000, 100_000),
            rng.i32(-1_000, 1_000),
        );
        let c = unsafe { (pair.c.hatch)(arguments.0, arguments.1, arguments.2, arguments.3) };
        let rust = unsafe { (pair.rust.hatch)(arguments.0, arguments.1, arguments.2, arguments.3) };
        assert_scalar(&format!("hatch repeated case {case}"), c, rust);
    }
}

unsafe fn verify_arithmetic_callbacks_and_state() {
    let pair = Pair::fresh();
    let mut rng = Rng::new(0xa34f_31ce_1dd8_0287);

    // CONFIGS rows 1 and 5: mutate and observe counter state.
    let mut expected_counter = 0i32;
    let mut counter_values = vec![0, 1, -1, 71, -29];
    counter_values.extend((0..256).map(|_| rng.i32(-10_000, 10_000)));
    for value in counter_values {
        let unused = rng.any_i32();
        unsafe {
            (pair.c.increment_counter)(value, unused);
            (pair.rust.increment_counter)(value, unused);
        }
        expected_counter = expected_counter.wrapping_add(value);
        let c = unsafe { (pair.c.complex_calc)(0, 0, 0) };
        let rust = unsafe { (pair.rust.complex_calc)(0, 0, 0) };
        assert_scalar("increment_counter observed via complex_calc", c, rust);
        assert_eq!(c, expected_counter);
    }
    unsafe {
        (pair.c.increment_counter)(17, 0);
        (pair.rust.increment_counter)(17, 0);
    }

    // CONFIGS row 2: mutate and observe accumulator state.
    let mut expected_accumulator = 0i32;
    let mut accumulator_values = vec![0, 1, -1, 83, -41];
    accumulator_values.extend((0..256).map(|_| rng.i32(-10_000, 10_000)));
    for value in accumulator_values {
        let unused = rng.any_i32();
        unsafe {
            (pair.c.update_accumulator)(value, unused);
            (pair.rust.update_accumulator)(value, unused);
        }
        expected_accumulator = expected_accumulator.wrapping_mul(2).wrapping_add(value);
        let mut zero = 0;
        let c = unsafe { (pair.c.process_pointer_data)(&mut zero, 0) };
        let rust = unsafe { (pair.rust.process_pointer_data)(&mut zero, 0) };
        assert_scalar(
            "update_accumulator observed via process_pointer_data",
            c,
            rust,
        );
        assert_eq!(c, expected_accumulator);
    }
    unsafe {
        (pair.c.update_accumulator)(19, 0);
        (pair.rust.update_accumulator)(19, 0);
    }

    // CONFIGS rows 3-8 and 15.
    for case in 0..512 {
        let a = rng.i32(-10_000, 10_000);
        let b = rng.i32(-10_000, 10_000);
        let c = rng.i32(-10_000, 10_000);

        assert_scalar(
            "add_three",
            unsafe { (pair.c.add_three)(a, b, c) },
            unsafe { (pair.rust.add_three)(a, b, c) },
        );
        assert_scalar(
            "multiply_add",
            unsafe { (pair.c.multiply_add)(a, b, c) },
            unsafe { (pair.rust.multiply_add)(a, b, c) },
        );
        assert_scalar(
            "complex_calc",
            unsafe { (pair.c.complex_calc)(a, b, c) },
            unsafe { (pair.rust.complex_calc)(a, b, c) },
        );
        assert_scalar(
            "apply_operation(add_three)",
            unsafe { (pair.c.apply_operation)(pair.c.add_three, a, b, c) },
            unsafe { (pair.rust.apply_operation)(pair.rust.add_three, a, b, c) },
        );
        assert_scalar(
            "apply_operation(multiply_add)",
            unsafe { (pair.c.apply_operation)(pair.c.multiply_add, a, b, c) },
            unsafe { (pair.rust.apply_operation)(pair.rust.multiply_add, a, b, c) },
        );
        assert_scalar(
            "apply_operation(complex_calc)",
            unsafe { (pair.c.apply_operation)(pair.c.complex_calc, a, b, c) },
            unsafe { (pair.rust.apply_operation)(pair.rust.complex_calc, a, b, c) },
        );

        let mut pointed_value = rng.i32(-10_000, 10_000);
        let multiplier = rng.i32(-10_000, 10_000);
        assert_scalar(
            &format!("process_pointer_data case {case}"),
            unsafe { (pair.c.process_pointer_data)(&mut pointed_value, multiplier) },
            unsafe { (pair.rust.process_pointer_data)(&mut pointed_value, multiplier) },
        );
    }
}

unsafe fn verify_array_shapes_and_range_rejections() {
    let pair = Pair::fresh();
    let mut rng = Rng::new(0xf0e1_d2c3_b4a5_9687);

    for _ in 0..256 {
        let values = (0..16).map(|_| rng.any_i32()).collect::<Vec<_>>();

        // CONFIGS rows 9-12: all one/many retained/zeroed cross-products.
        let many_size = rng.i32(3, 16);
        unsafe {
            compare_shift(&pair, &values[..2], 2, 1);
            compare_shift(&pair, &values[..many_size as usize], many_size, 1);
        }

        let size = rng.i32(4, 16);
        unsafe {
            compare_shift(&pair, &values[..size as usize], size, size - 1);
        }
        let shift = rng.i32(2, size - 2);
        unsafe {
            compare_shift(&pair, &values[..size as usize], size, shift);
        }

        // CONFIGS rows 13-14 and ERRORS rows 1-2: every inactive branch is
        // byte-preserving, including zero/one/many buffer shapes.
        for length in [0usize, 1, rng.i32(2, 16) as usize] {
            let before = values[..length].to_vec();
            for shift in [0, -1, -17] {
                let mut c_values = before.clone();
                let mut rust_values = before.clone();
                unsafe {
                    (pair.c.shift_array_data)(c_values.as_mut_ptr(), length as c_int, shift);
                    (pair.rust.shift_array_data)(rust_values.as_mut_ptr(), length as c_int, shift);
                }
                assert_eq!(c_values, before);
                assert_eq!(rust_values, before);
            }
            for shift in [length as c_int, length as c_int + 1] {
                let mut c_values = before.clone();
                let mut rust_values = before.clone();
                unsafe {
                    (pair.c.shift_array_data)(c_values.as_mut_ptr(), length as c_int, shift);
                    (pair.rust.shift_array_data)(rust_values.as_mut_ptr(), length as c_int, shift);
                }
                assert_eq!(c_values, before);
                assert_eq!(rust_values, before);
            }
        }
    }
}

unsafe fn verify_dynamic_memory_and_time() {
    let pair = Pair::fresh();
    let mut rng = Rng::new(0x4189_72ab_c35d_e60f);

    // CONFIGS rows 16-19: zero, one, many, and negative loop counts.
    for case in 0..256 {
        let base = rng.i32(-1_000_000, 1_000_000);
        for count in [0, 1, rng.i32(2, 128), -rng.i32(1, 1_000_000)] {
            assert_scalar(
                &format!("compute_with_dynamic_memory case {case}, count={count}"),
                unsafe { (pair.c.compute_with_dynamic_memory)(base, count) },
                unsafe { (pair.rust.compute_with_dynamic_memory)(base, count) },
            );
        }
    }

    // CONFIGS row 20 plus one-step-past arithmetic boundaries for Phase C.
    let boundary = c_int::MAX / 3600;
    let mut seeds = vec![-boundary - 1, -boundary, -1, 0, 1, boundary, boundary + 1];
    seeds.extend((0..512).map(|_| rng.i32(-boundary, boundary)));
    for seed in seeds {
        assert_scalar(
            &format!("get_time_based_value seed={seed}"),
            unsafe { (pair.c.get_time_based_value)(seed) },
            unsafe { (pair.rust.get_time_based_value)(seed) },
        );
    }
}

unsafe fn verify_record_shapes_and_range_rejections() {
    let pair = Pair::fresh();
    let mut rng = Rng::new(0x0123_4567_89ab_cdef);

    for _ in 0..256 {
        // CONFIGS row 21: active move with one remaining record.
        let count = rng.i32(2, 16);
        let records = random_records(&mut rng, count as usize);
        unsafe {
            compare_records(&pair, &records, count, count - 1);
        }

        // CONFIGS row 22: active move with many remaining records.
        let count = rng.i32(3, 16);
        let shift = rng.i32(1, count - 2);
        let records = random_records(&mut rng, count as usize);
        unsafe {
            compare_records(&pair, &records, count, shift);
        }

        // CONFIGS row 23 and ERRORS row 3 lower boundary.
        for count in [1, rng.i32(2, 16)] {
            let records = random_records(&mut rng, count as usize);
            let expected = records
                .iter()
                .fold(0i32, |sum, record| sum.wrapping_add(record.value));
            let result = unsafe { compare_records(&pair, &records, count, 0) };
            assert_eq!(result, expected);
        }

        // CONFIGS row 24 and ERRORS row 3 lower range. The backing allocation
        // deliberately includes every record the unusual C loop reads.
        let count = rng.i32(1, 12);
        let shift = -rng.i32(1, 4);
        let records = random_records(&mut rng, (count - shift) as usize);
        let expected = records
            .iter()
            .fold(0i32, |sum, record| sum.wrapping_add(record.value));
        let result = unsafe { compare_records(&pair, &records, count, shift) };
        assert_eq!(result, expected);

        // CONFIGS rows 25-26 and ERRORS row 4.
        let mut empty: Vec<DataRecord> = Vec::new();
        let c_empty = unsafe { (pair.c.manipulate_records)(empty.as_mut_ptr(), 0, 0) };
        let rust_empty = unsafe { (pair.rust.manipulate_records)(empty.as_mut_ptr(), 0, 0) };
        assert_scalar("manipulate_records empty", c_empty, rust_empty);
        assert_eq!(c_empty, 0);

        let count = rng.i32(1, 16);
        let records = random_records(&mut rng, count as usize);
        for shift in [count, count + 1, count + rng.i32(2, 100)] {
            let result = unsafe { compare_records(&pair, &records, count, shift) };
            assert_eq!(result, 0);
        }
    }
}

unsafe fn verify_generic_boundaries() {
    let pair = Pair::fresh();

    // Null pointers are accepted when the C branch and loop bounds guarantee
    // they are not dereferenced.
    unsafe {
        (pair.c.shift_array_data)(ptr::null_mut(), 0, 0);
        (pair.rust.shift_array_data)(ptr::null_mut(), 0, 0);
    }
    assert_scalar(
        "null records with zero length",
        unsafe { (pair.c.manipulate_records)(ptr::null_mut(), 0, 0) },
        unsafe { (pair.rust.manipulate_records)(ptr::null_mut(), 0, 0) },
    );

    // Zero, negative, and maximal lengths that select a non-dereferencing C
    // path. There are no public enum parameters in this library.
    let mut sentinel = 0x1357_2468;
    unsafe {
        (pair.c.shift_array_data)(&mut sentinel, c_int::MAX, c_int::MAX);
        (pair.rust.shift_array_data)(&mut sentinel, c_int::MAX, c_int::MAX);
    }
    assert_scalar(
        "negative dynamic count",
        unsafe { (pair.c.compute_with_dynamic_memory)(123, -1) },
        unsafe { (pair.rust.compute_with_dynamic_memory)(123, -1) },
    );
    assert_scalar(
        "maximal record length with equal shift",
        unsafe { (pair.c.manipulate_records)(ptr::null_mut(), c_int::MAX, c_int::MAX) },
        unsafe { (pair.rust.manipulate_records)(ptr::null_mut(), c_int::MAX, c_int::MAX) },
    );
}

unsafe fn verify_unchecked_pointer_outcomes() {
    use std::os::unix::process::ExitStatusExt;

    for case in [
        "null-callback",
        "null-data-pointer",
        "null-active-shift",
        "null-records",
    ] {
        let c = probe_status("c", case);
        let rust = probe_status("rust", case);
        assert_eq!(
            c.signal(),
            rust.signal(),
            "{case}: C status {c:?}, Rust status {rust:?}"
        );
        assert!(
            c.signal().is_some(),
            "{case}: unchecked C pointer unexpectedly returned normally"
        );
    }
}

fn probe_status(implementation: &str, case: &str) -> std::process::ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("crash_probe")
        .arg("--nocapture")
        .env("HATCH_CRASH_IMPLEMENTATION", implementation)
        .env("HATCH_CRASH_CASE", case)
        .output()
        .unwrap()
        .status
}

#[test]
fn crash_probe() {
    let Ok(implementation) = env::var("HATCH_CRASH_IMPLEMENTATION") else {
        return;
    };
    let case = env::var("HATCH_CRASH_CASE").unwrap();
    let (c_path, rust_path) = shared_objects();
    let path = if implementation == "c" {
        c_path
    } else {
        rust_path
    };
    let library = unsafe { Library::new(path).unwrap() };

    unsafe {
        match case.as_str() {
            "null-callback" => {
                type RawApply = unsafe extern "C" fn(*const c_void, c_int, c_int, c_int) -> c_int;
                let function = *library.get::<RawApply>(b"apply_operation\0").unwrap();
                function(ptr::null(), 1, 2, 3);
            }
            "null-data-pointer" => {
                let function = *library.get::<PointerFn>(b"process_pointer_data\0").unwrap();
                function(ptr::null_mut(), 7);
            }
            "null-active-shift" => {
                let function = *library.get::<ShiftFn>(b"shift_array_data\0").unwrap();
                function(ptr::null_mut(), 2, 1);
            }
            "null-records" => {
                let function = *library.get::<RecordsFn>(b"manipulate_records\0").unwrap();
                function(ptr::null_mut(), 1, 0);
            }
            _ => panic!("unknown crash probe: {case}"),
        }
    }
}
