use libloading::Library;
use std::ffi::c_int;
use std::fs;
use std::mem::{size_of, zeroed};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[repr(C)]
#[derive(Clone, Copy)]
struct Bs {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Gr {
    sfbtab: *const u8,
    part_23_length: u16,
    big_values: u16,
    scalefac_compress: u16,
    global_gain: u8,
    block_type: u8,
    mixed_block_flag: u8,
    n_long_sfb: u8,
    n_short_sfb: u8,
    table_select: [u8; 3],
    region_count: [u8; 3],
    subblock_gain: [u8; 3],
    preflag: u8,
    scalefac_scale: u8,
    count1_table: u8,
    scfsi: u8,
}

type ReadSideInfo = unsafe extern "C" fn(*mut Bs, *mut Gr, *const u8) -> c_int;

struct Api {
    _library: Library,
    read_side_info: ReadSideInfo,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let read_side_info = unsafe {
            *library
                .get::<ReadSideInfo>(b"read_side_info\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load read_side_info from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            read_side_info,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current test executable");
    let deps = exe.parent().expect("test executable parent");
    let candidates = [
        deps.join("libread_side_info_lib.so"),
        deps.parent()
            .expect("target profile directory")
            .join("libread_side_info_lib.so"),
        manifest_dir().join("target/release/libread_side_info_lib.so"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }
    manifest_dir().join("target/release/libread_side_info_lib.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.exists(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.exists(),
        "missing Rust shared object: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    N,
    M,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Shape {
    Normal,
    Type1,
    PureShort,
    MixedShort,
    Type3,
}

#[derive(Clone, Debug)]
struct Config {
    id: String,
    code: String,
    mode: Mode,
    sr_idx: u8,
    mono: bool,
    high_scalefac: bool,
    shape: Shape,
}

fn configurations() -> Vec<Config> {
    let shapes = [
        (Shape::Normal, "n"),
        (Shape::Type1, "w"),
        (Shape::PureShort, "p"),
        (Shape::MixedShort, "x"),
        (Shape::Type3, "t"),
    ];
    let mut configs = Vec::with_capacity(180);

    for sr_idx in 0..=5 {
        for (mono, channel) in [(true, "m"), (false, "s")] {
            for (high_scalefac, range) in [(false, "lo"), (true, "hi")] {
                for (shape, shape_code) in shapes {
                    configs.push(Config {
                        id: format!("C{:03}", configs.len() + 1),
                        code: format!("N-s{sr_idx}-{channel}-{range}-{shape_code}"),
                        mode: Mode::N,
                        sr_idx,
                        mono,
                        high_scalefac,
                        shape,
                    });
                }
            }
        }
    }

    for sr_idx in 2..=7 {
        for (mono, channel) in [(true, "m"), (false, "s")] {
            for (shape, shape_code) in shapes {
                configs.push(Config {
                    id: format!("C{:03}", configs.len() + 1),
                    code: format!("M-s{sr_idx}-{channel}-{shape_code}"),
                    mode: Mode::M,
                    sr_idx,
                    mono,
                    high_scalefac: false,
                    shape,
                });
            }
        }
    }

    configs
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

    fn below(&mut self, exclusive: u32) -> u32 {
        self.next_u32() % exclusive
    }

    fn bits(&mut self, width: u32) -> u32 {
        if width == 32 {
            self.next_u32()
        } else {
            self.next_u32() & ((1_u32 << width) - 1)
        }
    }
}

struct BitWriter {
    bytes: Vec<u8>,
    pos: usize,
}

impl BitWriter {
    fn new(start_pos: usize, rng: &mut Rng) -> Self {
        let mut bytes = vec![0_u8; 256];
        for byte in &mut bytes {
            *byte = rng.next_u32() as u8;
        }
        Self {
            bytes,
            pos: start_pos,
        }
    }

    fn put(&mut self, value: u32, width: u32) {
        assert!(width <= 32);
        assert!(width == 32 || value < (1_u32 << width));
        for bit_index in (0..width).rev() {
            let byte = self.pos / 8;
            let shift = 7 - (self.pos & 7);
            let mask = 1_u8 << shift;
            if ((value >> bit_index) & 1) != 0 {
                self.bytes[byte] |= mask;
            } else {
                self.bytes[byte] &= !mask;
            }
            self.pos += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    None,
    BigValues,
    BlockTypeZero,
    FinalBudget,
}

struct Input {
    bytes: Vec<u8>,
    header: [u8; 4],
    start_pos: c_int,
    limit: c_int,
    gr_count: usize,
}

fn header_for(config: &Config) -> [u8; 4] {
    let (version_bit, sample_index) = match (config.mode, config.sr_idx) {
        (Mode::N, 0) => (0_u8, 0_u8),
        (Mode::N, 1) => (0, 2),
        (Mode::N, index @ 2..=5) => (0x10, index - 2),
        (Mode::M, index @ 2..=4) => (0, index - 2),
        (Mode::M, index @ 5..=7) => (0x10, index - 5),
        _ => panic!("unsupported mode/sample-rate combination"),
    };
    let mode_bit = if config.mode == Mode::M { 0x08 } else { 0 };
    let channel = if config.mono { 0xc0 } else { 0 };
    [0xff, version_bit | mode_bit, sample_index << 2, channel]
}

fn make_input(config: &Config, iteration: u64, fault: Fault) -> Input {
    let numeric_id = config.id[1..].parse::<u64>().unwrap_or(0);
    let seed = 0x6a09_e667_f3bc_c909_u64 ^ (numeric_id << 32) ^ iteration ^ ((fault as u64) << 56);
    let mut rng = Rng::new(seed);
    let start_pos = (iteration as usize) & 7;
    let mut writer = BitWriter::new(start_pos, &mut rng);
    let base_gr_count = if config.mono { 1 } else { 2 };
    let gr_count = if config.mode == Mode::M {
        base_gr_count * 2
    } else {
        base_gr_count
    };

    let mut main_data_begin = if iteration % 7 == 0 {
        0
    } else {
        1 + rng.below(63)
    };
    if fault == Fault::FinalBudget {
        main_data_begin = 0;
    }

    if config.mode == Mode::M {
        writer.put(main_data_begin, 9);
        writer.put(rng.bits((7 + gr_count) as u32), (7 + gr_count) as u32);
    } else {
        let low = rng.bits(gr_count as u32);
        writer.put((main_data_begin << gr_count) | low, (8 + gr_count) as u32);
    }

    let budget = main_data_begin * 8;
    let mut remaining_budget = budget;
    for gr_index in 0..gr_count {
        let granule_shape = if gr_index == 0 {
            config.shape
        } else {
            match rng.below(5) {
                0 => Shape::Normal,
                1 => Shape::Type1,
                2 => Shape::PureShort,
                3 => Shape::MixedShort,
                4 => Shape::Type3,
                _ => unreachable!(),
            }
        };
        let part_23_length = if fault == Fault::FinalBudget && gr_index == 0 {
            1
        } else if budget == 0 {
            0
        } else {
            let remaining_granules = (gr_count - gr_index) as u32;
            let maximum = remaining_budget / remaining_granules;
            let value = if iteration % 11 == 0 {
                maximum
            } else {
                rng.below(maximum + 1)
            };
            remaining_budget -= value;
            value
        };
        writer.put(part_23_length, 12);

        let big_values = if fault == Fault::BigValues && gr_index == 0 {
            289
        } else if iteration % 13 == 0 {
            288
        } else {
            rng.below(289)
        };
        writer.put(big_values, 9);
        writer.put(rng.bits(8), 8);

        if config.mode == Mode::M {
            writer.put(rng.bits(4), 4);
        } else {
            let scalefac_compress = if config.high_scalefac {
                500 + rng.below(12)
            } else if iteration % 17 == 0 {
                499
            } else {
                rng.below(500)
            };
            writer.put(scalefac_compress, 9);
        }

        if granule_shape == Shape::Normal {
            writer.put(0, 1);
            writer.put(rng.bits(15), 15);
            writer.put(rng.bits(4), 4);
            writer.put(rng.bits(3), 3);
        } else {
            writer.put(1, 1);
            let block_type = if fault == Fault::BlockTypeZero && gr_index == 0 {
                0
            } else {
                match granule_shape {
                    Shape::Type1 => 1,
                    Shape::PureShort | Shape::MixedShort => 2,
                    Shape::Type3 => 3,
                    Shape::Normal => unreachable!(),
                }
            };
            writer.put(block_type, 2);
            if block_type != 0 {
                writer.put(u32::from(granule_shape == Shape::MixedShort), 1);
                writer.put(rng.bits(10), 10);
                writer.put(rng.bits(3), 3);
                writer.put(rng.bits(3), 3);
                writer.put(rng.bits(3), 3);
            }
        }

        if fault == Fault::BigValues || fault == Fault::BlockTypeZero {
            if gr_index == 0 {
                break;
            }
        }

        if config.mode == Mode::M {
            writer.put(rng.bits(1), 1);
        }
        writer.put(rng.bits(1), 1);
        writer.put(rng.bits(1), 1);
    }

    Input {
        bytes: writer.bytes,
        header: header_for(config),
        start_pos: start_pos as c_int,
        limit: writer.pos as c_int,
        gr_count,
    }
}

fn gr_bytes_without_pointer(gr: &Gr) -> &[u8] {
    let bytes =
        unsafe { std::slice::from_raw_parts((gr as *const Gr).cast::<u8>(), size_of::<Gr>()) };
    &bytes[size_of::<*const u8>()..]
}

fn compare_gr(case: &str, index: usize, c: &Gr, rust: &Gr, compare_table: bool) {
    assert_eq!(
        gr_bytes_without_pointer(c),
        gr_bytes_without_pointer(rust),
        "{case}: granule {index} output bytes differ"
    );
    assert_eq!(
        c.sfbtab.is_null(),
        rust.sfbtab.is_null(),
        "{case}: granule {index} sfbtab nullness differs"
    );
    if compare_table {
        assert!(!c.sfbtab.is_null(), "{case}: C sfbtab is null");
        assert!(!rust.sfbtab.is_null(), "{case}: Rust sfbtab is null");
        let table_length = if c.n_short_sfb == 0 { 23 } else { 40 };
        let c_table = unsafe { std::slice::from_raw_parts(c.sfbtab, table_length) };
        let rust_table = unsafe { std::slice::from_raw_parts(rust.sfbtab, table_length) };
        assert_eq!(
            c_table, rust_table,
            "{case}: granule {index} scalefactor table differs"
        );
    }
}

fn call_and_compare(c_api: &Api, rust_api: &Api, input: &Input, case: &str) -> c_int {
    let mut c_bytes = input.bytes.clone();
    let mut rust_bytes = input.bytes.clone();
    let mut c_bs = Bs {
        buf: c_bytes.as_mut_ptr(),
        pos: input.start_pos,
        limit: input.limit,
    };
    let mut rust_bs = Bs {
        buf: rust_bytes.as_mut_ptr(),
        pos: input.start_pos,
        limit: input.limit,
    };
    let mut c_gr = vec![unsafe { zeroed::<Gr>() }; input.gr_count];
    let mut rust_gr = vec![unsafe { zeroed::<Gr>() }; input.gr_count];

    let c_result =
        unsafe { (c_api.read_side_info)(&mut c_bs, c_gr.as_mut_ptr(), input.header.as_ptr()) };
    let rust_result = unsafe {
        (rust_api.read_side_info)(&mut rust_bs, rust_gr.as_mut_ptr(), input.header.as_ptr())
    };

    assert_eq!(c_result, rust_result, "{case}: return value differs");
    assert_eq!(c_bs.pos, rust_bs.pos, "{case}: final bit position differs");
    assert_eq!(
        c_bytes, rust_bytes,
        "{case}: input buffer was mutated differently"
    );
    for index in 0..input.gr_count {
        compare_gr(case, index, &c_gr[index], &rust_gr[index], c_result >= 0);
    }
    c_result
}

#[test]
fn configuration_table_matches_generator() {
    let configs = configurations();
    assert_eq!(configs.len(), 180);
    let markdown = fs::read_to_string(manifest_dir().join("CONFIGS.md")).expect("read CONFIGS.md");
    for config in configs {
        let row_fragment = format!("| {} | `read_side_info` | `{}` |", config.id, config.code);
        assert!(
            markdown.contains(&row_fragment),
            "CONFIGS.md is missing {row_fragment}"
        );
    }
    assert_eq!(
        markdown
            .lines()
            .filter(|line| line.starts_with("| C"))
            .count(),
        180
    );
}

#[test]
fn valid_configuration_surface_matches() {
    let (c_api, rust_api) = load_apis();
    for config in configurations() {
        for iteration in 0..32 {
            let input = make_input(&config, iteration, Fault::None);
            let case = format!("{} {} iteration {iteration}", config.id, config.code);
            let result = call_and_compare(&c_api, &rust_api, &input, &case);
            assert!(
                result >= 0,
                "{case}: generated valid input returned {result}"
            );
        }
    }
}

fn representative_config(shape: Shape) -> Config {
    Config {
        id: "error".to_string(),
        code: "error".to_string(),
        mode: Mode::M,
        sr_idx: 6,
        mono: false,
        high_scalefac: false,
        shape,
    }
}

#[test]
fn e1_bit_reader_exhaustion_matches() {
    let (c_api, rust_api) = load_apis();
    for iteration in 0..64 {
        let mut input = make_input(
            &representative_config(Shape::Normal),
            iteration,
            Fault::None,
        );
        input.limit = input.start_pos;
        let case = format!("E1 iteration {iteration}");
        assert_eq!(call_and_compare(&c_api, &rust_api, &input, &case), -1);
    }
}

#[test]
fn e2_big_values_above_288_matches() {
    let (c_api, rust_api) = load_apis();
    for iteration in 0..64 {
        let config = representative_config(if iteration & 1 == 0 {
            Shape::Normal
        } else {
            Shape::MixedShort
        });
        let input = make_input(&config, iteration, Fault::BigValues);
        let case = format!("E2 iteration {iteration}");
        assert_eq!(call_and_compare(&c_api, &rust_api, &input, &case), -1);
    }
}

#[test]
fn e3_switched_block_type_zero_matches() {
    let (c_api, rust_api) = load_apis();
    for iteration in 0..64 {
        let config = representative_config(Shape::Type1);
        let input = make_input(&config, iteration, Fault::BlockTypeZero);
        let case = format!("E3 iteration {iteration}");
        assert_eq!(call_and_compare(&c_api, &rust_api, &input, &case), -1);
    }
}

#[test]
fn e4_final_bit_budget_rejection_matches() {
    let (c_api, rust_api) = load_apis();
    for iteration in 0..64 {
        let config = representative_config(if iteration & 1 == 0 {
            Shape::Normal
        } else {
            Shape::PureShort
        });
        let input = make_input(&config, iteration, Fault::FinalBudget);
        let case = format!("E4 iteration {iteration}");
        assert_eq!(call_and_compare(&c_api, &rust_api, &input, &case), -1);
    }
}

#[test]
fn generic_bit_limit_boundaries_match() {
    let (c_api, rust_api) = load_apis();
    let config = representative_config(Shape::MixedShort);

    let mut zero = make_input(&config, 0, Fault::None);
    zero.start_pos = 0;
    zero.limit = 0;
    assert_eq!(
        call_and_compare(&c_api, &rust_api, &zero, "zero bit limit"),
        -1
    );

    for iteration in 0..64 {
        let mut oversized = make_input(&config, iteration, Fault::None);
        oversized.limit = (oversized.bytes.len() * 8) as c_int;
        let case = format!("oversized bit limit iteration {iteration}");
        assert!(
            call_and_compare(&c_api, &rust_api, &oversized, &case) >= 0,
            "{case}: expected accepted input"
        );
    }
}

#[test]
fn null_pointer_child() {
    let Ok(library_kind) = std::env::var("DIFF_NULL_LIBRARY") else {
        return;
    };
    let null_kind = std::env::var("DIFF_NULL_ARGUMENT").expect("null argument kind");
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown library kind"),
    };
    let api = unsafe { Api::load(&path) };
    let config = representative_config(Shape::Normal);
    let input = make_input(&config, 1, Fault::None);
    let mut bs = Bs {
        buf: input.bytes.as_ptr(),
        pos: input.start_pos,
        limit: input.limit,
    };
    let mut gr = vec![unsafe { zeroed::<Gr>() }; input.gr_count];
    let mut header = input.header;

    if null_kind == "buf" {
        bs.buf = std::ptr::null();
    }
    let bs_ptr = if null_kind == "bs" {
        std::ptr::null_mut()
    } else {
        &mut bs
    };
    let gr_ptr = if null_kind == "gr" {
        std::ptr::null_mut()
    } else {
        gr.as_mut_ptr()
    };
    let header_ptr = if null_kind == "hdr" {
        std::ptr::null()
    } else {
        header.as_mut_ptr()
    };

    unsafe {
        (api.read_side_info)(bs_ptr, gr_ptr, header_ptr);
    }
    panic!("{library_kind} unexpectedly returned for null {null_kind}");
}

#[test]
fn generic_null_pointer_behavior_matches() {
    let exe = std::env::current_exe().expect("current test executable");
    for argument in ["bs", "gr", "hdr", "buf"] {
        let run = |library: &str| {
            Command::new(&exe)
                .args(["--exact", "null_pointer_child", "--nocapture"])
                .env("DIFF_NULL_LIBRARY", library)
                .env("DIFF_NULL_ARGUMENT", argument)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap_or_else(|error| panic!("failed to run null child: {error}"))
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(!c_status.success(), "C returned for null {argument}");
        assert!(!rust_status.success(), "Rust returned for null {argument}");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "termination signal differs for null {argument}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}
