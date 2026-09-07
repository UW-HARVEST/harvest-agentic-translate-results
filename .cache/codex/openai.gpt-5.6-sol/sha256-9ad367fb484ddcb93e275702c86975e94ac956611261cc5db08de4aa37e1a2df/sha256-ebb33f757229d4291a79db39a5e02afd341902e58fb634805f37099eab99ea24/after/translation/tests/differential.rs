use libloading::Library;
use std::ffi::{c_int, c_long, c_void};
use std::fs;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicUsize, Ordering};

const CASES: usize = 128;
static COPY_ID: AtomicUsize = AtomicUsize::new(0);

type VoidIntInt = unsafe extern "C" fn(c_int, c_int);
type Int3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type Apply = unsafe extern "C" fn(*const c_void, c_int, c_int, c_int) -> c_int;
type ShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
type ProcessPointer = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type DynamicMemory = unsafe extern "C" fn(c_int, c_int) -> c_int;
type TimeValue = unsafe extern "C" fn(c_int) -> c_int;
type ManipulateRecords = unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int;
type Hatch = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: c_long,
    name: [i8; 32],
}

struct Api {
    _library: Library,
    increment_counter: VoidIntInt,
    update_accumulator: VoidIntInt,
    apply_operation: Apply,
    add_three: Int3,
    multiply_add: Int3,
    complex_calc: Int3,
    shift_array_data: ShiftArray,
    process_pointer_data: ProcessPointer,
    compute_with_dynamic_memory: DynamicMemory,
    get_time_based_value: TimeValue,
    manipulate_records: ManipulateRecords,
    hatch: Hatch,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let loaded = unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| {
                        panic!("failed to load {} from {}: {error}", $name, path.display())
                    });
                *loaded
            }};
        }

        Self {
            increment_counter: symbol!("increment_counter", VoidIntInt),
            update_accumulator: symbol!("update_accumulator", VoidIntInt),
            apply_operation: symbol!("apply_operation", Apply),
            add_three: symbol!("add_three", Int3),
            multiply_add: symbol!("multiply_add", Int3),
            complex_calc: symbol!("complex_calc", Int3),
            shift_array_data: symbol!("shift_array_data", ShiftArray),
            process_pointer_data: symbol!("process_pointer_data", ProcessPointer),
            compute_with_dynamic_memory: symbol!("compute_with_dynamic_memory", DynamicMemory),
            get_time_based_value: symbol!("get_time_based_value", TimeValue),
            manipulate_records: symbol!("manipulate_records", ManipulateRecords),
            hatch: symbol!("hatch", Hatch),
            _library: library,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    fn fresh(tag: &str) -> Self {
        let id = COPY_ID.fetch_add(1, Ordering::Relaxed);
        let copy_dir = manifest_dir().join("target").join("differential-libraries");
        fs::create_dir_all(&copy_dir).expect("create differential library directory");
        let stem = format!("{}_{}_{}", std::process::id(), id, tag);
        let c_copy = copy_dir.join(format!("{stem}_c.so"));
        let rust_copy = copy_dir.join(format!("{stem}_rust.so"));
        fs::copy(c_library_path(), &c_copy).expect("copy C shared library");
        fs::copy(rust_library_path(), &rust_copy).expect("copy Rust shared library");
        unsafe {
            Self {
                c: Api::load(&c_copy),
                rust: Api::load(&rust_copy),
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("read {}: {error}", build.display()))
        .map(|entry| entry.expect("C build entry").path())
        .filter(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some("so")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("lib") && !name.contains("_c") && !name.contains("_rust")
                    })
        })
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C .so in {}",
        build.display()
    );
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libhatch_lib.so")
}

fn function_pointer(function: Int3) -> *const c_void {
    function as *const () as *const c_void
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn bounded(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() as usize) % upper_exclusive
    }
}

