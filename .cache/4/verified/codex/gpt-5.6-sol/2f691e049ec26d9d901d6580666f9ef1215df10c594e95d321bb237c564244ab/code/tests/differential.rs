use libloading::Library;
use std::ffi::{c_int, c_long};
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());
const RANDOM_CASES: usize = 64;

type Operation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type Modifier = unsafe extern "C" fn(c_int, c_int);
type ApplyOperation = unsafe extern "C" fn(Operation, c_int, c_int, c_int) -> c_int;
type ShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
type ProcessPointer = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type ComputeDynamic = unsafe extern "C" fn(c_int, c_int) -> c_int;
type TimeBased = unsafe extern "C" fn(c_int) -> c_int;
type ManipulateRecords = unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int;
type Hatch = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[derive(Clone, Copy)]
#[repr(C)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: c_long,
    name: [i8; 32],
}

struct Api {
    _library: Library,
    increment_counter: Modifier,
    update_accumulator: Modifier,
    apply_operation: ApplyOperation,
    add_three: Operation,
    multiply_add: Operation,
    complex_calc: Operation,
    shift_array_data: ShiftArray,
    process_pointer_data: ProcessPointer,
    compute_with_dynamic_memory: ComputeDynamic,
    get_time_based_value: TimeBased,
    manipulate_records: ManipulateRecords,
    hatch: Hatch,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("failed to load {}: {error}", $name))
            };
        }

        Self {
            increment_counter: symbol!("increment_counter", Modifier),
            update_accumulator: symbol!("update_accumulator", Modifier),
            apply_operation: symbol!("apply_operation", ApplyOperation),
            add_three: symbol!("add_three", Operation),
            multiply_add: symbol!("multiply_add", Operation),
            complex_calc: symbol!("complex_calc", Operation),
            shift_array_data: symbol!("shift_array_data", ShiftArray),
            process_pointer_data: symbol!("process_pointer_data", ProcessPointer),
            compute_with_dynamic_memory: symbol!("compute_with_dynamic_memory", ComputeDynamic),
            get_time_based_value: symbol!("get_time_based_value", TimeBased),
            manipulate_records: symbol!("manipulate_records", ManipulateRecords),
            hatch: symbol!("hatch", Hatch),
            _library: library,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
    directory: PathBuf,
}

impl Pair {
    fn load(tag: &str) -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target = manifest.join("target");
        let directory = target.join(format!("differential-{}-{tag}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();

        let c_source = manifest.join("c_src/build/libtranslated_rust.so");
        let rust_source = target.join("debug/libhatch_lib.so");
        assert!(
            c_source.is_file(),
            "missing C library {}; build c_src first",
            c_source.display()
        );
        assert!(
            rust_source.is_file(),
            "missing Rust library {}; run cargo build first",
            rust_source.display()
        );

        let c_copy = directory.join("libground_truth.so");
        let rust_copy = directory.join("libtranslation.so");
        fs::copy(&c_source, &c_copy).unwrap();
        fs::copy(&rust_source, &rust_copy).unwrap();

        Self {
            c: unsafe { Api::load(&c_copy) },
            rust: unsafe { Api::load(&rust_copy) },
            directory,
        }
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        // Libraries are still open here, so cleanup is best-effort on platforms
        // that permit unlinking loaded shared objects.
        let _ = fs::remove_dir_all(&self.directory);
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn int(&mut self, min: c_int, max: c_int) -> c_int {
        let width = (i64::from(max) - i64::from(min) + 1) as u64;
        (i64::from(min) + u64::from(self.next_u32()).wrapping_rem(width) as i64) as c_int
    }
}

fn assert_call3(
    label: &str,
    c_function: Operation,
    rust_function: Operation,
    a: c_int,
    b: c_int,
    c: c_int,
) {
    let expected = unsafe { c_function(a, b, c) };
    let actual = unsafe { rust_function(a, b, c) };
    assert_eq!(actual, expected, "{label}({a}, {b}, {c})");
}

fn random_records(rng: &mut Rng, count: usize) -> Vec<DataRecord> {
    (0..count)
        .map(|index| {
            let mut name = [0_i8; 32];
            for byte in &mut name {
                *byte = rng.int(0, 127) as i8;
            }
            DataRecord {
                id: index as c_int,
                value: rng.int(-10_000, 10_000),
                timestamp: i64::from(rng.int(-1_000_000, 1_000_000)) as c_long,
                name,
            }
        })
        .collect()
}

fn record_bytes(records: &[DataRecord]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            records.as_ptr().cast::<u8>(),
            std::mem::size_of_val(records),
        )
    }
}

