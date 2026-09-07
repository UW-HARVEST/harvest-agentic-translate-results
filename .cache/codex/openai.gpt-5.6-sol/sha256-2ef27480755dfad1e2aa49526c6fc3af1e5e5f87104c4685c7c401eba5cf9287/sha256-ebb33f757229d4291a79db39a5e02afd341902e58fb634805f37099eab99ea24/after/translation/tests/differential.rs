use libloading::Library;
use std::ffi::{c_double, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ResultValue {
    value: c_int,
    scaled: c_double,
    rank: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ResultArray {
    data: [ResultValue; 10],
    count: c_int,
}

type Operation = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type ScalarOp = Operation;
type SafeDouble = unsafe extern "C" fn(c_double) -> c_int;
type Scaled = unsafe extern "C" fn(c_int, c_double) -> c_int;
type Compare = unsafe extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int;
type Init = unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int);
type Process = unsafe extern "C" fn(*mut ResultArray, Operation) -> c_int;
type Weighted = unsafe extern "C" fn(*mut ResultArray) -> c_int;
type Arrayfunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _lib: Library,
    add: ScalarOp,
    multiply: ScalarOp,
    subtract: ScalarOp,
    modulo: ScalarOp,
    safe_double: SafeDouble,
    scaled: Scaled,
    compare: Compare,
    init: Init,
    process: Process,
    weighted: Weighted,
    arrayfunc: Arrayfunc,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let lib = unsafe { Library::new(path) }.unwrap_or_else(|e| {
            panic!("failed to load {}: {e}", path.display());
        });
        unsafe {
            Self {
                add: *lib.get(b"add_operation\0").unwrap(),
                multiply: *lib.get(b"multiply_operation\0").unwrap(),
                subtract: *lib.get(b"subtract_operation\0").unwrap(),
                modulo: *lib.get(b"modulo_operation\0").unwrap(),
                safe_double: *lib.get(b"safe_double_to_int\0").unwrap(),
                scaled: *lib.get(b"compute_scaled_value\0").unwrap(),
                compare: *lib.get(b"compare_results_in_array\0").unwrap(),
                init: *lib.get(b"init_result_array\0").unwrap(),
                process: *lib.get(b"process_with_foreach\0").unwrap(),
                weighted: *lib.get(b"compute_weighted_sum\0").unwrap(),
                arrayfunc: *lib.get(b"arrayfunc\0").unwrap(),
                _lib: lib,
            }
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_library() -> PathBuf {
    root().join("c_src/build/libharvest-work-jrm6bV.so")
}

fn rust_library() -> PathBuf {
    root().join("translation/target/release/libarrayfunc_lib.so")
}

fn apis() -> (Api, Api) {
    assert!(c_library().is_file(), "C shared library has not been built");
    assert!(
        rust_library().is_file(),
        "release Rust shared library has not been built"
    );
    unsafe { (Api::load(&c_library()), Api::load(&rust_library())) }
}

fn blank_array() -> ResultArray {
    unsafe { std::mem::zeroed() }
}

fn array_bytes(arr: &ResultArray) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            std::ptr::from_ref(arr).cast::<u8>(),
            std::mem::size_of::<ResultArray>(),
        )
    }
}

fn assert_array_bytes(c: &ResultArray, rust: &ResultArray, context: &str) {
    assert_eq!(
        array_bytes(c),
        array_bytes(rust),
        "ResultArray bytes differ: {context}"
    );
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }
}

fn randomized_values(rng: &mut Rng) -> [i32; 10] {
    std::array::from_fn(|_| rng.i32())
}

fn init_pair(
    c: &Api,
    rust: &Api,
    values: &mut [i32; 10],
    count: i32,
) -> (ResultArray, ResultArray) {
    let mut c_arr = blank_array();
    let mut rust_arr = blank_array();
    unsafe {
        (c.init)(&mut c_arr, values.as_mut_ptr(), count);
        (rust.init)(&mut rust_arr, values.as_mut_ptr(), count);
    }
    assert_array_bytes(&c_arr, &rust_arr, &format!("init count={count}"));
    (c_arr, rust_arr)
}

