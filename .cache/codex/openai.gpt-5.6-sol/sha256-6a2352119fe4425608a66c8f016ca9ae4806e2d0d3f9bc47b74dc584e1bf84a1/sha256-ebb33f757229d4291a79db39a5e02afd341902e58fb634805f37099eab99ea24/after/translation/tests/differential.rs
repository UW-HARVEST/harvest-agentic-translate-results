use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_float, c_uint};
use std::path::{Path, PathBuf};
use std::process::Command;

type Colourblind = unsafe extern "C" fn(c_uint, *mut c_float, *mut c_float, *mut c_float);

const RANDOM_CASES_PER_CONFIGURATION: usize = 20_000;

#[derive(Clone, Copy, Debug)]
enum PointerLayout {
    Distinct,
    RedGreen,
    RedBlue,
    GreenBlue,
    All,
}

impl PointerLayout {
    const ALL: [Self; 5] = [
        Self::Distinct,
        Self::RedGreen,
        Self::RedBlue,
        Self::GreenBlue,
        Self::All,
    ];

    fn indices(self) -> [usize; 3] {
        match self {
            Self::Distinct => [0, 1, 2],
            Self::RedGreen => [0, 0, 2],
            Self::RedBlue => [0, 1, 0],
            Self::GreenBlue => [0, 1, 1],
            Self::All => [0, 0, 0],
        }
    }
}

struct LoadedApi {
    _library: Library,
    colourblind: Colourblind,
}

