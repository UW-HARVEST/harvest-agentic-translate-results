use libloading::{Library, Symbol};
use std::ffi::{c_double, c_int};
use std::mem::size_of;
use std::path::PathBuf;

const RANDOM_CASES: usize = 256;

type Operation = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type ScalarOperation = Operation;
type Convert = unsafe extern "C" fn(c_double) -> c_int;
type Scale = unsafe extern "C" fn(c_int, c_double) -> c_int;
type Compare = unsafe extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int;
type Init = unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int);
type Process = unsafe extern "C" fn(*mut ResultArray, Option<Operation>) -> c_int;
type Weighted = unsafe extern "C" fn(*mut ResultArray) -> c_int;
type ArrayFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" fn custom_operation(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int {
    a * 3 - b * 2 + unused1 + unused2
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CResult {
    value: c_int,
    scaled: c_double,
    rank: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ResultArray {
    data: [CResult; 10],
    count: c_int,
}

#[repr(align(8))]
struct ArrayStorage {
    bytes: [u8; size_of::<ResultArray>()],
}

impl ArrayStorage {
    fn as_mut_ptr(&mut self) -> *mut ResultArray {
        self.bytes.as_mut_ptr().cast()
    }

    fn set_count(&mut self, count: c_int) {
        unsafe {
            std::ptr::addr_of_mut!((*self.as_mut_ptr()).count).write(count);
        }
    }

    fn count(&mut self) -> c_int {
        unsafe { std::ptr::addr_of!((*self.as_mut_ptr()).count).read() }
    }
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target = option_env!("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target"));
        Self {
            c: unsafe {
                Library::new(root.join("c_src/build/libtranslated_rust.so"))
                    .expect("build the C shared library before running tests")
            },
            rust: unsafe {
                Library::new(target.join("debug/libarrayfunc_lib.so"))
                    .expect("Cargo did not build the Rust cdylib")
            },
        }
    }

    unsafe fn pair<T>(&self, symbol: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        (unsafe { self.c.get(symbol).unwrap() }, unsafe {
            self.rust.get(symbol).unwrap()
        })
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn i32_between(&mut self, low: i32, high: i32) -> i32 {
        low + (self.next_u32() % ((high - low + 1) as u32)) as i32
    }

    fn f64_between(&mut self, low: f64, high: f64) -> f64 {
        let unit = self.next_u32() as f64 / u32::MAX as f64;
        low + unit * (high - low)
    }
}

fn sentinel_array(byte: u8) -> ArrayStorage {
    ArrayStorage {
        bytes: [byte; size_of::<ResultArray>()],
    }
}

fn assert_array_bytes(c: &ArrayStorage, rust: &ArrayStorage, context: &str) {
    assert_eq!(c.bytes, rust.bytes, "{context}");
}

unsafe fn initialized_pair(
    libs: &Libraries,
    values: &mut [i32],
    count: i32,
    sentinel: u8,
) -> (ArrayStorage, ArrayStorage) {
    let (c_init, rust_init) = unsafe { libs.pair::<Init>(b"init_result_array\0") };
    let mut c_arr = sentinel_array(sentinel);
    let mut rust_arr = sentinel_array(sentinel);
    unsafe {
        c_init(c_arr.as_mut_ptr(), values.as_mut_ptr(), count);
        rust_init(rust_arr.as_mut_ptr(), values.as_mut_ptr(), count);
    }
    (c_arr, rust_arr)
}

#[test]
fn scalar_operations_match_randomized_inputs() {
    let libs = unsafe { Libraries::load() };
    let mut rng = Rng::new(0x7351_6ca9_98d2_40ef);
    let names: [(&[u8], i32); 3] = [
        (b"add_operation\0", 0),
        (b"multiply_operation\0", 1),
        (b"subtract_operation\0", 2),
    ];

    for (name, kind) in names {
        let (c_fn, rust_fn) = unsafe { libs.pair::<ScalarOperation>(name) };
        for _ in 0..RANDOM_CASES {
            let (a, b) = if kind == 1 {
                (
                    rng.i32_between(-20_000, 20_000),
                    rng.i32_between(-20_000, 20_000),
                )
            } else {
                (
                    rng.i32_between(-1_000_000_000, 1_000_000_000),
                    rng.i32_between(-1_000_000_000, 1_000_000_000),
                )
            };
            let unused1 = rng.next_u32() as i32;
            let unused2 = rng.next_u32() as i32;
            assert_eq!(unsafe { c_fn(a, b, unused1, unused2) }, unsafe {
                rust_fn(a, b, unused1, unused2)
            });
        }
    }

    let (c_mod, rust_mod) = unsafe { libs.pair::<ScalarOperation>(b"modulo_operation\0") };
    for _ in 0..RANDOM_CASES {
        let a = rng.i32_between(-1_000_000_000, 1_000_000_000);
        let mut b = rng.i32_between(-1_000_000, 1_000_000);
        if b == 0 {
            b = 1;
        }
        let unused1 = rng.next_u32() as i32;
        let unused2 = rng.next_u32() as i32;
        assert_eq!(unsafe { c_mod(a, b, unused1, unused2) }, unsafe {
            rust_mod(a, b, unused1, unused2)
        });
    }
}