#[test]
fn symbols_are_loadable_from_both_shared_libraries() {
    let names: [&[u8]; 11] = [
        b"add_operation\0",
        b"arrayfunc\0",
        b"compare_results_in_array\0",
        b"compute_scaled_value\0",
        b"compute_weighted_sum\0",
        b"init_result_array\0",
        b"modulo_operation\0",
        b"multiply_operation\0",
        b"process_with_foreach\0",
        b"safe_double_to_int\0",
        b"subtract_operation\0",
    ];

    for path in [c_library(), rust_library()] {
        let lib = unsafe { Library::new(&path) }.unwrap();
        for name in names {
            unsafe {
                lib.get::<*const c_void>(name)
                    .unwrap_or_else(|e| panic!("{}: {e}", String::from_utf8_lossy(name)));
            }
        }
    }
}

#[test]
fn valid_scalar_config_rows_1_to_6() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x5eed_a11c_e123_4567);

    for iteration in 0..2048 {
        let a = rng.i32();
        let b = rng.i32();
        let u1 = rng.i32();
        let u2 = rng.i32();
        unsafe {
            assert_eq!(
                (c.add)(a, b, u1, u2),
                (rust.add)(a, b, u1, u2),
                "CONFIG 1 iteration {iteration}"
            );
            assert_eq!(
                (c.multiply)(a, b, u1, u2),
                (rust.multiply)(a, b, u1, u2),
                "CONFIG 2 iteration {iteration}"
            );
            assert_eq!(
                (c.subtract)(a, b, u1, u2),
                (rust.subtract)(a, b, u1, u2),
                "CONFIG 3 iteration {iteration}"
            );
        }

        let mut divisor = b;
        if divisor == 0 || (a == i32::MIN && divisor == -1) {
            divisor = 1;
        }
        unsafe {
            assert_eq!(
                (c.modulo)(a, divisor, u1, u2),
                (rust.modulo)(a, divisor, u1, u2),
                "CONFIG 4 iteration {iteration}"
            );
        }

        let integer = (rng.next_u64() % 4_000_000_000) as i64 - 2_000_000_000;
        let fraction = (rng.next_u64() % 1000) as f64 / 1000.0;
        let d = integer as f64 + if integer < 0 { -fraction } else { fraction };
        unsafe {
            assert_eq!(
                (c.safe_double)(d),
                (rust.safe_double)(d),
                "CONFIG 5 iteration {iteration}, d={d:?}"
            );
        }

        let base = (rng.next_u64() % 2_000_001) as i32 - 1_000_000;
        let scale = (rng.next_u64() % 200_001) as f64 / 1000.0 - 100.0;
        unsafe {
            assert_eq!(
                (c.scaled)(base, scale),
                (rust.scaled)(base, scale),
                "CONFIG 6 iteration {iteration}, base={base}, scale={scale:?}"
            );
        }
    }
}

#[test]
fn valid_compare_config_rows_7_to_9() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xc0de_0007_0009);

    for iteration in 0..512 {
        let mut values = randomized_values(&mut rng);
        let count = 2 + rng.usize(9) as i32;
        let (mut c_arr, mut rust_arr) = init_pair(&c, &rust, &mut values, count);
        let low = rng.usize((count - 1) as usize) as i32;
        let high = low + 1 + rng.usize((count - low - 1) as usize) as i32;

        for (row, idx1, idx2) in [(7, low, high), (8, high, low), (9, low, low)] {
            unsafe {
                assert_eq!(
                    (c.compare)(&mut c_arr, idx1, idx2),
                    (rust.compare)(&mut rust_arr, idx1, idx2),
                    "CONFIG {row} iteration {iteration}"
                );
            }
        }
    }
}

