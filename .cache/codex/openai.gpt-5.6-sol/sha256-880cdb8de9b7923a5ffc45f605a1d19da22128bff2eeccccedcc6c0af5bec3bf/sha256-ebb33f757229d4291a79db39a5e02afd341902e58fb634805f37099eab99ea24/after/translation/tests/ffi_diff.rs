use libloading::Library;
use std::ffi::c_int;
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

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
    unsafe fn load(path: PathBuf) -> Self {
        let library = unsafe { Library::new(&path) }
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

fn c_library_path() -> PathBuf {
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build.display()))
        .map(|entry| entry.expect("invalid C build entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one reference .so in {}",
        build.display()
    );
    candidates.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libread_side_info_lib.so")
}

fn load_pair() -> (Api, Api) {
    unsafe { (Api::load(c_library_path()), Api::load(rust_library_path())) }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Version {
    Mpeg25,
    Mpeg2,
    Reserved,
    Mpeg1,
}

impl Version {
    fn header_bits(self) -> u8 {
        match self {
            Self::Mpeg25 => 0,
            Self::Mpeg2 => 0x10,
            Self::Reserved => 0x08,
            Self::Mpeg1 => 0x18,
        }
    }

    fn is_mpeg1(self) -> bool {
        matches!(self, Self::Reserved | Self::Mpeg1)
    }
}

#[derive(Clone, Copy, Debug)]
enum Layout {
    Long,
    Start,
    Short,
    Mixed,
    Stop,
}

#[derive(Clone, Copy, Debug)]
struct Config {
    row: usize,
    version: Version,
    sample_code: u8,
    alternate_sample_code: Option<u8>,
    mono: bool,
    layout: Layout,
    high_scalefac_compress: Option<bool>,
}

fn all_configs() -> Vec<Config> {
    let non_mpeg_samples = [
        (Version::Mpeg25, 0, Some(1)),
        (Version::Mpeg25, 2, None),
        (Version::Mpeg2, 0, None),
        (Version::Mpeg2, 1, None),
        (Version::Mpeg2, 2, None),
    ];
    let mpeg1_samples = [
        (Version::Mpeg1, 0, None),
        (Version::Mpeg1, 1, None),
        (Version::Mpeg1, 2, None),
    ];
    let layouts = [
        Layout::Long,
        Layout::Start,
        Layout::Short,
        Layout::Mixed,
        Layout::Stop,
    ];
    let mut configs = Vec::new();
    let mut row = 1;

    for (version, sample_code, alternate_sample_code) in non_mpeg_samples {
        for mono in [true, false] {
            for layout in layouts {
                for high_scalefac_compress in [false, true] {
                    configs.push(Config {
                        row,
                        version,
                        sample_code,
                        alternate_sample_code,
                        mono,
                        layout,
                        high_scalefac_compress: Some(high_scalefac_compress),
                    });
                    row += 1;
                }
            }
        }
    }

    for (version, sample_code, alternate_sample_code) in mpeg1_samples {
        for mono in [true, false] {
            for layout in layouts {
                configs.push(Config {
                    row,
                    version,
                    sample_code,
                    alternate_sample_code,
                    mono,
                    layout,
                    high_scalefac_compress: None,
                });
                row += 1;
            }
        }
    }

    for mono in [true, false] {
        for layout in layouts {
            for high_scalefac_compress in [false, true] {
                configs.push(Config {
                    row,
                    version: Version::Mpeg2,
                    sample_code: 3,
                    alternate_sample_code: None,
                    mono,
                    layout,
                    high_scalefac_compress: Some(high_scalefac_compress),
                });
                row += 1;
            }
        }
    }

    for sample_code in 0..=3 {
        for mono in [true, false] {
            for layout in layouts {
                configs.push(Config {
                    row,
                    version: Version::Reserved,
                    sample_code,
                    alternate_sample_code: None,
                    mono,
                    layout,
                    high_scalefac_compress: None,
                });
                row += 1;
            }
        }
    }

    for mono in [true, false] {
        for layout in layouts {
            configs.push(Config {
                row,
                version: Version::Mpeg1,
                sample_code: 3,
                alternate_sample_code: None,
                mono,
                layout,
                high_scalefac_compress: None,
            });
            row += 1;
        }
    }

    assert_eq!(row, 201);
    configs
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        (value >> 16) as u32
    }

    fn below(&mut self, upper: u32) -> u32 {
        self.next_u32() % upper
    }
}

