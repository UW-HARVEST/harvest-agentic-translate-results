use libloading::Library;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const INPUT_LEN: usize = 16 * 1024;
const OUTPUT_LEN: usize = 4096;
const SENTINEL: u32 = 0x7fc1_2345;

#[repr(C)]
struct Bs {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[derive(Clone)]
#[repr(C)]
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
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let dequantize = unsafe {
            *library
                .get::<Dequantize>(b"dequantize_granule\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to resolve dequantize_granule in {}: {error}",
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

struct Apis {
    c: Api,
    rust: Api,
}

impl Apis {
    fn load() -> Self {
        Self {
            c: Api::load(&manifest_dir().join("c_src/build/libtranslated_rust.so")),
            rust: Api::load(&rust_library_path()),
        }
    }
}

#[derive(Debug)]
struct Outcome {
    result: c_int,
    pos: c_int,
    limit: c_int,
    output: Vec<u32>,
}

struct Case {
    input: Vec<u8>,
    pos: c_int,
    limit: c_int,
    sci: L12ScaleInfo,
    group_size: c_int,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        manifest_dir().join(target)
    };
    target.join("debug/libdequantize_granule_lib.so")
}

fn run(api: &Api, case: &Case) -> Outcome {
    let mut sci = case.sci.clone();
    let mut bs = Bs {
        buf: case.input.as_ptr(),
        pos: case.pos,
        limit: case.limit,
    };
    let mut output = vec![f32::from_bits(SENTINEL); OUTPUT_LEN];
    let result =
        unsafe { (api.dequantize)(output.as_mut_ptr(), &mut bs, &mut sci, case.group_size) };
    Outcome {
        result,
        pos: bs.pos,
        limit: bs.limit,
        output: output.into_iter().map(f32::to_bits).collect(),
    }
}

fn compare(apis: &Apis, case: Case, label: &str) -> Outcome {
    let c = run(&apis.c, &case);
    let rust = run(&apis.rust, &case);
    assert_eq!(rust.result, c.result, "{label}: return value");
    assert_eq!(rust.pos, c.pos, "{label}: final bit position");
    assert_eq!(rust.limit, c.limit, "{label}: bit limit");
    assert_eq!(rust.output, c.output, "{label}: output bytes");
    c
}

fn scale_info(total_bands: u8, rng: &mut Rng) -> L12ScaleInfo {
    let mut scf = [0.0; 3 * 64];
    for value in &mut scf {
        *value = f32::from_bits(rng.next_u32());
    }
    let mut scfcod = [0; 64];
    rng.fill(&mut scfcod);
    L12ScaleInfo {
        scf,
        total_bands,
        stereo_bands: rng.next_u32() as u8,
        bitalloc: [0; 64],
        scfcod,
    }
}

fn random_input(rng: &mut Rng) -> Vec<u8> {
    let mut input = vec![0; INPUT_LEN];
    rng.fill(&mut input);
    input
}

fn full_case(rng: &mut Rng, sci: L12ScaleInfo, group_size: c_int, pos: c_int) -> Case {
    Case {
        input: random_input(rng),
        pos,
        limit: (INPUT_LEN * 8) as c_int,
        sci,
        group_size,
    }
}

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

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn range(&mut self, start: u32, end: u32) -> u32 {
        start + self.next_u32() % (end - start)
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u32() as u8;
        }
    }
}

fn exercise_fixed_ba(ba: u8, seed: u64, group_sizes: &[c_int]) {
    let apis = Apis::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..96 {
        let total_bands = rng.range(1, 9) as u8;
        let mut sci = scale_info(total_bands, &mut rng);
        sci.bitalloc[..2 * total_bands as usize].fill(ba);
        let group_size = group_sizes[iteration % group_sizes.len()];
        let pos = rng.range(0, 8) as c_int;
        compare(
            &apis,
            full_case(&mut rng, sci, group_size, pos),
            &format!("ba={ba}, iteration={iteration}"),
        );
    }
}

#[test]
fn config_01_empty_bands_empty_groups() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x0101);
    for iteration in 0..64 {
        let sci = scale_info(0, &mut rng);
        let pos = rng.range(0, 8) as c_int;
        let outcome = compare(
            &apis,
            full_case(&mut rng, sci, 0, pos),
            &format!("iteration={iteration}"),
        );
        assert_eq!(outcome.result, 0);
        assert_eq!(outcome.pos, pos);
    }
}

#[test]
fn config_02_empty_bands_positive_groups() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x0202);
    for iteration in 0..64 {
        let sci = scale_info(0, &mut rng);
        let group_size = [1, 3, 19][iteration % 3];
        let pos = rng.range(0, 8) as c_int;
        let outcome = compare(
            &apis,
            full_case(&mut rng, sci, group_size, pos),
            &format!("iteration={iteration}"),
        );
        assert_eq!(outcome.pos, pos);
        assert!(outcome.output.iter().all(|bits| *bits == SENTINEL));
    }
}

