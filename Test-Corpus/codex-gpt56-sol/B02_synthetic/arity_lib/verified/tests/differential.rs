use libloading::Library;
use std::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::ptr;

type ShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
type ProcessString = unsafe extern "C" fn(*const c_char) -> c_int;
type ApplyBitmask = unsafe extern "C" fn(c_int, c_int) -> c_int;
type InitMatrix = unsafe extern "C" fn(*mut [[c_int; 4]; 3]);
type CompareAllocations = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Arity4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type Arity2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Arity3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

// The public C header declares int even though the C definition uses unsigned
// char. Calling with c_int reproduces what a consumer compiled from the header
// does; both implementations consume the low byte on this target ABI.
type Arity = unsafe extern "C" fn(c_int, *mut c_int) -> c_int;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = PathBuf::from(env!("RUST_TEST_SO"));
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );

        Self {
            c: unsafe { Library::new(c_path).expect("load C shared library") },
            rust: unsafe { Library::new(rust_path).expect("load Rust shared library") },
        }
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = unsafe { *self.c.get::<T>(name).expect("load C symbol") };
        let rust = unsafe { *self.rust.get::<T>(name).expect("load Rust symbol") };
        (c, rust)
    }

    unsafe fn c_compare(&self) -> CompareAllocations {
        unsafe {
            *self
                .c
                .get::<CompareAllocations>(b"compare_allocations")
                .expect("load C allocator symbol")
        }
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

    fn range(&mut self, low: c_int, high: c_int) -> c_int {
        assert!(low <= high);
        let width = (i64::from(high) - i64::from(low) + 1) as u64;
        (i64::from(low) + (u64::from(self.next_u32()) % width) as i64) as c_int
    }

    fn nonzero(&mut self, magnitude: c_int) -> c_int {
        loop {
            let value = self.range(-magnitude, magnitude);
            if value != 0 {
                return value;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Param1Class {
    Z,
    P0,
    P1,
    P2,
    P3,
    N0,
    ND,
}

const PARAM1_CLASSES: [Param1Class; 7] = [
    Param1Class::Z,
    Param1Class::P0,
    Param1Class::P1,
    Param1Class::P2,
    Param1Class::P3,
    Param1Class::N0,
    Param1Class::ND,
];

fn param1(rng: &mut Rng, class: Param1Class) -> c_int {
    match class {
        Param1Class::Z => 0,
        Param1Class::P0 => 4 * rng.range(1, 50),
        Param1Class::P1 => 4 * rng.range(0, 49) + 1,
        Param1Class::P2 => 4 * rng.range(0, 49) + 2,
        Param1Class::P3 => 4 * rng.range(0, 49) + 3,
        Param1Class::N0 => -4 * rng.range(1, 50),
        Param1Class::ND => -(4 * rng.range(0, 49) + rng.range(1, 3)),
    }
}

unsafe fn compare_allocating_call<C, R>(libs: &Libraries, c_call: C, rust_call: R)
where
    C: FnOnce() -> c_int,
    R: FnOnce() -> c_int,
{
    let c_result = c_call();
    // Each call allocates and frees two ints. This neutral call restores the
    // free-list orientation so Rust sees the same pointer ordering as C.
    let _ = unsafe { (libs.c_compare())(0, 0) };
    let rust_result = rust_call();
    assert_eq!(rust_result, c_result);
}

#[test]
fn valid_shift_array_rows_1_through_6() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<ShiftArray>(b"shift_array") };

    unsafe {
        c(ptr::null_mut(), 0, 0);
        rust(ptr::null_mut(), 0, 0);
    }

    let mut rng = Rng::new(0x8d92_91ab_43cc_1021);
    for _ in 0..128 {
        let len = rng.range(1, 32) as usize;
        let original: Vec<c_int> = (0..len).map(|_| rng.range(-10_000, 10_000)).collect();

        for positions in [-rng.range(1, 8), 0, len as c_int, len as c_int + 3] {
            let mut c_values = original.clone();
            let mut rust_values = original.clone();
            unsafe {
                c(c_values.as_mut_ptr(), len as c_int, positions);
                rust(rust_values.as_mut_ptr(), len as c_int, positions);
            }
            assert_eq!(rust_values, c_values);
            assert_eq!(c_values, original);
        }

        let mut active_positions = vec![1];
        if len > 2 {
            active_positions.push(rng.range(1, len as c_int - 1));
            active_positions.push(len as c_int - 1);
        }
        for positions in active_positions {
            if positions >= len as c_int {
                continue;
            }
            let mut c_values = original.clone();
            let mut rust_values = original.clone();
            unsafe {
                c(c_values.as_mut_ptr(), len as c_int, positions);
                rust(rust_values.as_mut_ptr(), len as c_int, positions);
            }
            assert_eq!(rust_values, c_values);
        }
    }

    let mut c_values = [7, 9];
    let mut rust_values = c_values;
    unsafe {
        c(c_values.as_mut_ptr(), 2, 1);
        rust(rust_values.as_mut_ptr(), 2, 1);
    }
    assert_eq!(rust_values, c_values);

    let mut c_values: Vec<c_int> = (0..4096).collect();
    let mut rust_values = c_values.clone();
    unsafe {
        c(c_values.as_mut_ptr(), 4096, 2048);
        rust(rust_values.as_mut_ptr(), 4096, 2048);
    }
    assert_eq!(rust_values, c_values);
}

#[test]
fn valid_process_string_rows_7_through_9() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<ProcessString>(b"process_string") };
    let cases = [vec![0], vec![b'X', 0]];

    for value in cases {
        assert_eq!(unsafe { rust(value.as_ptr().cast()) }, unsafe {
            c(value.as_ptr().cast())
        });
    }

    let mut rng = Rng::new(0x490a_36d8_ee10_7ca1);
    for _ in 0..256 {
        let len = rng.range(2, 128) as usize;
        let mut value: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        value.push(0);
        assert_eq!(unsafe { rust(value.as_ptr().cast::<c_char>()) }, unsafe {
            c(value.as_ptr().cast::<c_char>())
        });
    }

    let mut large = vec![b'Q'; 16 * 1024];
    large.push(0);
    assert_eq!(unsafe { rust(large.as_ptr().cast()) }, unsafe {
        c(large.as_ptr().cast())
    });
}