impl LoadedApi {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let colourblind = {
            let symbol: Symbol<'_, Colourblind> = unsafe { library.get(b"colourblind\0") }
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load colourblind from {}: {error}",
                        path.display()
                    )
                });
            *symbol
        };
        Self {
            _library: library,
            colourblind,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("../c_src/build")
        .join("libharvest-work-subbLC.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir()
        .join("target/release")
        .join("libcolourblind_lib.so")
}

fn assert_libraries_exist() {
    for path in [c_library_path(), rust_library_path()] {
        assert!(
            path.is_file(),
            "required shared library does not exist: {}; build both libraries first",
            path.display()
        );
    }
}

fn next_random(state: &mut u64) -> u32 {
    // SplitMix64, with a fixed test seed supplied by the caller.
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (value ^ (value >> 31)) as u32
}

unsafe fn run_case(
    function: Colourblind,
    impairment: u32,
    initial_bits: [u32; 3],
    layout: PointerLayout,
) -> [u32; 3] {
    let mut storage = initial_bits.map(f32::from_bits);
    let indices = layout.indices();
    let base = storage.as_mut_ptr();
    unsafe {
        function(
            impairment,
            base.add(indices[0]),
            base.add(indices[1]),
            base.add(indices[2]),
        );
    }
    storage.map(f32::to_bits)
}

fn compare_case(
    c_function: Colourblind,
    rust_function: Colourblind,
    impairment: u32,
    layout: PointerLayout,
    initial_bits: [u32; 3],
    case_number: usize,
) {
    let c_output = unsafe { run_case(c_function, impairment, initial_bits, layout) };
    let rust_output = unsafe { run_case(rust_function, impairment, initial_bits, layout) };
    assert_eq!(
        c_output, rust_output,
        "bit mismatch: impairment={impairment}, layout={layout:?}, \
         case={case_number}, input={initial_bits:08x?}, \
         C={c_output:08x?}, Rust={rust_output:08x?}"
    );
}

#[test]
fn all_valid_configurations_match_byte_for_byte() {
    assert_libraries_exist();
    let c_api = unsafe { LoadedApi::load(&c_library_path()) };
    let rust_api = unsafe { LoadedApi::load(&rust_library_path()) };

    let special_bits = [
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x0000_0001, // least positive subnormal
        0x007f_ffff, // greatest positive subnormal
        0x0080_0000, // least positive normal
        0x3f80_0000, // 1
        0x7f7f_ffff, // greatest finite
        0xff7f_ffff, // least finite
        0x7f80_0000, // +infinity
        0xff80_0000, // -infinity
        0x7fc0_0000, // canonical quiet NaN
        0xffc0_0001, // negative quiet NaN with payload
        0x7f80_0001, // signaling NaN with payload
    ];

    for impairment in 0..=2 {
        for layout in PointerLayout::ALL {
            let mut case_number = 0;

            for &red in &special_bits {
                for &green in &special_bits {
                    for &blue in &special_bits {
                        compare_case(
                            c_api.colourblind,
                            rust_api.colourblind,
                            impairment,
                            layout,
                            [red, green, blue],
                            case_number,
                        );
                        case_number += 1;
                    }
                }
            }

            let mut random_state =
                0x4342_4c49_4e44_0000_u64 ^ ((impairment as u64) << 8) ^ case_number as u64;
            for _ in 0..RANDOM_CASES_PER_CONFIGURATION {
                let initial_bits = [
                    next_random(&mut random_state),
                    next_random(&mut random_state),
                    next_random(&mut random_state),
                ];
                compare_case(
                    c_api.colourblind,
                    rust_api.colourblind,
                    impairment,
                    layout,
                    initial_bits,
                    case_number,
                );
                case_number += 1;
            }
        }
    }
}

#[test]
fn out_of_range_enum_values_are_exact_no_ops() {
    assert_libraries_exist();
    let c_api = unsafe { LoadedApi::load(&c_library_path()) };
    let rust_api = unsafe { LoadedApi::load(&rust_library_path()) };
    let explicit_invalid_values = [3, 4, i32::MAX as u32, 0x8000_0000, u32::MAX];
    let mut random_state = 0x494e_5641_4c49_4400_u64;

    for case_number in 0..20_000 {
        let impairment = if case_number < explicit_invalid_values.len() {
            explicit_invalid_values[case_number]
        } else {
            loop {
                let candidate = next_random(&mut random_state);
                if candidate > 2 {
                    break candidate;
                }
            }
        };
        let initial_bits = [
            next_random(&mut random_state),
            next_random(&mut random_state),
            next_random(&mut random_state),
        ];

        for layout in PointerLayout::ALL {
            let c_output = unsafe { run_case(c_api.colourblind, impairment, initial_bits, layout) };
            let rust_output =
                unsafe { run_case(rust_api.colourblind, impairment, initial_bits, layout) };
            assert_eq!(
                c_output, initial_bits,
                "C invalid enum was not a no-op: impairment={impairment:#010x}, \
                 layout={layout:?}, input={initial_bits:08x?}, output={c_output:08x?}"
            );
            assert_eq!(
                rust_output, c_output,
                "invalid-enum mismatch: impairment={impairment:#010x}, \
                 layout={layout:?}, input={initial_bits:08x?}, \
                 C={c_output:08x?}, Rust={rust_output:08x?}"
            );
        }

        // The unmatched C switch must not dereference any pointer.
        unsafe {
            (c_api.colourblind)(
                impairment,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            (rust_api.colourblind)(
                impairment,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
}

#[test]
fn null_pointer_crash_probe_child() {
    let Ok(library_kind) = env::var("COLOURBLIND_CRASH_LIBRARY") else {
        return;
    };
    let null_index: usize = env::var("COLOURBLIND_NULL_INDEX")
        .expect("COLOURBLIND_NULL_INDEX is required")
        .parse()
        .expect("COLOURBLIND_NULL_INDEX must be an integer");
    let impairment: u32 = env::var("COLOURBLIND_IMPAIRMENT")
        .expect("COLOURBLIND_IMPAIRMENT is required")
        .parse()
        .expect("COLOURBLIND_IMPAIRMENT must be an integer");
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown library kind: {other}"),
    };
    let api = unsafe { LoadedApi::load(&path) };
    let mut values = [1.25_f32, -2.5_f32, 3.75_f32];
    let mut pointers = [
        &mut values[0] as *mut f32,
        &mut values[1] as *mut f32,
        &mut values[2] as *mut f32,
    ];
    pointers[null_index] = std::ptr::null_mut();

    unsafe {
        (api.colourblind)(impairment, pointers[0], pointers[1], pointers[2]);
    }
    panic!("call unexpectedly returned after dereferencing a null pointer");
}

#[test]
fn valid_modes_have_matching_null_pointer_rejection() {
    assert_libraries_exist();
    let current_executable = env::current_exe().expect("failed to locate test executable");

    for impairment in 0..=2_u32 {
        for null_index in 0..3_usize {
            let run_child = |library_kind: &str| {
                Command::new(&current_executable)
                    .args(["--exact", "null_pointer_crash_probe_child", "--nocapture"])
                    .env("COLOURBLIND_CRASH_LIBRARY", library_kind)
                    .env("COLOURBLIND_NULL_INDEX", null_index.to_string())
                    .env("COLOURBLIND_IMPAIRMENT", impairment.to_string())
                    .status()
                    .unwrap_or_else(|error| {
                        panic!("failed to run {library_kind} crash child: {error}")
                    })
            };

            let c_status = run_child("c");
            let rust_status = run_child("rust");
            assert!(
                !c_status.success() && !rust_status.success(),
                "null pointer unexpectedly succeeded: impairment={impairment}, \
                 null_index={null_index}, C={c_status}, Rust={rust_status}"
            );

            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(
                    c_status.signal(),
                    rust_status.signal(),
                    "different termination signals: impairment={impairment}, \
                     null_index={null_index}, C={c_status}, Rust={rust_status}"
                );
            }
        }
    }
}