struct BitWriter {
    bytes: Vec<u8>,
    bits: usize,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bits: 0,
        }
    }

    fn push(&mut self, value: u32, width: usize) {
        assert!(width <= 32);
        for shift in (0..width).rev() {
            let byte = self.bits / 8;
            let bit = 7 - self.bits % 8;
            if byte == self.bytes.len() {
                self.bytes.push(0);
            }
            if (value >> shift) & 1 != 0 {
                self.bytes[byte] |= 1 << bit;
            }
            self.bits += 1;
        }
    }

    fn backed_to(&mut self, limit: usize) {
        self.bytes.resize((limit + 7) / 8 + 8, 0);
    }
}

struct Case {
    bytes: Vec<u8>,
    header: [u8; 4],
    initial_pos: c_int,
    limit: c_int,
    granules: usize,
    table_bytes_defined: bool,
}

fn granule_count(version: Version, mono: bool) -> usize {
    match (version.is_mpeg1(), mono) {
        (true, true) => 2,
        (true, false) => 4,
        (false, true) => 1,
        (false, false) => 2,
    }
}

fn make_header(version: Version, sample_code: u8, mono: bool, rng: &mut Rng) -> [u8; 4] {
    let mut header = [
        rng.next_u32() as u8,
        rng.next_u32() as u8,
        rng.next_u32() as u8,
        rng.next_u32() as u8,
    ];
    header[1] = (header[1] & !0x18) | version.header_bits();
    header[2] = (header[2] & !0x0c) | (sample_code << 2);
    header[3] = if mono {
        (header[3] & 0x3f) | 0xc0
    } else {
        (header[3] & 0x3f) | [0x00, 0x40, 0x80][rng.below(3) as usize]
    };
    header
}

fn build_valid_case(config: Config, iteration: usize) -> Case {
    let seed = 0x9e37_79b9_7f4a_7c15_u64
        ^ (config.row as u64).wrapping_mul(0xd1b5_4a32_d192_ed03)
        ^ iteration as u64;
    let mut rng = Rng::new(seed);
    let sample_code = if iteration % 2 == 1 {
        config.alternate_sample_code.unwrap_or(config.sample_code)
    } else {
        config.sample_code
    };
    let header = make_header(config.version, sample_code, config.mono, &mut rng);
    let raw_sr_idx = sample_code as usize
        + match config.version {
            Version::Mpeg25 => 0,
            Version::Mpeg2 | Version::Reserved => 3,
            Version::Mpeg1 => 6,
        };
    let sr_idx = raw_sr_idx.saturating_sub(usize::from(raw_sr_idx != 0));
    let granules = granule_count(config.version, config.mono);
    let initial_pos = iteration % 8;
    let mut writer = BitWriter::new();
    writer.push(rng.next_u32(), initial_pos);

    if config.version.is_mpeg1() {
        writer.push(rng.below(512), 9);
        writer.push(rng.next_u32(), 7 + granules);
    } else {
        let main_data_begin = rng.below(256);
        let discarded = rng.next_u32() & ((1_u32 << granules) - 1);
        writer.push((main_data_begin << granules) | discarded, 8 + granules);
    }

    let mut part_23_sum = 0_usize;
    for _ in 0..granules {
        let part_23_length = rng.below(65);
        part_23_sum += part_23_length as usize;
        writer.push(part_23_length, 12);
        writer.push(rng.below(289), 9);
        writer.push(rng.next_u32(), 8);
        if config.version.is_mpeg1() {
            writer.push(rng.below(16), 4);
        } else if config.high_scalefac_compress == Some(true) {
            writer.push(500 + rng.below(12), 9);
        } else {
            writer.push(rng.below(500), 9);
        }

        match config.layout {
            Layout::Long => {
                writer.push(0, 1);
                writer.push(rng.next_u32(), 15);
                writer.push(rng.next_u32(), 4);
                writer.push(rng.next_u32(), 3);
            }
            Layout::Start | Layout::Short | Layout::Mixed | Layout::Stop => {
                writer.push(1, 1);
                let block_type = match config.layout {
                    Layout::Start => 1,
                    Layout::Short | Layout::Mixed => 2,
                    Layout::Stop => 3,
                    Layout::Long => unreachable!(),
                };
                writer.push(block_type, 2);
                let mixed = match config.layout {
                    Layout::Mixed => 1,
                    Layout::Short => 0,
                    Layout::Start | Layout::Stop => rng.below(2),
                    Layout::Long => unreachable!(),
                };
                writer.push(mixed, 1);
                writer.push(rng.next_u32(), 10);
                writer.push(rng.next_u32(), 3);
                writer.push(rng.next_u32(), 3);
                writer.push(rng.next_u32(), 3);
            }
        }

        if config.version.is_mpeg1() {
            writer.push(rng.below(2), 1);
        }
        writer.push(rng.below(2), 1);
        writer.push(rng.below(2), 1);
    }

    let limit = writer.bits + part_23_sum;
    writer.backed_to(limit);
    Case {
        bytes: writer.bytes,
        header,
        initial_pos: initial_pos as c_int,
        limit: limit as c_int,
        granules,
        table_bytes_defined: sr_idx < 8,
    }
}