#[test]
fn valid_init_config_rows_10_to_14() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x1a17_0010_0014);

    for iteration in 0..256 {
        let mut values = randomized_values(&mut rng);
        for (row, count) in [
            (10, 0),
            (11, 1),
            (12, 2 + rng.usize(8) as i32),
            (13, 10),
            (14, 11 + rng.usize(1000) as i32),
        ] {
            let (c_arr, rust_arr) = init_pair(&c, &rust, &mut values, count);
            assert_array_bytes(
                &c_arr,
                &rust_arr,
                &format!("CONFIG {row} iteration {iteration}"),
            );
        }
    }
}

fn exercise_process_rows(
    c: &Api,
    rust: &Api,
    c_op: Operation,
    rust_op: Operation,
    rows: [usize; 3],
    rng: &mut Rng,
) {
    for iteration in 0..256 {
        for (row, count) in [
            (rows[0], 0),
            (rows[1], 1),
            (rows[2], 2 + rng.usize(9) as i32),
        ] {
            let mut values = randomized_values(rng);
            let (mut c_arr, mut rust_arr) = init_pair(c, rust, &mut values, count);
            let (c_result, rust_result) = unsafe {
                (
                    (c.process)(&mut c_arr, c_op),
                    (rust.process)(&mut rust_arr, rust_op),
                )
            };
            assert_eq!(
                c_result, rust_result,
                "CONFIG {row} return iteration {iteration}"
            );
            assert_array_bytes(
                &c_arr,
                &rust_arr,
                &format!("CONFIG {row} state iteration {iteration}"),
            );
        }
    }
}

#[test]
fn valid_process_config_rows_15_to_26() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xf0ee_0015_0026);
    let operations = [
        (c.add, rust.add, [15, 16, 17]),
        (c.multiply, rust.multiply, [18, 19, 20]),
        (c.subtract, rust.subtract, [21, 22, 23]),
        (c.modulo, rust.modulo, [24, 25, 26]),
    ];
    for (c_op, rust_op, rows) in operations {
        exercise_process_rows(&c, &rust, c_op, rust_op, rows, &mut rng);
    }
}

#[test]
fn valid_weighted_config_rows_27_to_29() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x5a11_0027_0029);

    for iteration in 0..512 {
        for (row, count) in [(27, 0), (28, 1), (29, 2 + rng.usize(9) as i32)] {
            let mut values = randomized_values(&mut rng);
            let (mut c_arr, mut rust_arr) = init_pair(&c, &rust, &mut values, count);
            let c_result = unsafe { (c.weighted)(&mut c_arr) };
            let rust_result = unsafe { (rust.weighted)(&mut rust_arr) };
            assert_eq!(c_result, rust_result, "CONFIG {row} iteration {iteration}");
            assert_array_bytes(
                &c_arr,
                &rust_arr,
                &format!("CONFIG {row} unchanged state iteration {iteration}"),
            );
        }
    }
}

#[test]
fn valid_arrayfunc_config_row_30() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xa22a_0030_0030);
    let boundaries = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];

    for (iteration, &(a, b, d, e)) in [
        (0, 0, 0, 0),
        (1, -1, 2, -3),
        (i32::MIN, i32::MAX, i32::MIN, i32::MAX),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
    ]
    .iter()
    .enumerate()
    {
        unsafe {
            assert_eq!(
                (c.arrayfunc)(a, b, d, e),
                (rust.arrayfunc)(a, b, d, e),
                "CONFIG 30 directed iteration {iteration}"
            );
        }
    }

    for iteration in 0..4096 {
        let args = if iteration < boundaries.len() {
            [
                boundaries[iteration],
                boundaries[(iteration + 1) % boundaries.len()],
                boundaries[(iteration + 2) % boundaries.len()],
                boundaries[(iteration + 3) % boundaries.len()],
            ]
        } else {
            [rng.i32(), rng.i32(), rng.i32(), rng.i32()]
        };
        unsafe {
            assert_eq!(
                (c.arrayfunc)(args[0], args[1], args[2], args[3]),
                (rust.arrayfunc)(args[0], args[1], args[2], args[3]),
                "CONFIG 30 random iteration {iteration}, args={args:?}"
            );
        }
    }
}