fn boundary_i32(index: usize, rng: &mut Rng) -> i32 {
    const BOUNDARIES: [i32; 12] = [
        i32::MIN,
        i32::MIN + 1,
        -65_536,
        -1,
        0,
        1,
        2,
        65_535,
        65_536,
        i32::MAX - 1,
        i32::MAX,
        1_193_047,
    ];
    if index < BOUNDARIES.len() {
        BOUNDARIES[index]
    } else {
        rng.i32()
    }
}

fn record(rng: &mut Rng) -> DataRecord {
    let mut name = [0_i8; 32];
    for byte in &mut name {
        *byte = rng.i32() as i8;
    }
    DataRecord {
        id: rng.i32(),
        value: rng.i32(),
        timestamp: rng.next_u64() as c_long,
        name,
    }
}

fn records_bytes(records: &[DataRecord]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            records.as_ptr().cast::<u8>(),
            std::mem::size_of_val(records),
        )
    }
}

unsafe extern "C" fn external_callback(a: c_int, b: c_int, c: c_int) -> c_int {
    a.rotate_left(7)
        .wrapping_add(b.rotate_right(3))
        .wrapping_sub(c)
}

#[test]
fn valid_configuration_surface_matches() {
    assert!(
        rust_library_path().is_file(),
        "build the release Rust .so before running differential tests"
    );

    // C11, C19: direct low-level calls from zero global state.
    {
        let pair = Pair::fresh("zero_state");
        let mut rng = Rng::new(0x6a09_e667_f3bc_c909);
        for index in 0..CASES {
            let a = boundary_i32(index, &mut rng);
            let b = rng.i32();
            let c = rng.i32();
            unsafe {
                assert_eq!(
                    (pair.c.complex_calc)(a, b, c),
                    (pair.rust.complex_calc)(a, b, c),
                    "C11 case {index}"
                );
                let mut c_value = rng.i32();
                let mut rust_value = c_value;
                let multiplier = boundary_i32(index, &mut rng);
                assert_eq!(
                    (pair.c.process_pointer_data)(&mut c_value, multiplier),
                    (pair.rust.process_pointer_data)(&mut rust_value, multiplier),
                    "C19 case {index}"
                );
            }
        }
    }

    // C1-C4, C12, C20: state transitions and observations.
    {
        let pair = Pair::fresh("state");
        let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);
        for index in 0..CASES {
            let value = boundary_i32(index, &mut rng);
            unsafe {
                (pair.c.increment_counter)(value, rng.i32());
                (pair.rust.increment_counter)(value, rng.i32());
                assert_eq!(
                    (pair.c.complex_calc)(0, 0, 0),
                    (pair.rust.complex_calc)(0, 0, 0),
                    "C1/C2 case {index}"
                );

                let accumulator_value = boundary_i32(CASES - 1 - index, &mut rng);
                (pair.c.update_accumulator)(accumulator_value, rng.i32());
                (pair.rust.update_accumulator)(accumulator_value, rng.i32());
                let mut c_probe = rng.i32();
                let mut rust_probe = c_probe;
                let multiplier = rng.i32();
                assert_eq!(
                    (pair.c.process_pointer_data)(&mut c_probe, multiplier),
                    (pair.rust.process_pointer_data)(&mut rust_probe, multiplier),
                    "C3/C4/C20 case {index}"
                );

                let a = rng.i32();
                let b = rng.i32();
                let c = rng.i32();
                assert_eq!(
                    (pair.c.complex_calc)(a, b, c),
                    (pair.rust.complex_calc)(a, b, c),
                    "C12 case {index}"
                );
            }
        }
    }

    // C5-C10: callback dispatch and direct arithmetic exports.
    {
        let pair = Pair::fresh("operations");
        let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
        unsafe {
            (pair.c.increment_counter)(0x1357_2468, 0);
            (pair.rust.increment_counter)(0x1357_2468, 0);
        }
        for index in 0..CASES {
            let a = boundary_i32(index, &mut rng);
            let b = rng.i32();
            let c = rng.i32();
            unsafe {
                assert_eq!(
                    (pair.c.apply_operation)(function_pointer(pair.c.add_three), a, b, c),
                    (pair.rust.apply_operation)(function_pointer(pair.rust.add_three), a, b, c),
                    "C5 case {index}"
                );
                assert_eq!(
                    (pair.c.apply_operation)(function_pointer(pair.c.multiply_add), a, b, c),
                    (pair.rust.apply_operation)(function_pointer(pair.rust.multiply_add), a, b, c),
                    "C6 case {index}"
                );
                assert_eq!(
                    (pair.c.apply_operation)(function_pointer(pair.c.complex_calc), a, b, c),
                    (pair.rust.apply_operation)(function_pointer(pair.rust.complex_calc), a, b, c),
                    "C7 case {index}"
                );
                assert_eq!(
                    (pair.c.apply_operation)(function_pointer(external_callback), a, b, c),
                    (pair.rust.apply_operation)(function_pointer(external_callback), a, b, c),
                    "C8 case {index}"
                );
                assert_eq!(
                    (pair.c.add_three)(a, b, c),
                    (pair.rust.add_three)(a, b, c),
                    "C9 case {index}"
                );
                assert_eq!(
                    (pair.c.multiply_add)(a, b, c),
                    (pair.rust.multiply_add)(a, b, c),
                    "C10 case {index}"
                );
            }
        }
    }

    // C13-C18: every array-shift guard and data shape.
    {
        let pair = Pair::fresh("arrays");
        let mut rng = Rng::new(0xa54f_f53a_5f1d_36f1);
        for index in 0..CASES {
            let size = 2 + rng.bounded(31);
            let original: Vec<i32> = (0..size).map(|_| rng.i32()).collect();

            let mut c_one = original.clone();
            let mut rust_one = original.clone();
            unsafe {
                (pair.c.shift_array_data)(c_one.as_mut_ptr(), size as i32, 1);
                (pair.rust.shift_array_data)(rust_one.as_mut_ptr(), size as i32, 1);
            }
            assert_eq!(c_one, rust_one, "C13 case {index}");

            let many_size = 3 + rng.bounded(30);
            let many_shift = 2 + rng.bounded(many_size - 2);
            let many_original: Vec<i32> = (0..many_size).map(|_| rng.i32()).collect();
            let mut c_many = many_original.clone();
            let mut rust_many = many_original.clone();
            unsafe {
                (pair.c.shift_array_data)(c_many.as_mut_ptr(), many_size as i32, many_shift as i32);
                (pair.rust.shift_array_data)(
                    rust_many.as_mut_ptr(),
                    many_size as i32,
                    many_shift as i32,
                );
            }
            assert_eq!(c_many, rust_many, "C14 case {index}");

            for (row, shift) in [
                ("C15", 0),
                ("C16", -(1 + rng.bounded(32) as i32)),
                (
                    "C17",
                    size as i32
                        + if index % 2 == 0 {
                            0
                        } else {
                            1 + rng.bounded(32) as i32
                        },
                ),
            ] {
                let mut c_data = original.clone();
                let mut rust_data = original.clone();
                unsafe {
                    (pair.c.shift_array_data)(c_data.as_mut_ptr(), size as i32, shift);
                    (pair.rust.shift_array_data)(rust_data.as_mut_ptr(), size as i32, shift);
                }
                assert_eq!(c_data, original, "{row} C changed data, case {index}");
                assert_eq!(rust_data, original, "{row} Rust changed data, case {index}");
                assert_eq!(c_data, rust_data, "{row} case {index}");
            }
        }
        unsafe {
            (pair.c.shift_array_data)(std::ptr::null_mut(), 0, 0);
            (pair.rust.shift_array_data)(std::ptr::null_mut(), 0, 0);
        }
    }

    // C21-C24: negative, zero, one, and many allocation/loop counts.
    {
        let pair = Pair::fresh("dynamic");
        let mut rng = Rng::new(0x510e_527f_ade6_82d1);
        for index in 0..CASES {
            let base = boundary_i32(index, &mut rng);
            let negative_count = -(1 + rng.bounded(4096) as i32);
            unsafe {
                assert_eq!(
                    (pair.c.compute_with_dynamic_memory)(base, negative_count),
                    (pair.rust.compute_with_dynamic_memory)(base, negative_count),
                    "C21 case {index}"
                );
                assert_eq!(
                    (pair.c.compute_with_dynamic_memory)(base, 0),
                    (pair.rust.compute_with_dynamic_memory)(base, 0),
                    "C22 case {index}"
                );
                assert_eq!(
                    (pair.c.compute_with_dynamic_memory)(base, 1),
                    (pair.rust.compute_with_dynamic_memory)(base, 1),
                    "C23 case {index}"
                );
                let count = 2 + rng.bounded(63) as i32;
                assert_eq!(
                    (pair.c.compute_with_dynamic_memory)(base, count),
                    (pair.rust.compute_with_dynamic_memory)(base, count),
                    "C24 case {index}"
                );
            }
        }
    }

    // C25-C28: zero, signed non-overflow, and product-overflow seed shapes.
    {
        let pair = Pair::fresh("time");
        let mut rng = Rng::new(0x9b05_688c_2b3e_6c1f);
        unsafe {
            assert_eq!(
                (pair.c.get_time_based_value)(0),
                (pair.rust.get_time_based_value)(0),
                "C25"
            );
        }
        let safe_limit = i32::MAX / 3600;
        let overflow_seeds = [
            safe_limit,
            safe_limit + 1,
            -safe_limit,
            -safe_limit - 1,
            i32::MIN,
            i32::MAX,
        ];
        for index in 0..CASES {
            let positive = 1 + (rng.i32().unsigned_abs() % safe_limit as u32) as i32;
            let negative = -positive;
            let overflow = if index < overflow_seeds.len() {
                overflow_seeds[index]
            } else if index % 2 == 0 {
                safe_limit + 1 + rng.bounded(1_000_000) as i32
            } else {
                -safe_limit - 1 - rng.bounded(1_000_000) as i32
            };
            unsafe {
                assert_eq!(
                    (pair.c.get_time_based_value)(positive),
                    (pair.rust.get_time_based_value)(positive),
                    "C26 case {index}"
                );
                assert_eq!(
                    (pair.c.get_time_based_value)(negative),
                    (pair.rust.get_time_based_value)(negative),
                    "C27 case {index}"
                );
                assert_eq!(
                    (pair.c.get_time_based_value)(overflow),
                    (pair.rust.get_time_based_value)(overflow),
                    "C28 case {index}, seed {overflow}"
                );
            }
        }
    }

    // C29-C36: every record-shift branch and loop-bound shape.
    {
        let pair = Pair::fresh("records");
        let mut rng = Rng::new(0x1f83_d9ab_fb41_bd6b);
        for index in 0..CASES {
            let num = 2 + rng.bounded(15);

            let one_remaining_shift = num - 1;
            let original_one: Vec<_> = (0..num).map(|_| record(&mut rng)).collect();
            let mut c_one = original_one.clone();
            let mut rust_one = original_one.clone();
            let c_result = unsafe {
                (pair.c.manipulate_records)(
                    c_one.as_mut_ptr(),
                    num as i32,
                    one_remaining_shift as i32,
                )
            };
            let rust_result = unsafe {
                (pair.rust.manipulate_records)(
                    rust_one.as_mut_ptr(),
                    num as i32,
                    one_remaining_shift as i32,
                )
            };
            assert_eq!(c_result, rust_result, "C29 result case {index}");
            assert_eq!(
                records_bytes(&c_one),
                records_bytes(&rust_one),
                "C29 bytes case {index}"
            );

            let many_num = 3 + rng.bounded(14);
            let many_shift = 1 + rng.bounded(many_num - 2);
            let original_many: Vec<_> = (0..many_num).map(|_| record(&mut rng)).collect();
            let mut c_many = original_many.clone();
            let mut rust_many = original_many.clone();
            let c_result = unsafe {
                (pair.c.manipulate_records)(c_many.as_mut_ptr(), many_num as i32, many_shift as i32)
            };
            let rust_result = unsafe {
                (pair.rust.manipulate_records)(
                    rust_many.as_mut_ptr(),
                    many_num as i32,
                    many_shift as i32,
                )
            };
            assert_eq!(c_result, rust_result, "C30 result case {index}");
            assert_eq!(
                records_bytes(&c_many),
                records_bytes(&rust_many),
                "C30 bytes case {index}"
            );

            let original_zero: Vec<_> = (0..num).map(|_| record(&mut rng)).collect();
            let mut c_zero = original_zero.clone();
            let mut rust_zero = original_zero.clone();
            let c_result =
                unsafe { (pair.c.manipulate_records)(c_zero.as_mut_ptr(), num as i32, 0) };
            let rust_result =
                unsafe { (pair.rust.manipulate_records)(rust_zero.as_mut_ptr(), num as i32, 0) };
            assert_eq!(c_result, rust_result, "C31 result case {index}");
            assert_eq!(
                records_bytes(&c_zero),
                records_bytes(&rust_zero),
                "C31 bytes case {index}"
            );

            let negative_shift = -(1 + rng.bounded(8) as i32);
            let extended_len = (num as i32 - negative_shift) as usize;
            let original_negative: Vec<_> = (0..extended_len).map(|_| record(&mut rng)).collect();
            let mut c_negative = original_negative.clone();
            let mut rust_negative = original_negative.clone();
            let c_result = unsafe {
                (pair.c.manipulate_records)(c_negative.as_mut_ptr(), num as i32, negative_shift)
            };
            let rust_result = unsafe {
                (pair.rust.manipulate_records)(
                    rust_negative.as_mut_ptr(),
                    num as i32,
                    negative_shift,
                )
            };
            assert_eq!(c_result, rust_result, "C32 result case {index}");
            assert_eq!(
                records_bytes(&c_negative),
                records_bytes(&rust_negative),
                "C32 bytes case {index}"
            );

            for (row, shift) in [
                ("C33", num as i32),
                ("C34", num as i32 + 1 + rng.bounded(16) as i32),
            ] {
                let original: Vec<_> = (0..num).map(|_| record(&mut rng)).collect();
                let mut c_records = original.clone();
                let mut rust_records = original.clone();
                let c_result = unsafe {
                    (pair.c.manipulate_records)(c_records.as_mut_ptr(), num as i32, shift)
                };
                let rust_result = unsafe {
                    (pair.rust.manipulate_records)(rust_records.as_mut_ptr(), num as i32, shift)
                };
                assert_eq!(c_result, 0, "{row} C result case {index}");
                assert_eq!(rust_result, 0, "{row} Rust result case {index}");
                assert_eq!(
                    records_bytes(&c_records),
                    records_bytes(&rust_records),
                    "{row} bytes case {index}"
                );
            }
        }
        unsafe {
            assert_eq!(
                (pair.c.manipulate_records)(std::ptr::null_mut(), 0, 0),
                (pair.rust.manipulate_records)(std::ptr::null_mut(), 0, 0),
                "C35"
            );
            for num in [-1, -2, i32::MIN] {
                assert_eq!(
                    (pair.c.manipulate_records)(std::ptr::null_mut(), num, 0),
                    (pair.rust.manipulate_records)(std::ptr::null_mut(), num, 0),
                    "C36 num {num}"
                );
            }
        }
    }

    // C37: many independent first calls, each from fresh zero state.
    {
        let mut rng = Rng::new(0x5be0_cd19_137e_2179);
        for index in 0..32 {
            let pair = Pair::fresh(&format!("hatch_first_{index}"));
            let params = (rng.i32(), rng.i32(), rng.i32(), rng.i32());
            unsafe {
                assert_eq!(
                    (pair.c.hatch)(params.0, params.1, params.2, params.3),
                    (pair.rust.hatch)(params.0, params.1, params.2, params.3),
                    "C37 case {index}"
                );
            }
        }
    }

    // C38: persistent state over many composed calls.
    {
        let pair = Pair::fresh("hatch_repeated");
        let mut rng = Rng::new(0xcbbb_9d5d_c105_9ed8);
        for index in 0..CASES {
            let params = (
                boundary_i32(index, &mut rng),
                rng.i32(),
                rng.i32(),
                rng.i32(),
            );
            unsafe {
                assert_eq!(
                    (pair.c.hatch)(params.0, params.1, params.2, params.3),
                    (pair.rust.hatch)(params.0, params.1, params.2, params.3),
                    "C38 case {index}"
                );
            }
        }
    }
}