#[test]
fn valid_apply_bitmask_rows_10_through_15() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<ApplyBitmask>(b"apply_bitmask") };
    let mut rng = Rng::new(0xd19c_61e8_2343_9107);

    for operation in [-17, -1, 0, 1, 2, 3, 4, 29] {
        for _ in 0..256 {
            let value = rng.next_u32() as c_int;
            assert_eq!(unsafe { rust(value, operation) }, unsafe {
                c(value, operation)
            });
        }
    }
}

#[test]
fn valid_init_matrix_row_16() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<InitMatrix>(b"init_matrix") };
    let mut c_matrix = [[c_int::MIN; 4]; 3];
    let mut rust_matrix = [[c_int::MAX; 4]; 3];

    unsafe {
        c(&mut c_matrix);
        rust(&mut rust_matrix);
    }
    assert_eq!(rust_matrix, c_matrix);
}

#[test]
fn valid_compare_allocations_rows_17_and_18() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<CompareAllocations>(b"compare_allocations") };
    let mut rng = Rng::new(0x57be_03d1_b22f_77c9);

    for positive in [false, true] {
        for _ in 0..256 {
            let val1 = if positive {
                rng.range(1, 10_000)
            } else {
                rng.range(-10_000, 0)
            };
            let val2 = rng.range(-10_000, 10_000);
            unsafe {
                compare_allocating_call(&libs, || c(val1, val2), || rust(val1, val2));
            }
        }
    }
}