#[test]
fn error_return_and_boundary_rows_1_to_10_and_17_to_22() {
    let (c, rust) = apis();

    for a in [i32::MIN, -12345, -1, 0, 1, 12345, i32::MAX] {
        unsafe {
            assert_eq!((c.modulo)(a, 0, 7, 9), 0, "ERROR 1 C");
            assert_eq!((c.modulo)(a, 0, 7, 9), (rust.modulo)(a, 0, 7, 9), "ERROR 1");
        }
    }

    for (row, values) in [
        (
            2,
            vec![i32::MAX as f64, i32::MAX as f64 + 1.0, f64::INFINITY],
        ),
        (
            3,
            vec![i32::MIN as f64, i32::MIN as f64 - 1.0, f64::NEG_INFINITY],
        ),
        (4, vec![f64::NAN, f64::from_bits(0x7ff8_0000_0000_0001)]),
    ] {
        for value in values {
            unsafe {
                assert_eq!(
                    (c.safe_double)(value),
                    (rust.safe_double)(value),
                    "ERROR {row}, value={value:?}"
                );
            }
        }
    }

    let mut values = [11, 22, 33, 44, 55, 66, 77, 88, 99, 111];
    let (mut c_arr, mut rust_arr) = init_pair(&c, &rust, &mut values, 10);
    for (row, idx1, idx2) in [(5, 10, 0), (6, 0, 10), (7, -1, 0), (8, 0, -1)] {
        unsafe {
            assert_eq!(
                (c.compare)(&mut c_arr, idx1, idx2),
                (rust.compare)(&mut rust_arr, idx1, idx2),
                "ERROR {row}"
            );
        }
    }

    let (negative_c, negative_rust) = init_pair(&c, &rust, &mut values, -1);
    assert_eq!(negative_c.count, -1, "ERROR 9 C count");
    assert_array_bytes(&negative_c, &negative_rust, "ERROR 9");

    let (oversized_c, oversized_rust) = init_pair(&c, &rust, &mut values, 11);
    assert_eq!(oversized_c.count, 10, "ERROR 10 C count");
    assert_array_bytes(&oversized_c, &oversized_rust, "ERROR 10");

    let mut c_negative_count = blank_array();
    let mut rust_negative_count = blank_array();
    c_negative_count.count = -1;
    rust_negative_count.count = -1;
    unsafe {
        assert_eq!((c.weighted)(&mut c_negative_count), 0, "ERROR 17 C");
        assert_eq!(
            (c.weighted)(&mut c_negative_count),
            (rust.weighted)(&mut rust_negative_count),
            "ERROR 17"
        );
    }

    for (row, base, scale) in [
        (18, i32::MAX, 1.0),
        (18, 1, f64::INFINITY),
        (19, i32::MIN, 1.0),
        (19, 1, f64::NEG_INFINITY),
        (20, 0, f64::INFINITY),
        (20, 1, f64::NAN),
    ] {
        unsafe {
            assert_eq!(
                (c.scaled)(base, scale),
                (rust.scaled)(base, scale),
                "ERROR {row}, base={base}, scale={scale:?}"
            );
        }
    }

    let mut c_null_values = blank_array();
    let mut rust_null_values = blank_array();
    unsafe {
        (c.init)(&mut c_null_values, std::ptr::null_mut(), 0);
        (rust.init)(&mut rust_null_values, std::ptr::null_mut(), 0);
    }
    assert_array_bytes(&c_null_values, &rust_null_values, "ERROR 21");

    type RawProcess = unsafe extern "C" fn(*mut ResultArray, *const c_void) -> c_int;
    let c_lib = unsafe { Library::new(c_library()) }.unwrap();
    let rust_lib = unsafe { Library::new(rust_library()) }.unwrap();
    let c_process: RawProcess =
        unsafe { *c_lib.get::<RawProcess>(b"process_with_foreach\0").unwrap() };
    let rust_process: RawProcess = unsafe {
        *rust_lib
            .get::<RawProcess>(b"process_with_foreach\0")
            .unwrap()
    };
    let mut c_empty = blank_array();
    let mut rust_empty = blank_array();
    let c_result = unsafe { c_process(&mut c_empty, std::ptr::null()) };
    let rust_result = unsafe { rust_process(&mut rust_empty, std::ptr::null()) };
    assert_eq!(c_result, rust_result, "ERROR 22 return");
    assert_array_bytes(&c_empty, &rust_empty, "ERROR 22 state");
}