#[test]
fn explicit_error_surface_matches() {
    let pair = Pair::fresh("errors");
    let mut rng = Rng::new(0x243f_6a88_85a3_08d3);

    for index in 0..CASES {
        let size = 1 + rng.bounded(32);
        let original: Vec<i32> = (0..size).map(|_| rng.i32()).collect();

        // E1
        let shift = if index % 2 == 0 {
            0
        } else {
            -(1 + rng.bounded(64) as i32)
        };
        let mut c_data = original.clone();
        let mut rust_data = original.clone();
        unsafe {
            (pair.c.shift_array_data)(c_data.as_mut_ptr(), size as i32, shift);
            (pair.rust.shift_array_data)(rust_data.as_mut_ptr(), size as i32, shift);
        }
        assert_eq!(c_data, original, "E1 C case {index}");
        assert_eq!(rust_data, original, "E1 Rust case {index}");

        // E2
        let shift = size as i32 + rng.bounded(64) as i32;
        let mut c_data = original.clone();
        let mut rust_data = original.clone();
        unsafe {
            (pair.c.shift_array_data)(c_data.as_mut_ptr(), size as i32, shift);
            (pair.rust.shift_array_data)(rust_data.as_mut_ptr(), size as i32, shift);
        }
        assert_eq!(c_data, original, "E2 C case {index}");
        assert_eq!(rust_data, original, "E2 Rust case {index}");

        // E3
        let num = 1 + rng.bounded(16);
        let shift = if index % 2 == 0 {
            0
        } else {
            -(1 + rng.bounded(8) as i32)
        };
        let length = (num as i32 - shift) as usize;
        let source: Vec<_> = (0..length).map(|_| record(&mut rng)).collect();
        let mut c_records = source.clone();
        let mut rust_records = source.clone();
        let c_result =
            unsafe { (pair.c.manipulate_records)(c_records.as_mut_ptr(), num as i32, shift) };
        let rust_result =
            unsafe { (pair.rust.manipulate_records)(rust_records.as_mut_ptr(), num as i32, shift) };
        assert_eq!(c_result, rust_result, "E3 result case {index}");
        assert_eq!(
            records_bytes(&c_records),
            records_bytes(&rust_records),
            "E3 bytes case {index}"
        );

        // E4
        let shift = num as i32 + rng.bounded(16) as i32;
        let source: Vec<_> = (0..num).map(|_| record(&mut rng)).collect();
        let mut c_records = source.clone();
        let mut rust_records = source.clone();
        let c_result =
            unsafe { (pair.c.manipulate_records)(c_records.as_mut_ptr(), num as i32, shift) };
        let rust_result =
            unsafe { (pair.rust.manipulate_records)(rust_records.as_mut_ptr(), num as i32, shift) };
        assert_eq!(c_result, 0, "E4 C result case {index}");
        assert_eq!(rust_result, 0, "E4 Rust result case {index}");
        assert_eq!(
            records_bytes(&c_records),
            records_bytes(&rust_records),
            "E4 bytes case {index}"
        );
    }
}

