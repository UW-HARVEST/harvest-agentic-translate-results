use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const OUTPUT_FLOATS: usize = 2048;
const RANDOM_CASES: usize = 64;

#[repr(C)]
#[derive(Clone)]
struct Bs {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
#[derive(Clone)]
struct L12ScaleInfo {
    scf: [f32; 3 * 64],
    total_bands: u8,
    stereo_bands: u8,
    bitalloc: [u8; 64],
    scfcod: [u8; 64],
}

type Dequantize = unsafe extern "C" fn(*mut f32, *mut Bs, *mut L12ScaleInfo, c_int) -> c_int;

struct Api {
    _library: Library,
    dequantize: Dequantize,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let dequantize = unsafe {
            *library
                .get::<Dequantize>(b"dequantize_granule\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load dequantize_granule from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            dequantize,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_dir
        .join("../c_src/build")
        .join("libharvest-work-v0LDLj.so");
    let rust_library = crate_dir
        .join("target/release")
        .join("libdequantize_granule_lib.so");
    assert!(
        c_library.is_file(),
        "build the C reference library first: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "build the Rust release cdylib first: {}",
        rust_library.display()
    );
    (c_library, rust_library)
}

fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[derive(Clone)]
struct Case {
    seed: u64,
    total_bands: u8,
    group_size: c_int,
    start_pos: c_int,
    limit: Limit,
    bitalloc: [u8; 64],
}

#[derive(Clone, Copy)]
enum Limit {
    Exact,
    Padded(c_int),
    Explicit(c_int),
}

#[derive(Clone)]
struct RunResult {
    return_value: c_int,
    final_pos: c_int,
    output: Vec<f32>,
    sci: L12ScaleInfo,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn range(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }
}

fn grouped_bits(ba: u8) -> c_int {
    let modulus = (2_u32 << (u32::from(ba) - 17)) + 1;
    (modulus + 2 - (modulus >> 3)) as c_int
}

fn consumed_bits(case: &Case) -> c_int {
    let slots = (2 * usize::from(case.total_bands)).min(case.bitalloc.len());
    let per_outer_group: c_int = case.bitalloc[..slots]
        .iter()
        .map(|&ba| match ba {
            0 => 0,
            1..=16 => c_int::from(ba) * case.group_size.max(0),
            17..=21 => grouped_bits(ba),
            _ => panic!("test generated unsupported allocation {ba}"),
        })
        .sum();
    4 * per_outer_group
}

fn output_bytes(output: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(output.as_ptr().cast::<u8>(), std::mem::size_of_val(output))
    }
}

fn make_sci(case: &Case, rng: &mut Rng) -> L12ScaleInfo {
    let mut sci = L12ScaleInfo {
        scf: [0.0; 3 * 64],
        total_bands: case.total_bands,
        stereo_bands: rng.next_u8(),
        bitalloc: case.bitalloc,
        scfcod: [0; 64],
    };
    for value in &mut sci.scf {
        *value = f32::from_bits(rng.next_u64() as u32);
    }
    for value in &mut sci.scfcod {
        *value = rng.next_u8();
    }
    if case.total_bands > 32 {
        sci.scfcod[0] = 0;
        sci.scfcod[1] = 0;
    }
    sci
}

fn run_one(api: &Api, case: &Case, bytes: &[u8], initial: &[f32], sci: L12ScaleInfo) -> RunResult {
    let mut output = initial.to_vec();
    let mut sci = sci;
    let limit = match case.limit {
        Limit::Exact => case.start_pos + consumed_bits(case),
        Limit::Padded(bits) => case.start_pos + consumed_bits(case) + bits,
        Limit::Explicit(bits) => bits,
    };
    let mut bs = Bs {
        buf: bytes.as_ptr(),
        pos: case.start_pos,
        limit,
    };
    let return_value =
        unsafe { (api.dequantize)(output.as_mut_ptr(), &mut bs, &mut sci, case.group_size) };
    RunResult {
        return_value,
        final_pos: bs.pos,
        output,
        sci,
    }
}

fn compare(case: Case) -> (RunResult, RunResult) {
    let (c_api, rust_api) = apis();
    let mut rng = Rng::new(case.seed);
    let byte_len = ((case.start_pos.max(0) + consumed_bits(&case).max(0)) as usize / 8) + 64;
    let mut bytes = vec![0_u8; byte_len.max(64)];
    for byte in &mut bytes {
        *byte = rng.next_u8();
    }
    let mut initial = vec![0.0_f32; OUTPUT_FLOATS];
    for value in &mut initial {
        *value = f32::from_bits(rng.next_u64() as u32);
    }
    let sci = make_sci(&case, &mut rng);
    let c_result = run_one(&c_api, &case, &bytes, &initial, sci.clone());
    let rust_result = run_one(&rust_api, &case, &bytes, &initial, sci);

    assert_eq!(
        rust_result.return_value, c_result.return_value,
        "return value mismatch for seed {:#x}",
        case.seed
    );
    assert_eq!(
        rust_result.final_pos, c_result.final_pos,
        "bit position mismatch for seed {:#x}",
        case.seed
    );
    assert_eq!(
        output_bytes(&rust_result.output),
        output_bytes(&c_result.output),
        "output mismatch for seed {:#x}",
        case.seed
    );
    assert_eq!(
        rust_result.sci.scf.map(f32::to_bits),
        c_result.sci.scf.map(f32::to_bits),
        "scf mutation mismatch for seed {:#x}",
        case.seed
    );
    assert_eq!(
        rust_result.sci.bitalloc, c_result.sci.bitalloc,
        "bitalloc mutation mismatch for seed {:#x}",
        case.seed
    );
    assert_eq!(
        rust_result.sci.scfcod, c_result.sci.scfcod,
        "scfcod mutation mismatch for seed {:#x}",
        case.seed
    );
    (c_result, rust_result)
}

fn allocations(value: u8) -> [u8; 64] {
    [value; 64]
}

fn mixed_allocations(seed: u64) -> [u8; 64] {
    let mut rng = Rng::new(seed);
    let mut values = [0_u8; 64];
    for (index, value) in values.iter_mut().enumerate() {
        *value = match index % 4 {
            0 => 0,
            1 => 1 + rng.range(16) as u8,
            2 => 17 + rng.range(5) as u8,
            _ => 1 + rng.range(21) as u8,
        };
    }
    values
}

fn valid_case(
    row: u64,
    iteration: usize,
    total_bands: u8,
    group_size: c_int,
    start_pos: c_int,
    bitalloc: [u8; 64],
) -> Case {
    Case {
        seed: 0x9e37_79b9_7f4a_7c15 ^ (row << 48) ^ iteration as u64,
        total_bands,
        group_size,
        start_pos,
        limit: Limit::Padded(37),
        bitalloc,
    }
}

#[test]
fn config_01_empty_bands() {
    let groups = [0, 1, 3, 4];
    for iteration in 0..RANDOM_CASES {
        compare(valid_case(
            1,
            iteration,
            0,
            groups[iteration % groups.len()],
            (iteration % 8) as c_int,
            mixed_allocations(iteration as u64),
        ));
    }
}

#[test]
fn config_02_zero_group_direct() {
    for iteration in 0..RANDOM_CASES {
        let mut values = [0_u8; 64];
        let mut rng = Rng::new(iteration as u64 + 20);
        for value in &mut values {
            *value = rng.range(17) as u8;
        }
        compare(valid_case(
            2,
            iteration,
            1 + (iteration % 32) as u8,
            0,
            (iteration % 8) as c_int,
            values,
        ));
    }
}

#[test]
fn config_03_zero_group_grouped_consumes_codes() {
    for iteration in 0..RANDOM_CASES {
        compare(valid_case(
            3,
            iteration,
            1 + (iteration % 32) as u8,
            0,
            (iteration % 8) as c_int,
            allocations(17 + (iteration % 5) as u8),
        ));
    }
}

#[test]
fn config_04_zero_allocations_preserve_destination() {
    let groups = [1, 3, 4];
    let totals = [1, 7, 32];
    for iteration in 0..RANDOM_CASES {
        compare(valid_case(
            4,
            iteration,
            totals[iteration % totals.len()],
            groups[iteration % groups.len()],
            (iteration % 8) as c_int,
            allocations(0),
        ));
    }
}

fn exercise_uniform_row(row: u64, allocations_to_test: &[u8]) {
    for iteration in 0..RANDOM_CASES {
        let ba = allocations_to_test[iteration % allocations_to_test.len()];
        compare(valid_case(
            row,
            iteration,
            1 + (iteration % 12) as u8,
            [1, 3, 4][iteration % 3],
            (iteration % 8) as c_int,
            allocations(ba),
        ));
    }
}

#[test]
fn config_05_direct_one_bit() {
    exercise_uniform_row(5, &[1]);
}

#[test]
fn config_06_direct_two_to_seven_bits() {
    exercise_uniform_row(6, &[2, 3, 4, 5, 6, 7]);
}

#[test]
fn config_07_direct_eight_bits() {
    exercise_uniform_row(7, &[8]);
}

#[test]
fn config_08_direct_nine_to_fifteen_bits() {
    exercise_uniform_row(8, &[9, 10, 11, 12, 13, 14, 15]);
}

#[test]
fn config_09_direct_sixteen_bits() {
    exercise_uniform_row(9, &[16]);
}

#[test]
fn config_10_grouped_modulus_three() {
    exercise_uniform_row(10, &[17]);
}

#[test]
fn config_11_grouped_modulus_five() {
    exercise_uniform_row(11, &[18]);
}

#[test]
fn config_12_grouped_modulus_nine() {
    exercise_uniform_row(12, &[19]);
}

#[test]
fn config_13_grouped_modulus_seventeen() {
    exercise_uniform_row(13, &[20]);
}

#[test]
fn config_14_grouped_modulus_thirty_three() {
    exercise_uniform_row(14, &[21]);
}

#[test]
fn config_15_paired_mixed_channels() {
    for iteration in 0..RANDOM_CASES {
        let mut values = [0_u8; 64];
        values[0] = [0, 1, 8, 16, 17, 21][iteration % 6];
        values[1] = [21, 19, 0, 7, 1, 17][iteration % 6];
        compare(valid_case(
            15,
            iteration,
            1,
            1,
            (iteration % 8) as c_int,
            values,
        ));
    }
}

#[test]
fn config_16_many_bands_mixed_layout() {
    for iteration in 0..RANDOM_CASES {
        compare(valid_case(
            16,
            iteration,
            2 + (iteration % 30) as u8,
            3,
            (iteration % 8) as c_int,
            mixed_allocations(iteration as u64 + 1600),
        ));
    }
}

#[test]
fn config_17_maximum_bands_larger_group() {
    for iteration in 0..RANDOM_CASES {
        compare(valid_case(
            17,
            iteration,
            32,
            4,
            (iteration % 8) as c_int,
            mixed_allocations(iteration as u64 + 1700),
        ));
    }
}

#[test]
fn config_18_exact_bit_limit() {
    for iteration in 0..RANDOM_CASES {
        let mut case = valid_case(
            18,
            iteration,
            1 + (iteration % 32) as u8,
            [1, 3, 4][iteration % 3],
            (iteration % 8) as c_int,
            mixed_allocations(iteration as u64 + 1800),
        );
        case.limit = Limit::Exact;
        compare(case);
    }
}

#[test]
fn config_19_padded_bit_limit() {
    for iteration in 0..RANDOM_CASES {
        let mut case = valid_case(
            19,
            iteration,
            1 + (iteration % 32) as u8,
            [1, 3, 4][iteration % 3],
            (iteration % 8) as c_int,
            mixed_allocations(iteration as u64 + 1900),
        );
        case.limit = Limit::Padded(1 + (iteration % 97) as c_int);
        compare(case);
    }
}

#[test]
fn error_01_direct_read_past_limit() {
    for iteration in 0..RANDOM_CASES {
        let ba = 1 + (iteration % 16) as u8;
        let group_size = [1, 3, 4][iteration % 3];
        let start_pos = (iteration % 8) as c_int;
        let mut values = [0_u8; 64];
        values[0] = ba;
        let case = Case {
            seed: 0xe001_0000 + iteration as u64,
            total_bands: 1,
            group_size,
            start_pos,
            limit: Limit::Explicit(start_pos + c_int::from(ba) - 1),
            bitalloc: values,
        };
        let expected_pos = start_pos + 4 * group_size * c_int::from(ba);
        let expected = -((1_i32 << (ba - 1)) - 1);
        let (c_result, _) = compare(case);
        assert_eq!(c_result.final_pos, expected_pos);
        for outer in 0..4 {
            for sample in 0..group_size {
                let index = (outer * group_size + sample) as usize;
                assert_eq!(
                    c_result.output[index].to_bits(),
                    (expected as f32).to_bits()
                );
            }
        }
    }
}

#[test]
fn error_02_grouped_read_past_limit() {
    for iteration in 0..RANDOM_CASES {
        let ba = 17 + (iteration % 5) as u8;
        let group_size = [1, 3, 4][iteration % 3];
        let start_pos = (iteration % 8) as c_int;
        let bits = grouped_bits(ba);
        let mut values = [0_u8; 64];
        values[0] = ba;
        let case = Case {
            seed: 0xe002_0000 + iteration as u64,
            total_bands: 1,
            group_size,
            start_pos,
            limit: Limit::Explicit(start_pos + bits - 1),
            bitalloc: values,
        };
        let modulus = (2_u32 << (u32::from(ba) - 17)) + 1;
        let expected = 0_u32.wrapping_sub(modulus / 2) as i32;
        let (c_result, _) = compare(case);
        assert_eq!(c_result.final_pos, start_pos + 4 * bits);
        for outer in 0..4 {
            for sample in 0..group_size {
                let index = (outer * group_size + sample) as usize;
                assert_eq!(
                    c_result.output[index].to_bits(),
                    (expected as f32).to_bits()
                );
            }
        }
    }
}

#[test]
fn generic_oversized_total_bands_one_past_array_pair_capacity() {
    for iteration in 0..RANDOM_CASES {
        let mut values = mixed_allocations(iteration as u64 + 3300);
        values[62] = 0;
        values[63] = 0;
        let mut case = valid_case(33, iteration, 33, 1, (iteration % 8) as c_int, values);
        // C reads the first two scfcod bytes after bitalloc for slots 64 and 65.
        // make_sci fixes those bytes at zero so the extra slots consume no bits.
        case.limit = Limit::Padded(256);
        compare(case);
    }
}

fn child_status(library: &str, pointer: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_null_pointer_child", "--nocapture"])
        .env("DIFFERENTIAL_NULL_LIBRARY", library)
        .env("DIFFERENTIAL_NULL_POINTER", pointer)
        .status()
        .expect("run null-pointer child")
}

#[test]
fn generic_null_pointers_have_matching_process_result() {
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    for pointer in ["grbuf", "bs", "sci"] {
        let c_status = child_status("c", pointer);
        let rust_status = child_status("rust", pointer);
        assert!(
            !c_status.success(),
            "C unexpectedly accepted null {pointer}"
        );
        assert!(
            !rust_status.success(),
            "Rust unexpectedly accepted null {pointer}"
        );
        #[cfg(unix)]
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "different termination signal for null {pointer}: C={c_status:?}, Rust={rust_status:?}"
        );
        #[cfg(not(unix))]
        assert_eq!(
            rust_status.code(),
            c_status.code(),
            "different exit code for null {pointer}"
        );
    }
}

#[test]
fn ffi_null_pointer_child() {
    let Ok(library) = std::env::var("DIFFERENTIAL_NULL_LIBRARY") else {
        return;
    };
    let pointer = std::env::var("DIFFERENTIAL_NULL_POINTER").expect("null pointer selection");
    let (c_path, rust_path) = library_paths();
    let api = unsafe { Api::load(if library == "c" { &c_path } else { &rust_path }) };

    let bytes = [0xa5_u8; 64];
    let mut output = [0.0_f32; 1200];
    let mut bs = Bs {
        buf: bytes.as_ptr(),
        pos: 0,
        limit: 64 * 8,
    };
    let mut sci = L12ScaleInfo {
        scf: [0.0; 3 * 64],
        total_bands: 1,
        stereo_bands: 0,
        bitalloc: allocations(1),
        scfcod: [0; 64],
    };
    let grbuf_ptr = if pointer == "grbuf" {
        std::ptr::null_mut()
    } else {
        output.as_mut_ptr()
    };
    let bs_ptr = if pointer == "bs" {
        std::ptr::null_mut()
    } else {
        &mut bs
    };
    let sci_ptr = if pointer == "sci" {
        std::ptr::null_mut()
    } else {
        &mut sci
    };
    unsafe {
        (api.dequantize)(grbuf_ptr, bs_ptr, sci_ptr, 1);
    }
    panic!("{library} unexpectedly returned after null {pointer}");
}