#[test]
fn config_03_zero_groups_direct_allocations_do_not_read() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x0303);
    for iteration in 0..96 {
        let total = rng.range(1, 33) as u8;
        let mut sci = scale_info(total, &mut rng);
        for ba in &mut sci.bitalloc[..2 * total as usize] {
            *ba = rng.range(0, 17) as u8;
        }
        let pos = rng.range(0, 64) as c_int;
        let outcome = compare(
            &apis,
            full_case(&mut rng, sci, 0, pos),
            &format!("iteration={iteration}"),
        );
        assert_eq!(outcome.pos, pos);
    }
}

#[test]
fn config_04_zero_groups_grouped_allocations_still_read() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x0404);
    for iteration in 0..96 {
        let total = rng.range(1, 33) as u8;
        let mut sci = scale_info(total, &mut rng);
        for ba in &mut sci.bitalloc[..2 * total as usize] {
            *ba = rng.range(17, 22) as u8;
        }
        let pos = rng.range(0, 8) as c_int;
        let outcome = compare(
            &apis,
            full_case(&mut rng, sci, 0, pos),
            &format!("iteration={iteration}"),
        );
        assert!(outcome.pos > pos);
    }
}

#[test]
fn config_05_zero_allocations_leave_output_untouched() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x0505);
    for iteration in 0..96 {
        let total = if iteration % 2 == 0 {
            1
        } else {
            rng.range(2, 33) as u8
        };
        let sci = scale_info(total, &mut rng);
        let pos = rng.range(0, 8) as c_int;
        let outcome = compare(
            &apis,
            full_case(&mut rng, sci, [1, 3, 19][iteration % 3], pos),
            &format!("iteration={iteration}"),
        );
        assert_eq!(outcome.pos, pos);
        assert!(outcome.output.iter().all(|bits| *bits == SENTINEL));
    }
}

#[test]
fn config_06_direct_one_bit() {
    exercise_fixed_ba(1, 0x0606, &[1, 3, 12]);
}

#[test]
fn config_07_direct_two_to_seven_bits() {
    for ba in 2..=7 {
        exercise_fixed_ba(ba, 0x0700 + u64::from(ba), &[1, 3, 12]);
    }
}

#[test]
fn config_08_direct_eight_bits() {
    exercise_fixed_ba(8, 0x0808, &[1, 3, 12]);
}

#[test]
fn config_09_direct_nine_to_fifteen_bits() {
    for ba in 9..=15 {
        exercise_fixed_ba(ba, 0x0900 + u64::from(ba), &[1, 3, 12]);
    }
}

#[test]
fn config_10_direct_sixteen_bits() {
    exercise_fixed_ba(16, 0x1010, &[1, 3, 12]);
}

#[test]
fn config_11_grouped_modulus_three() {
    exercise_fixed_ba(17, 0x1111, &[1, 3, 19]);
}

#[test]
fn config_12_grouped_modulus_five() {
    exercise_fixed_ba(18, 0x1212, &[1, 3, 19]);
}

#[test]
fn config_13_grouped_modulus_nine() {
    exercise_fixed_ba(19, 0x1313, &[1, 3, 19]);
}

#[test]
fn config_14_grouped_modulus_seventeen() {
    exercise_fixed_ba(20, 0x1414, &[1, 3, 19]);
}

#[test]
fn config_15_grouped_modulus_thirty_three() {
    exercise_fixed_ba(21, 0x1515, &[1, 3, 19]);
}

#[test]
fn config_16_mixed_sparse_allocations_and_band_offsets() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x1616);
    for iteration in 0..192 {
        let total = rng.range(2, 33) as u8;
        let mut sci = scale_info(total, &mut rng);
        for ba in &mut sci.bitalloc[..2 * total as usize] {
            *ba = rng.range(0, 22) as u8;
        }
        sci.bitalloc[0] = 0;
        sci.bitalloc[1] = rng.range(1, 17) as u8;
        sci.bitalloc[2] = rng.range(17, 22) as u8;
        let pos = rng.range(0, 8) as c_int;
        compare(
            &apis,
            full_case(&mut rng, sci, [1, 3, 12][iteration % 3], pos),
            &format!("iteration={iteration}"),
        );
    }
}

#[test]
fn config_17_group_size_shapes_and_overlapping_writes() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x1717);
    for iteration in 0..128 {
        let total = rng.range(2, 9) as u8;
        let mut sci = scale_info(total, &mut rng);
        for ba in &mut sci.bitalloc[..2 * total as usize] {
            *ba = rng.range(1, 22) as u8;
        }
        let pos = rng.range(0, 8) as c_int;
        compare(
            &apis,
            full_case(&mut rng, sci, [1, 3, 12, 19][iteration % 4], pos),
            &format!("iteration={iteration}"),
        );
    }
}