fn child_status(library: &str, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_boundary_child")
        .arg("--nocapture")
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_CASE", case)
        .status()
        .unwrap_or_else(|error| panic!("spawn {library}/{case}: {error}"))
}

#[cfg(unix)]
fn termination_signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[test]
fn generic_ffi_boundaries_match() {
    let pair = Pair::fresh("safe_boundaries");
    unsafe {
        // Null pointers are accepted when the C branch/loop does not use them.
        (pair.c.shift_array_data)(std::ptr::null_mut(), 0, 0);
        (pair.rust.shift_array_data)(std::ptr::null_mut(), 0, 0);
        assert_eq!(
            (pair.c.manipulate_records)(std::ptr::null_mut(), 0, 0),
            (pair.rust.manipulate_records)(std::ptr::null_mut(), 0, 0)
        );

        // Zero and negative counts include the C allocation-size conversion.
        for count in [0, -1, -2, i32::MIN] {
            assert_eq!(
                (pair.c.compute_with_dynamic_memory)(i32::MAX, count),
                (pair.rust.compute_with_dynamic_memory)(i32::MAX, count),
                "count {count}"
            );
        }

        // Oversized record lengths with a non-positive loop bound return zero.
        assert_eq!(
            (pair.c.manipulate_records)(std::ptr::null_mut(), i32::MAX, i32::MAX),
            (pair.rust.manipulate_records)(std::ptr::null_mut(), i32::MAX, i32::MAX)
        );
    }

    for case in [
        "null_callback",
        "null_process_pointer",
        "null_shift_pointer",
        "null_records_pointer",
        "oversized_allocation",
    ] {
        let c_status = child_status("c", case);
        let rust_status = child_status("rust", case);
        assert!(!c_status.success(), "{case}: C unexpectedly succeeded");
        assert!(
            !rust_status.success(),
            "{case}: Rust unexpectedly succeeded"
        );
        assert_eq!(
            termination_signal(c_status),
            termination_signal(rust_status),
            "{case}: termination signal differs"
        );
    }
}