struct CallResult {
    return_value: c_int,
    bs: Bs,
    gr: [Gr; 4],
}

fn call(api: &Api, case: &Case) -> CallResult {
    let mut bs = Bs {
        buf: case.bytes.as_ptr(),
        pos: case.initial_pos,
        limit: case.limit,
    };
    let mut gr: [Gr; 4] = unsafe { zeroed() };
    let return_value =
        unsafe { (api.read_side_info)(&mut bs, gr.as_mut_ptr(), case.header.as_ptr()) };
    CallResult {
        return_value,
        bs,
        gr,
    }
}

fn gr_bytes_without_pointer(gr: &Gr) -> &[u8] {
    let pointer_size = size_of::<*const u8>();
    unsafe {
        std::slice::from_raw_parts(
            (gr as *const Gr as *const u8).add(pointer_size),
            size_of::<Gr>() - pointer_size,
        )
    }
}

fn assert_calls_equal(c: &CallResult, rust: &CallResult, context: &str, compare_table_bytes: bool) {
    assert_eq!(
        c.return_value, rust.return_value,
        "return value differs: {context}"
    );
    assert_eq!(
        c.bs.pos, rust.bs.pos,
        "final bit position differs: {context}"
    );
    assert_eq!(
        c.bs.limit, rust.bs.limit,
        "bitstream limit was mutated differently: {context}"
    );

    for index in 0..4 {
        assert_eq!(
            gr_bytes_without_pointer(&c.gr[index]),
            gr_bytes_without_pointer(&rust.gr[index]),
            "granule {index} bytes differ: {context}"
        );
        assert_eq!(
            c.gr[index].sfbtab.is_null(),
            rust.gr[index].sfbtab.is_null(),
            "granule {index} table nullness differs: {context}"
        );
        if compare_table_bytes && !c.gr[index].sfbtab.is_null() {
            let length = if c.gr[index].n_short_sfb == 0 { 23 } else { 40 };
            let c_table = unsafe { std::slice::from_raw_parts(c.gr[index].sfbtab, length) };
            let rust_table = unsafe { std::slice::from_raw_parts(rust.gr[index].sfbtab, length) };
            assert_eq!(
                c_table, rust_table,
                "granule {index} scalefactor table differs: {context}"
            );
        }
    }

    for left in 0..4 {
        for right in 0..4 {
            assert_eq!(
                c.gr[left].sfbtab == c.gr[right].sfbtab,
                rust.gr[left].sfbtab == rust.gr[right].sfbtab,
                "table pointer aliasing differs for granules {left}/{right}: {context}"
            );
        }
    }
}

#[test]
fn all_valid_configuration_rows_match() {
    let (c, rust) = load_pair();
    let configs = all_configs();
    assert_eq!(configs.len(), 200);

    for config in configs {
        for iteration in 0..32 {
            let case = build_valid_case(config, iteration);
            let context = format!(
                "CONFIGS.md row {}, iteration {}, {:?}",
                config.row, iteration, config
            );
            let c_result = call(&c, &case);
            let rust_result = call(&rust, &case);
            assert_calls_equal(&c_result, &rust_result, &context, case.table_bytes_defined);
            assert!(
                c_result.return_value >= 0,
                "reference unexpectedly rejected valid case: {context}"
            );
            assert_eq!(
                case.granules,
                granule_count(config.version, config.mono),
                "internal test granule count mismatch: {context}"
            );
        }
    }
}

fn mpeg2_mono_header() -> [u8; 4] {
    [0, 0x10, 0, 0xc0]
}

fn finish_case(mut writer: BitWriter, limit: usize) -> Case {
    writer.backed_to(limit);
    Case {
        bytes: writer.bytes,
        header: mpeg2_mono_header(),
        initial_pos: 0,
        limit: limit as c_int,
        granules: 1,
        table_bytes_defined: true,
    }
}

fn big_values_289_case() -> Case {
    let mut writer = BitWriter::new();
    writer.push(0, 9);
    writer.push(0, 12);
    writer.push(289, 9);
    let limit = writer.bits;
    finish_case(writer, limit)
}