#[test]
fn config_18_exact_bit_limit_is_accepted() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x1818);
    for iteration in 0..128 {
        let grouped = iteration % 2 != 0;
        let ba = if grouped {
            rng.range(17, 22) as u8
        } else {
            rng.range(1, 17) as u8
        };
        let width = if grouped {
            let modulus = (2_u32 << (ba - 17)) + 1;
            modulus + 2 - (modulus >> 3)
        } else {
            u32::from(ba)
        };
        let mut sci = scale_info(1, &mut rng);
        sci.bitalloc[0] = ba;
        let group_size = if grouped {
            rng.range(1, 20) as c_int
        } else {
            1
        };
        let pos = rng.range(0, 8) as c_int;
        let reads_per_outer_loop = if grouped {
            width
        } else {
            width * group_size as u32
        };
        let mut case = full_case(&mut rng, sci, group_size, pos);
        case.limit = pos + (4 * reads_per_outer_loop) as c_int;
        let outcome = compare(&apis, case, &format!("iteration={iteration}"));
        assert_eq!(outcome.pos, outcome.limit);
    }
}

#[test]
fn error_01_bit_limit_overrun_substitutes_zero_and_advances() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe001);
    for iteration in 0..192 {
        let grouped = iteration % 2 != 0;
        let ba = if grouped {
            rng.range(17, 22) as u8
        } else {
            rng.range(1, 17) as u8
        };
        let mut sci = scale_info(1, &mut rng);
        sci.bitalloc[0] = ba;
        let group_size = rng.range(1, 5) as c_int;
        let pos = rng.range(0, 8) as c_int;
        let mut case = full_case(&mut rng, sci, group_size, pos);
        case.limit = pos;
        let outcome = compare(&apis, case, &format!("iteration={iteration}"));
        assert_eq!(outcome.result, group_size * 4);
        assert!(outcome.pos > outcome.limit);
    }
}

#[test]
fn generic_null_bs_is_ignored_when_allocations_are_zero() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xb500);
    let mut sci = scale_info(1, &mut rng);
    sci.bitalloc.fill(0);
    for (name, api) in [("C", &apis.c), ("Rust", &apis.rust)] {
        let mut output = vec![f32::from_bits(SENTINEL); OUTPUT_LEN];
        let result =
            unsafe { (api.dequantize)(output.as_mut_ptr(), std::ptr::null_mut(), &mut sci, 3) };
        assert_eq!(result, 12, "{name}");
        assert!(
            output.iter().all(|value| value.to_bits() == SENTINEL),
            "{name}"
        );
    }
}

#[test]
fn null_pointer_child() {
    let Ok(library_kind) = std::env::var("DIFFERENTIAL_NULL_LIBRARY") else {
        return;
    };
    let pointer_kind =
        std::env::var("DIFFERENTIAL_NULL_POINTER").expect("null pointer kind is required");
    let path = match library_kind.as_str() {
        "c" => manifest_dir().join("c_src/build/libtranslated_rust.so"),
        "rust" => rust_library_path(),
        _ => panic!("unknown library kind: {library_kind}"),
    };
    let api = Api::load(&path);
    let input = vec![0xa5; 64];
    let mut bs = Bs {
        buf: if pointer_kind == "buf" {
            std::ptr::null()
        } else {
            input.as_ptr()
        },
        pos: 0,
        limit: 64 * 8,
    };
    let mut rng = Rng::new(0x0bad_cafe);
    let mut sci = scale_info(1, &mut rng);
    sci.bitalloc[0] = 1;
    let mut output = vec![0.0; OUTPUT_LEN];
    let grbuf = if pointer_kind == "grbuf" {
        std::ptr::null_mut()
    } else {
        output.as_mut_ptr()
    };
    let bs_ptr = if pointer_kind == "bs" {
        std::ptr::null_mut()
    } else {
        &mut bs
    };
    let sci_ptr = if pointer_kind == "sci" {
        std::ptr::null_mut()
    } else {
        &mut sci
    };
    unsafe {
        (api.dequantize)(grbuf, bs_ptr, sci_ptr, 1);
    }
}

#[test]
fn generic_active_null_pointers_have_the_same_process_result() {
    let executable = std::env::current_exe().expect("current test executable");
    for pointer_kind in ["grbuf", "bs", "buf", "sci"] {
        let run = |library_kind: &str| {
            Command::new(&executable)
                .args(["--exact", "null_pointer_child", "--nocapture"])
                .env("DIFFERENTIAL_NULL_LIBRARY", library_kind)
                .env("DIFFERENTIAL_NULL_POINTER", pointer_kind)
                .status()
                .unwrap_or_else(|error| {
                    panic!("failed to run {library_kind}/{pointer_kind}: {error}")
                })
        };
        let c = run("c");
        let rust = run("rust");
        assert!(!c.success(), "C unexpectedly accepted null {pointer_kind}");
        assert_eq!(
            rust.signal(),
            c.signal(),
            "null {pointer_kind}: process signal differs"
        );
    }
}