#[repr(C)]
struct RLimit {
    current: u64,
    maximum: u64,
}

unsafe extern "C" {
    fn setrlimit(resource: c_int, limit: *const RLimit) -> c_int;
}

#[test]
fn ffi_boundary_child() {
    let Ok(library_kind) = std::env::var("DIFF_CHILD_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFF_CHILD_CASE").expect("DIFF_CHILD_CASE");
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown child library {other}"),
    };
    let api = unsafe { Api::load(&path) };

    unsafe {
        match case.as_str() {
            "null_callback" => {
                (api.apply_operation)(std::ptr::null(), 1, 2, 3);
            }
            "null_process_pointer" => {
                (api.process_pointer_data)(std::ptr::null_mut(), 7);
            }
            "null_shift_pointer" => {
                (api.shift_array_data)(std::ptr::null_mut(), 2, 1);
            }
            "null_records_pointer" => {
                (api.manipulate_records)(std::ptr::null_mut(), 2, 1);
            }
            "oversized_allocation" => {
                const RLIMIT_AS: c_int = 9;
                let limit = RLimit {
                    current: 256 * 1024 * 1024,
                    maximum: 256 * 1024 * 1024,
                };
                assert_eq!(setrlimit(RLIMIT_AS, &limit), 0, "setrlimit");
                (api.compute_with_dynamic_memory)(0, i32::MAX);
            }
            other => panic!("unknown child case {other}"),
        }
    }

    panic!("{library_kind}/{case} unexpectedly returned");
}

#[test]
fn data_record_layout_matches_c_abi() {
    assert_eq!(size_of::<DataRecord>(), 48);
    assert_eq!(std::mem::align_of::<DataRecord>(), 8);
}