#[test]
fn valid_arity4_rows_19_through_46() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<Arity4>(b"arity4") };
    let mut rng = Rng::new(0x233b_5a81_7780_f013);

    for class in PARAM1_CLASSES {
        for param3_nonzero in [false, true] {
            for param4_nonzero in [false, true] {
                for _ in 0..96 {
                    let p1 = param1(&mut rng, class);
                    let p2 = rng.range(-200, 200);
                    let p3 = if param3_nonzero { rng.nonzero(20) } else { 0 };
                    let p4 = if param4_nonzero { rng.nonzero(200) } else { 0 };
                    unsafe {
                        compare_allocating_call(
                            &libs,
                            || c(p1, p2, p3, p4),
                            || rust(p1, p2, p3, p4),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn valid_arity2_rows_47_through_53() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<Arity2>(b"arity2") };
    let mut rng = Rng::new(0x702a_219f_4620_cc91);

    for class in PARAM1_CLASSES {
        for _ in 0..128 {
            let p1 = param1(&mut rng, class);
            let p2 = rng.range(-200, 200);
            unsafe {
                compare_allocating_call(&libs, || c(p1, p2), || rust(p1, p2));
            }
        }
    }
}

#[test]
fn valid_arity3_rows_54_through_67() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<Arity3>(b"arity3") };
    let mut rng = Rng::new(0x6cf1_9214_5e80_27a3);

    for class in PARAM1_CLASSES {
        for param3_nonzero in [false, true] {
            for _ in 0..128 {
                let p1 = param1(&mut rng, class);
                let p2 = rng.range(-200, 200);
                let p3 = if param3_nonzero { rng.nonzero(20) } else { 0 };
                unsafe {
                    compare_allocating_call(&libs, || c(p1, p2, p3), || rust(p1, p2, p3));
                }
            }
        }
    }
}

#[test]
fn valid_arity_rows_68_through_72() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<Arity>(b"arity") };
    let mut rng = Rng::new(0xa72a_9428_3109_c6d5);
    let lengths = [2, 3, 4, 5, 17, 255, 258, 515, -252, c_int::MAX];

    for len in lengths {
        for _ in 0..128 {
            let mut params: Vec<c_int> = (0..8).map(|_| rng.range(-100, 100)).collect();
            let params_ptr = params.as_mut_ptr();
            unsafe {
                compare_allocating_call(&libs, || c(len, params_ptr), || rust(len, params_ptr));
            }
        }
    }
}

#[test]
fn error_shift_array_rows_1_and_2() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<ShiftArray>(b"shift_array") };
    let original = [3, -8, 13, 21];

    for positions in [-10, -1, 0, 4, 5, c_int::MAX] {
        let mut c_values = original;
        let mut rust_values = original;
        unsafe {
            c(c_values.as_mut_ptr(), 4, positions);
            rust(rust_values.as_mut_ptr(), 4, positions);
        }
        assert_eq!(c_values, original);
        assert_eq!(rust_values, c_values);
    }

    unsafe {
        c(ptr::null_mut(), 0, 0);
        rust(ptr::null_mut(), 0, 0);
    }
}

#[test]
fn error_apply_bitmask_rows_3_and_4() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<ApplyBitmask>(b"apply_bitmask") };
    let mut rng = Rng::new(0x17c8_d0a1_00e4_b35f);

    for operation in [-1, c_int::MIN, 4, c_int::MAX] {
        for _ in 0..256 {
            let value = rng.next_u32() as c_int;
            let c_result = unsafe { c(value, operation) };
            let rust_result = unsafe { rust(value, operation) };
            assert_eq!(c_result, value);
            assert_eq!(rust_result, c_result);
        }
    }
}