fn compare_shift(pair: &Pair, rng: &mut Rng, size: usize, shift: c_int, label: &str) {
    let original: Vec<c_int> = (0..size).map(|_| rng.int(-100_000, 100_000)).collect();
    let mut c_values = original.clone();
    let mut rust_values = original;
    unsafe {
        (pair.c.shift_array_data)(c_values.as_mut_ptr(), size as c_int, shift);
        (pair.rust.shift_array_data)(rust_values.as_mut_ptr(), size as c_int, shift);
    }
    assert_eq!(rust_values, c_values, "{label}: size={size}, shift={shift}");
}

fn compare_records(
    pair: &Pair,
    records: Vec<DataRecord>,
    num_records: c_int,
    shift: c_int,
    label: &str,
) {
    let mut c_records = records.clone();
    let mut rust_records = records;
    let expected =
        unsafe { (pair.c.manipulate_records)(c_records.as_mut_ptr(), num_records, shift) };
    let actual =
        unsafe { (pair.rust.manipulate_records)(rust_records.as_mut_ptr(), num_records, shift) };
    assert_eq!(actual, expected, "{label}: return value");
    assert_eq!(
        record_bytes(&rust_records),
        record_bytes(&c_records),
        "{label}: record bytes"
    );
}

#[test]
fn valid_configuration_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let pair = Pair::load("valid-low-level");
    let mut rng = Rng::new(0x6d5a_56da_7c31_9e27);

    // C03-C05 and C17: branchless arithmetic and initial global state.
    for _ in 0..RANDOM_CASES {
        let (a, b, c) = (
            rng.int(-1_000, 1_000),
            rng.int(-1_000, 1_000),
            rng.int(-1_000, 1_000),
        );
        assert_call3(
            "C03 add_three",
            pair.c.add_three,
            pair.rust.add_three,
            a,
            b,
            c,
        );
        assert_call3(
            "C04 multiply_add",
            pair.c.multiply_add,
            pair.rust.multiply_add,
            a,
            b,
            c,
        );
        assert_call3(
            "C05 complex_calc",
            pair.c.complex_calc,
            pair.rust.complex_calc,
            a,
            b,
            c,
        );

        let mut value = rng.int(-100_000, 100_000);
        let multiplier = rng.int(-1_000, 1_000);
        let expected = unsafe { (pair.c.process_pointer_data)(&mut value, multiplier) };
        let actual = unsafe { (pair.rust.process_pointer_data)(&mut value, multiplier) };
        assert_eq!(actual, expected, "C17 process_pointer_data");
    }

    // C07-C08: callback dispatch through function pointers from each .so.
    for _ in 0..RANDOM_CASES {
        let (a, b, c) = (
            rng.int(-1_000, 1_000),
            rng.int(-1_000, 1_000),
            rng.int(-1_000, 1_000),
        );
        let expected = unsafe { (pair.c.apply_operation)(pair.c.add_three, a, b, c) };
        let actual = unsafe { (pair.rust.apply_operation)(pair.rust.add_three, a, b, c) };
        assert_eq!(actual, expected, "C07 apply_operation(add_three)");

        let expected = unsafe { (pair.c.apply_operation)(pair.c.multiply_add, a, b, c) };
        let actual = unsafe { (pair.rust.apply_operation)(pair.rust.multiply_add, a, b, c) };
        assert_eq!(actual, expected, "C08 apply_operation(multiply_add)");
    }

    // C01, C06, and C09: accumulated counter state.
    for _ in 0..RANDOM_CASES {
        let value = rng.int(-100, 100);
        let ignored = rng.int(c_int::MIN, c_int::MAX);
        unsafe {
            (pair.c.increment_counter)(value, ignored);
            (pair.rust.increment_counter)(value, ignored);
        }
        let (a, b, c) = (
            rng.int(-1_000, 1_000),
            rng.int(-1_000, 1_000),
            rng.int(-1_000, 1_000),
        );
        assert_call3(
            "C06 complex_calc",
            pair.c.complex_calc,
            pair.rust.complex_calc,
            a,
            b,
            c,
        );
        let expected = unsafe { (pair.c.apply_operation)(pair.c.complex_calc, a, b, c) };
        let actual = unsafe { (pair.rust.apply_operation)(pair.rust.complex_calc, a, b, c) };
        assert_eq!(actual, expected, "C09 apply_operation(complex_calc)");
    }

    // C02 and C18: keep the accumulator bounded while varying every update.
    let mut c_accumulator = 0_i32;
    for _ in 0..RANDOM_CASES {
        let desired = rng.int(-10_000, 10_000);
        let value = desired - 2 * c_accumulator;
        let ignored = rng.int(c_int::MIN, c_int::MAX);
        unsafe {
            (pair.c.update_accumulator)(value, ignored);
            (pair.rust.update_accumulator)(value, ignored);
        }
        c_accumulator = desired;

        let mut pointed = rng.int(-100_000, 100_000);
        let multiplier = rng.int(-1_000, 1_000);
        let expected = unsafe { (pair.c.process_pointer_data)(&mut pointed, multiplier) };
        let actual = unsafe { (pair.rust.process_pointer_data)(&mut pointed, multiplier) };
        assert_eq!(actual, expected, "C18 process_pointer_data");
    }

    // C10-C16: every branch and boundary of the shift guard.
    unsafe {
        (pair.c.shift_array_data)(ptr::null_mut(), 0, 0);
        (pair.rust.shift_array_data)(ptr::null_mut(), 0, 0);
    }
    for _ in 0..RANDOM_CASES {
        compare_shift(&pair, &mut rng, 1, 0, "C11");
        compare_shift(&pair, &mut rng, 12, 1, "C12");
        compare_shift(&pair, &mut rng, 12, 5, "C13");
        compare_shift(&pair, &mut rng, 12, 11, "C14");
        compare_shift(&pair, &mut rng, 12, -1, "C15");
        let rejected_shift = 12 + rng.int(0, 20);
        compare_shift(&pair, &mut rng, 12, rejected_shift, "C16");
    }

    // C19-C21: empty, one-element, and many-element generated arrays.
    for _ in 0..RANDOM_CASES {
        let base = rng.int(-10_000, 10_000);
        for (label, count) in [("C19", 0), ("C20", 1), ("C21", rng.int(2, 64))] {
            let expected = unsafe { (pair.c.compute_with_dynamic_memory)(base, count) };
            let actual = unsafe { (pair.rust.compute_with_dynamic_memory)(base, count) };
            assert_eq!(actual, expected, "{label}: base={base}, count={count}");
        }
    }

    // C22: time is sampled internally, but both outputs depend only on seed.
    for _ in 0..RANDOM_CASES {
        let seed = rng.int(-500_000, 500_000);
        let expected = unsafe { (pair.c.get_time_based_value)(seed) };
        let actual = unsafe { (pair.rust.get_time_based_value)(seed) };
        assert_eq!(actual, expected, "C22: seed={seed}");
    }

    // C23-C29: every record count/shift shape that C treats differently.
    unsafe {
        assert_eq!((pair.c.manipulate_records)(ptr::null_mut(), 0, 0), 0);
        assert_eq!((pair.rust.manipulate_records)(ptr::null_mut(), 0, 0), 0);
    }
    for _ in 0..RANDOM_CASES {
        compare_records(&pair, random_records(&mut rng, 1), 1, 0, "C24");
        compare_records(&pair, random_records(&mut rng, 12), 12, 0, "C25");
        compare_records(&pair, random_records(&mut rng, 12), 12, 1, "C26");
        compare_records(&pair, random_records(&mut rng, 12), 12, 5, "C27");
        compare_records(&pair, random_records(&mut rng, 12), 12, 11, "C28");
        compare_records(&pair, random_records(&mut rng, 12), 12, 12, "C29");
    }

    // C30-C31 reload isolated copies for each randomized fresh-state case.
    for fresh_case in 0..RANDOM_CASES {
        let hatch_pair = Pair::load(&format!("valid-hatch-{fresh_case}"));
        for repeated_case in 0..8 {
            let parameters = (
                rng.int(-20, 20),
                rng.int(-3, 3),
                rng.int(-20, 20),
                rng.int(-20, 20),
            );
            let expected = unsafe {
                (hatch_pair.c.hatch)(parameters.0, parameters.1, parameters.2, parameters.3)
            };
            let actual = unsafe {
                (hatch_pair.rust.hatch)(parameters.0, parameters.1, parameters.2, parameters.3)
            };
            assert_eq!(
                actual,
                expected,
                "{}: parameters={parameters:?}",
                if repeated_case == 0 { "C30" } else { "C31" }
            );
        }
    }
}