fn switched_block_type_zero_case() -> Case {
    let mut writer = BitWriter::new();
    writer.push(0, 9);
    writer.push(0, 12);
    writer.push(0, 9);
    writer.push(0, 8);
    writer.push(0, 9);
    writer.push(1, 1);
    writer.push(0, 2);
    let limit = writer.bits;
    finish_case(writer, limit)
}

fn final_reservoir_failure_case() -> Case {
    let mut writer = BitWriter::new();
    writer.push(0, 9);
    writer.push(1, 12);
    writer.push(0, 9);
    writer.push(0, 8);
    writer.push(0, 9);
    writer.push(0, 1);
    writer.push(0, 15);
    writer.push(0, 4);
    writer.push(0, 3);
    writer.push(0, 1);
    writer.push(0, 1);
    let limit = writer.bits;
    finish_case(writer, limit)
}

fn zero_limit_case() -> Case {
    Case {
        bytes: vec![0; 256],
        header: mpeg2_mono_header(),
        initial_pos: 0,
        limit: 0,
        granules: 1,
        table_bytes_defined: true,
    }
}

fn oversized_limit_case() -> Case {
    Case {
        bytes: vec![0; 256],
        header: mpeg2_mono_header(),
        initial_pos: 0,
        limit: 1_000_000,
        granules: 1,
        table_bytes_defined: true,
    }
}

fn assert_error_case(case: Case, expected: c_int, context: &str) {
    let (c, rust) = load_pair();
    let c_result = call(&c, &case);
    let rust_result = call(&rust, &case);
    assert_calls_equal(&c_result, &rust_result, context, case.table_bytes_defined);
    assert_eq!(c_result.return_value, expected, "{context}");
}

#[test]
fn error_row_1_truncated_zero_limit_matches() {
    assert_error_case(zero_limit_case(), -1, "ERRORS.md row 1 / zero limit");
}

#[test]
fn error_row_2_big_values_one_past_max_matches() {
    assert_error_case(
        big_values_289_case(),
        -1,
        "ERRORS.md row 2 / big_values=289",
    );
}

#[test]
fn error_row_3_switched_block_type_zero_matches() {
    assert_error_case(
        switched_block_type_zero_case(),
        -1,
        "ERRORS.md row 3 / switched block_type=0",
    );
}

#[test]
fn error_row_4_final_reservoir_check_matches() {
    assert_error_case(
        final_reservoir_failure_case(),
        -1,
        "ERRORS.md row 4 / reservoir bound",
    );
}

#[test]
fn generic_oversized_but_safely_backed_limit_matches() {
    assert_error_case(
        oversized_limit_case(),
        0,
        "generic oversized but safely backed limit",
    );
}

fn null_probe_status(library: &str, target: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--ignored")
        .arg("--exact")
        .arg("ffi_null_probe")
        .env("FFI_NULL_PROBE_LIBRARY", library)
        .env("FFI_NULL_PROBE_TARGET", target)
        .status()
        .unwrap_or_else(|error| panic!("failed to run null probe {library}/{target}: {error}"))
}

#[test]
#[cfg(unix)]
fn null_pointer_process_behavior_matches() {
    for target in ["bs", "gr", "hdr"] {
        let c_status = null_probe_status("c", target);
        let rust_status = null_probe_status("rust", target);
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "null {target} terminated C and Rust differently: C={c_status:?}, Rust={rust_status:?}"
        );
        assert!(
            c_status.signal().is_some(),
            "null {target} unexpectedly returned normally: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
#[ignore]
fn ffi_null_probe() {
    let library = std::env::var("FFI_NULL_PROBE_LIBRARY")
        .expect("FFI_NULL_PROBE_LIBRARY is required for the ignored helper");
    let target = std::env::var("FFI_NULL_PROBE_TARGET")
        .expect("FFI_NULL_PROBE_TARGET is required for the ignored helper");
    let path = match library.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown probe library {library}"),
    };
    let api = unsafe { Api::load(path) };
    let bytes = [0_u8; 256];
    let header = mpeg2_mono_header();
    let mut bs = Bs {
        buf: bytes.as_ptr(),
        pos: 0,
        limit: 1_000_000,
    };
    let mut gr: [Gr; 4] = unsafe { zeroed() };

    unsafe {
        match target.as_str() {
            "bs" => (api.read_side_info)(std::ptr::null_mut(), gr.as_mut_ptr(), header.as_ptr()),
            "gr" => (api.read_side_info)(&mut bs, std::ptr::null_mut(), header.as_ptr()),
            "hdr" => (api.read_side_info)(&mut bs, gr.as_mut_ptr(), std::ptr::null()),
            _ => panic!("unknown probe target {target}"),
        };
    }
    std::process::exit(90);
}