#[test]
fn error_compare_allocations_row_5() {
    const CHILD_MARKER: &str = "ARITY_MALLOC_FAILURE_CHILD";

    if std::env::var_os(CHILD_MARKER).is_some() {
        let libs = Libraries::load();
        let (c, rust) = unsafe { libs.pair::<CompareAllocations>(b"compare_allocations") };
        let process = libloading::os::unix::Library::this();
        let arm = unsafe {
            *process
                .get::<unsafe extern "C" fn(c_int)>(b"fail_nth_int_malloc")
                .expect("load allocation-failure control symbol")
        };

        for allocation in [1, 2] {
            unsafe { arm(allocation) };
            let c_result = unsafe { c(7, -11) };
            unsafe { arm(allocation) };
            let rust_result = unsafe { rust(7, -11) };
            assert_eq!(c_result, -1);
            assert_eq!(rust_result, c_result);
        }
        return;
    }

    let status = Command::new(std::env::current_exe().expect("find test executable"))
        .args(["--exact", "error_compare_allocations_row_5", "--nocapture"])
        .env(CHILD_MARKER, "1")
        .env("LD_PRELOAD", env!("FAIL_MALLOC_SO"))
        .status()
        .expect("run allocation-failure child test");
    assert!(status.success(), "allocation-failure child test failed");
}

#[test]
fn error_arity_rows_6_through_8() {
    let libs = Libraries::load();
    let (c, rust) = unsafe { libs.pair::<Arity>(b"arity") };

    for len in [0, 1, 256, 257] {
        for params in [ptr::null_mut(), ptr::dangling_mut::<c_int>()] {
            let c_result = unsafe { c(len, params) };
            let rust_result = unsafe { rust(len, params) };
            assert_eq!(c_result, -1);
            assert_eq!(rust_result, c_result);
        }
    }
}

#[test]
fn null_pointer_termination_parity() {
    const CHILD_MARKER: &str = "ARITY_NULL_POINTER_CHILD";
    const SCENARIO: &str = "ARITY_NULL_POINTER_SCENARIO";
    const LIBRARY: &str = "ARITY_NULL_POINTER_LIBRARY";

    if std::env::var_os(CHILD_MARKER).is_some() {
        let libs = Libraries::load();
        let use_c = std::env::var(LIBRARY).unwrap() == "c";
        match std::env::var(SCENARIO).unwrap().as_str() {
            "shift_array" => {
                let (c, rust) = unsafe { libs.pair::<ShiftArray>(b"shift_array") };
                unsafe { (if use_c { c } else { rust })(ptr::null_mut(), 2, 1) };
            }
            "process_string" => {
                let (c, rust) = unsafe { libs.pair::<ProcessString>(b"process_string") };
                unsafe { (if use_c { c } else { rust })(ptr::null()) };
            }
            "init_matrix" => {
                let (c, rust) = unsafe { libs.pair::<InitMatrix>(b"init_matrix") };
                unsafe { (if use_c { c } else { rust })(ptr::null_mut()) };
            }
            "arity" => {
                let (c, rust) = unsafe { libs.pair::<Arity>(b"arity") };
                unsafe { (if use_c { c } else { rust })(2, ptr::null_mut()) };
            }
            scenario => panic!("unknown null-pointer scenario: {scenario}"),
        }
        panic!("unchecked null-pointer call unexpectedly returned");
    }

    use std::os::unix::process::ExitStatusExt;

    for scenario in ["shift_array", "process_string", "init_matrix", "arity"] {
        let run = |library: &str| {
            Command::new(std::env::current_exe().expect("find test executable"))
                .args(["--exact", "null_pointer_termination_parity"])
                .env(CHILD_MARKER, "1")
                .env(SCENARIO, scenario)
                .env(LIBRARY, library)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("run null-pointer child test")
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "termination mismatch for {scenario}: C={c_status:?}, Rust={rust_status:?}"
        );
        assert!(
            c_status.signal().is_some(),
            "unchecked C null-pointer call unexpectedly returned for {scenario}"
        );
    }
}