#[test]
fn handled_error_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let pair = Pair::load("errors");
    let mut rng = Rng::new(0xd1b5_4a32_d192_ed03);

    // E01: nonpositive shifts do not access or mutate the array.
    for shift in [c_int::MIN, -100, -1, 0] {
        let original: Vec<c_int> = (0..16).map(|_| rng.int(-10_000, 10_000)).collect();
        let mut c_values = original.clone();
        let mut rust_values = original.clone();
        unsafe {
            (pair.c.shift_array_data)(c_values.as_mut_ptr(), 16, shift);
            (pair.rust.shift_array_data)(rust_values.as_mut_ptr(), 16, shift);
        }
        assert_eq!(c_values, original, "E01 C changed input");
        assert_eq!(rust_values, c_values, "E01 shift={shift}");
    }
    unsafe {
        (pair.c.shift_array_data)(ptr::null_mut(), 16, 0);
        (pair.rust.shift_array_data)(ptr::null_mut(), 16, 0);
    }

    // E02: shifts at and above size, including oversized values, are no-ops.
    for shift in [16, 17, 1_000, c_int::MAX] {
        let original: Vec<c_int> = (0..16).map(|_| rng.int(-10_000, 10_000)).collect();
        let mut c_values = original.clone();
        let mut rust_values = original.clone();
        unsafe {
            (pair.c.shift_array_data)(c_values.as_mut_ptr(), 16, shift);
            (pair.rust.shift_array_data)(rust_values.as_mut_ptr(), 16, shift);
        }
        assert_eq!(c_values, original, "E02 C changed input");
        assert_eq!(rust_values, c_values, "E02 shift={shift}");
    }
    unsafe {
        (pair.c.shift_array_data)(ptr::null_mut(), 16, c_int::MAX);
        (pair.rust.shift_array_data)(ptr::null_mut(), 16, c_int::MAX);
    }

    // E03-E04: no record access when num_records - shift is nonpositive.
    for (num_records, shift) in [
        (16, 16),
        (16, 17),
        (16, c_int::MAX),
        (0, 0),
        (0, 1),
        (-1, -1),
    ] {
        let expected = unsafe { (pair.c.manipulate_records)(ptr::null_mut(), num_records, shift) };
        let actual = unsafe { (pair.rust.manipulate_records)(ptr::null_mut(), num_records, shift) };
        assert_eq!(expected, 0, "E03/E04 C sentinel");
        assert_eq!(
            actual, expected,
            "E03/E04 num_records={num_records}, shift={shift}"
        );
    }

    // E05: negative counts skip both loops and return exactly zero.
    for count in [-1, -2, -17, c_int::MIN] {
        let base = rng.int(c_int::MIN, c_int::MAX);
        let expected = unsafe { (pair.c.compute_with_dynamic_memory)(base, count) };
        let actual = unsafe { (pair.rust.compute_with_dynamic_memory)(base, count) };
        assert_eq!(expected, 0, "E05 C sentinel");
        assert_eq!(actual, expected, "E05 base={base}, count={count}");
    }
}