#[test]
fn conversion_and_scaling_branches_match() {
    let libs = unsafe { Libraries::load() };
    let (c_convert, rust_convert) = unsafe { libs.pair::<Convert>(b"safe_double_to_int\0") };
    let (c_scale, rust_scale) = unsafe { libs.pair::<Scale>(b"compute_scaled_value\0") };
    let mut rng = Rng::new(0xb847_51c2_24af_91d3);

    let boundary_values = [
        i32::MAX as f64,
        i32::MAX as f64 + 1.0,
        f64::INFINITY,
        i32::MIN as f64,
        i32::MIN as f64 - 1.0,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    for value in boundary_values {
        assert_eq!(unsafe { c_convert(value) }, unsafe { rust_convert(value) });
    }

    for _ in 0..RANDOM_CASES {
        let value = rng.f64_between(i32::MIN as f64 + 1.0, i32::MAX as f64 - 1.0);
        assert_eq!(unsafe { c_convert(value) }, unsafe { rust_convert(value) });

        let base = rng.i32_between(-1_000_000, 1_000_000);
        let factor = rng.f64_between(-100.0, 100.0);
        assert_eq!(unsafe { c_scale(base, factor) }, unsafe {
            rust_scale(base, factor)
        });
    }

    let scale_shapes = [
        (i32::MAX, 2.0),
        (1, f64::INFINITY),
        (i32::MIN, 2.0),
        (1, f64::NEG_INFINITY),
        (1, f64::NAN),
        (0, f64::INFINITY),
    ];
    for (base, factor) in scale_shapes {
        assert_eq!(unsafe { c_scale(base, factor) }, unsafe {
            rust_scale(base, factor)
        });
    }

    for _ in 0..RANDOM_CASES {
        let upper_base = rng.i32_between(1_500_000_000, i32::MAX);
        let upper_factor = rng.f64_between(2.0, 100.0);
        assert_eq!(unsafe { c_scale(upper_base, upper_factor) }, unsafe {
            rust_scale(upper_base, upper_factor)
        });

        let lower_base = rng.i32_between(i32::MIN, -1_500_000_000);
        let lower_factor = rng.f64_between(2.0, 100.0);
        assert_eq!(unsafe { c_scale(lower_base, lower_factor) }, unsafe {
            rust_scale(lower_base, lower_factor)
        });

        let nan_base = rng.i32_between(-1_000_000_000, 1_000_000_000);
        assert_eq!(unsafe { c_scale(nan_base, f64::NAN) }, unsafe {
            rust_scale(nan_base, f64::NAN)
        });
    }
}

#[test]
fn explicit_error_surface_returns_exact_c_results() {
    let libs = unsafe { Libraries::load() };
    let (c_modulo, rust_modulo) = unsafe { libs.pair::<ScalarOperation>(b"modulo_operation\0") };
    assert_eq!(unsafe { c_modulo(123, 0, 7, 9) }, 0);
    assert_eq!(unsafe { rust_modulo(123, 0, 7, 9) }, 0);

    let (c_convert, rust_convert) = unsafe { libs.pair::<Convert>(b"safe_double_to_int\0") };
    for (input, expected) in [
        (i32::MAX as f64, i32::MAX),
        (f64::INFINITY, i32::MAX),
        (i32::MIN as f64, i32::MIN),
        (f64::NEG_INFINITY, i32::MIN),
        (f64::NAN, 0),
    ] {
        assert_eq!(unsafe { c_convert(input) }, expected);
        assert_eq!(unsafe { rust_convert(input) }, expected);
    }

    let mut values = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110];
    let (mut c_arr, mut rust_arr) = unsafe { initialized_pair(&libs, &mut values, 2, 0xa6) };
    let (c_compare, rust_compare) = unsafe { libs.pair::<Compare>(b"compare_results_in_array\0") };
    for (idx1, idx2, expected) in [(2, 0, 0), (0, 2, 0), (0, 1, -1)] {
        assert_eq!(
            unsafe { c_compare(c_arr.as_mut_ptr(), idx1, idx2) },
            expected
        );
        assert_eq!(
            unsafe { rust_compare(rust_arr.as_mut_ptr(), idx1, idx2) },
            expected
        );
    }

    let (c_init, rust_init) = unsafe { libs.pair::<Init>(b"init_result_array\0") };
    unsafe {
        c_init(c_arr.as_mut_ptr(), values.as_mut_ptr(), 11);
        rust_init(rust_arr.as_mut_ptr(), values.as_mut_ptr(), 11);
    }
    assert_eq!(c_arr.count(), 10);
    assert_eq!(rust_arr.count(), 10);
    assert_array_bytes(&c_arr, &rust_arr, "count cap");
}