fn run_null_probe(library: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("null_probe_child")
        .arg("--nocapture")
        .env("ARRAYFUNC_NULL_PROBE", case)
        .env("ARRAYFUNC_PROBE_LIBRARY", library)
        .status()
        .unwrap()
}

#[test]
fn error_null_pointer_rows_11_to_16() {
    for (row, case) in [
        (11, "compare_arr"),
        (12, "init_arr"),
        (13, "init_values"),
        (14, "process_arr"),
        (15, "process_op"),
        (16, "weighted_arr"),
    ] {
        let c_status = run_null_probe(&c_library(), case);
        let rust_status = run_null_probe(&rust_library(), case);
        assert!(!c_status.success(), "ERROR {row}: C unexpectedly succeeded");
        assert!(
            !rust_status.success(),
            "ERROR {row}: Rust unexpectedly succeeded"
        );
        #[cfg(unix)]
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "ERROR {row}: process termination signal differs"
        );
    }
}

#[test]
fn null_probe_child() {
    let Ok(case) = std::env::var("ARRAYFUNC_NULL_PROBE") else {
        return;
    };
    let path = PathBuf::from(std::env::var_os("ARRAYFUNC_PROBE_LIBRARY").unwrap());
    let lib = unsafe { Library::new(path) }.unwrap();
    let mut arr = blank_array();
    arr.count = 1;
    let mut value = 1;

    unsafe {
        match case.as_str() {
            "compare_arr" => {
                let function: libloading::Symbol<
                    unsafe extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int,
                > = lib.get(b"compare_results_in_array\0").unwrap();
                function(std::ptr::null_mut(), 0, 0);
            }
            "init_arr" => {
                let function: libloading::Symbol<
                    unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int),
                > = lib.get(b"init_result_array\0").unwrap();
                function(std::ptr::null_mut(), &mut value, 1);
            }
            "init_values" => {
                let function: libloading::Symbol<
                    unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int),
                > = lib.get(b"init_result_array\0").unwrap();
                function(&mut arr, std::ptr::null_mut(), 1);
            }
            "process_arr" => {
                let add: Operation = *lib.get(b"add_operation\0").unwrap();
                let function: libloading::Symbol<
                    unsafe extern "C" fn(*mut ResultArray, Operation) -> c_int,
                > = lib.get(b"process_with_foreach\0").unwrap();
                function(std::ptr::null_mut(), add);
            }
            "process_op" => {
                let function: libloading::Symbol<
                    unsafe extern "C" fn(*mut ResultArray, *const c_void) -> c_int,
                > = lib.get(b"process_with_foreach\0").unwrap();
                function(&mut arr, std::ptr::null());
            }
            "weighted_arr" => {
                let function: libloading::Symbol<unsafe extern "C" fn(*mut ResultArray) -> c_int> =
                    lib.get(b"compute_weighted_sum\0").unwrap();
                function(std::ptr::null_mut());
            }
            other => panic!("unknown null probe {other}"),
        }
    }
    panic!("null probe unexpectedly returned");
}