#[test]
fn comparison_orders_and_bounds_match() {
    let libs = unsafe { Libraries::load() };
    let (c_compare, rust_compare) = unsafe { libs.pair::<Compare>(b"compare_results_in_array\0") };
    let mut rng = Rng::new(0x06b2_e3f7_91ca_542d);

    for _ in 0..RANDOM_CASES {
        let count = rng.i32_between(2, 10);
        let mut values = [0_i32; 10];
        for value in &mut values {
            *value = rng.i32_between(-1_000_000, 1_000_000);
        }
        let (mut c_arr, mut rust_arr) =
            unsafe { initialized_pair(&libs, &mut values, count, 0x55) };
        let low = rng.i32_between(0, count - 2);
        let high = rng.i32_between(low + 1, count - 1);
        for (idx1, idx2) in [
            (low, low),
            (low, high),
            (high, low),
            (count, 0),
            (0, count),
            (count + 1, count + 2),
        ] {
            assert_eq!(
                unsafe { c_compare(c_arr.as_mut_ptr(), idx1, idx2) },
                unsafe { rust_compare(rust_arr.as_mut_ptr(), idx1, idx2) },
                "indices ({idx1}, {idx2}), count {count}"
            );
        }
    }
}

#[test]
fn initialization_count_shapes_match_byte_for_byte() {
    let libs = unsafe { Libraries::load() };
    let (c_init, rust_init) = unsafe { libs.pair::<Init>(b"init_result_array\0") };
    let mut rng = Rng::new(0x19d3_b8af_477c_20e1);

    for count in [-7, 0, 1, 2, 5, 9, 10, 11, 64] {
        for case in 0..RANDOM_CASES {
            let mut values = [0_i32; 64];
            for value in &mut values {
                *value = rng.i32_between(-1_000_000, 1_000_000);
            }
            let sentinel = (case as u8).wrapping_mul(37);
            let mut c_arr = sentinel_array(sentinel);
            let mut rust_arr = sentinel_array(sentinel);
            unsafe {
                c_init(c_arr.as_mut_ptr(), values.as_mut_ptr(), count);
                rust_init(rust_arr.as_mut_ptr(), values.as_mut_ptr(), count);
            }
            assert_array_bytes(&c_arr, &rust_arr, &format!("count {count}, case {case}"));
        }
    }

    for count in [-1, 0] {
        let mut c_arr = sentinel_array(0xa5);
        let mut rust_arr = sentinel_array(0xa5);
        unsafe {
            c_init(c_arr.as_mut_ptr(), std::ptr::null_mut(), count);
            rust_init(rust_arr.as_mut_ptr(), std::ptr::null_mut(), count);
        }
        assert_array_bytes(&c_arr, &rust_arr, "unused null values pointer");
    }
}

#[test]
fn foreach_operation_and_count_cross_product_matches() {
    let libs = unsafe { Libraries::load() };
    let (c_process, rust_process) = unsafe { libs.pair::<Process>(b"process_with_foreach\0") };
    let operation_names: [&[u8]; 4] = [
        b"add_operation\0",
        b"multiply_operation\0",
        b"subtract_operation\0",
        b"modulo_operation\0",
    ];
    let mut rng = Rng::new(0xef27_10a5_c384_b69d);

    for count in [0, 1, 2, 5, 9, 10] {
        for operation_name in operation_names {
            let c_op = unsafe { *libs.c.get::<Operation>(operation_name).unwrap() };
            let rust_op = unsafe { *libs.rust.get::<Operation>(operation_name).unwrap() };
            for case in 0..RANDOM_CASES {
                let mut values = [0_i32; 10];
                for value in &mut values {
                    *value = rng.i32_between(-10_000, 10_000);
                }
                let (mut c_arr, mut rust_arr) =
                    unsafe { initialized_pair(&libs, &mut values, count, case as u8) };
                let c_result = unsafe { c_process(c_arr.as_mut_ptr(), Some(c_op)) };
                let rust_result = unsafe { rust_process(rust_arr.as_mut_ptr(), Some(rust_op)) };
                assert_eq!(c_result, rust_result, "count {count}, case {case}");
                assert_array_bytes(&c_arr, &rust_arr, &format!("count {count}, case {case}"));
            }
        }
    }

    for count in [1, 5, 10] {
        for case in 0..RANDOM_CASES {
            let mut values = [0_i32; 10];
            for value in &mut values {
                *value = rng.i32_between(-10_000, 10_000);
            }
            let (mut c_arr, mut rust_arr) =
                unsafe { initialized_pair(&libs, &mut values, count, case as u8) };
            let c_result = unsafe { c_process(c_arr.as_mut_ptr(), Some(custom_operation)) };
            let rust_result =
                unsafe { rust_process(rust_arr.as_mut_ptr(), Some(custom_operation)) };
            assert_eq!(c_result, rust_result, "custom callback count {count}");
            assert_array_bytes(&c_arr, &rust_arr, "custom callback output");
        }
    }

    let mut c_arr = sentinel_array(0x3c);
    let mut rust_arr = sentinel_array(0x3c);
    c_arr.set_count(0);
    rust_arr.set_count(0);
    assert_eq!(unsafe { c_process(c_arr.as_mut_ptr(), None) }, unsafe {
        rust_process(rust_arr.as_mut_ptr(), None)
    });
    assert_array_bytes(&c_arr, &rust_arr, "unused null callback");
}

#[test]
fn weighted_sum_count_shapes_match() {
    let libs = unsafe { Libraries::load() };
    let (c_weighted, rust_weighted) = unsafe { libs.pair::<Weighted>(b"compute_weighted_sum\0") };
    let mut rng = Rng::new(0x429d_81e6_a730_fc15);

    for count in [-2, 0, 1, 2, 5, 9, 10] {
        for case in 0..RANDOM_CASES {
            let mut values = [0_i32; 10];
            for value in &mut values {
                *value = rng.i32_between(-1_000_000, 1_000_000);
            }
            let (mut c_arr, mut rust_arr) =
                unsafe { initialized_pair(&libs, &mut values, count, case as u8) };
            assert_eq!(
                unsafe { c_weighted(c_arr.as_mut_ptr()) },
                unsafe { rust_weighted(rust_arr.as_mut_ptr()) },
                "count {count}, case {case}"
            );
            assert_array_bytes(&c_arr, &rust_arr, "weighted sum must not mutate input");
        }
    }
}

#[test]
fn arrayfunc_matches_randomized_end_to_end_inputs() {
    let libs = unsafe { Libraries::load() };
    let (c_arrayfunc, rust_arrayfunc) = unsafe { libs.pair::<ArrayFunc>(b"arrayfunc\0") };
    let mut rng = Rng::new(0xd51a_739c_2ef8_460b);

    let edge_cases = [
        (0, 0, 0, 0),
        (1, -1, 2, -2),
        (-10_000, 10_000, -10_000, 10_000),
        (10_000, -10_000, 10_000, -10_000),
    ];
    for (a, b, c, d) in edge_cases {
        assert_eq!(unsafe { c_arrayfunc(a, b, c, d) }, unsafe {
            rust_arrayfunc(a, b, c, d)
        });
    }

    for _ in 0..(RANDOM_CASES * 4) {
        let a = rng.i32_between(-10_000, 10_000);
        let b = rng.i32_between(-10_000, 10_000);
        let c = rng.i32_between(-10_000, 10_000);
        let d = rng.i32_between(-10_000, 10_000);
        assert_eq!(
            unsafe { c_arrayfunc(a, b, c, d) },
            unsafe { rust_arrayfunc(a, b, c, d) },
            "inputs ({a}, {b}, {c}, {d})"
        );
    }
}
