use libloading::Library;
use std::ffi::c_int;
use std::mem::{MaybeUninit, size_of};
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy)]
struct Bs {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct L3GrInfo {
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

type ReadSideInfo = unsafe extern "C" fn(*mut Bs, *mut L3GrInfo, *const u8) -> c_int;

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

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-B6YCD0.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libread_side_info_lib.so")
}

#[derive(Clone, Copy)]
struct HeaderProfile {
    hdr1: u8,
    sample_rate_selector: u8,
}

const PROFILES: [HeaderProfile; 9] = [
    HeaderProfile {
        hdr1: 0x00,
        sample_rate_selector: 0,
    },
    HeaderProfile {
        hdr1: 0x00,
        sample_rate_selector: 1,
    },
    HeaderProfile {
        hdr1: 0x00,
        sample_rate_selector: 2,
    },
    HeaderProfile {
        hdr1: 0x10,
        sample_rate_selector: 0,
    },
    HeaderProfile {
        hdr1: 0x10,
        sample_rate_selector: 1,
    },
    HeaderProfile {
        hdr1: 0x10,
        sample_rate_selector: 2,
    },
    HeaderProfile {
        hdr1: 0x18,
        sample_rate_selector: 0,
    },
    HeaderProfile {
        hdr1: 0x18,
        sample_rate_selector: 1,
    },
    HeaderProfile {
        hdr1: 0x18,
        sample_rate_selector: 2,
    },
];

#[derive(Clone, Copy)]
enum CodingShape {
    Normal,
    Switched1,
    Switched2Short,
    Switched2Mixed,
    Switched3,
}

const SHAPES: [CodingShape; 5] = [
    CodingShape::Normal,
    CodingShape::Switched1,
    CodingShape::Switched2Short,
    CodingShape::Switched2Mixed,
    CodingShape::Switched3,
];

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
    bit_len: usize,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_len: 0,
        }
    }

    fn write(&mut self, value: u32, width: usize) {
        for shift in (0..width).rev() {
            let byte_index = self.bit_len / 8;
            if byte_index == self.bytes.len() {
                self.bytes.push(0);
            }
            let bit = if shift < u32::BITS as usize {
                ((value >> shift) & 1) as u8
            } else {
                0
            };
            self.bytes[byte_index] |= bit << (7 - self.bit_len % 8);
            self.bit_len += 1;
        }
    }

    fn pad_random(&mut self, count: usize, rng: &mut Rng) {
        for _ in 0..count {
            self.write(rng.bits(1), 1);
        }
    }
}

struct Fixture {
    hdr: [u8; 4],
    bytes: Vec<u8>,
    pos: c_int,
    limit: c_int,
    granules: usize,
}

fn granule_count(hdr1: u8, mono: bool) -> usize {
    let base = if mono { 1 } else { 2 };
    if hdr1 & 0x08 != 0 { base * 2 } else { base }
}

fn valid_fixture(
    profile: HeaderProfile,
    mono: bool,
    shape: CodingShape,
    iteration: usize,
    seed: u64,
) -> Fixture {
    let mut rng = Rng::new(seed);
    let initial_pos = iteration % 8;
    let mut writer = BitWriter::new();
    writer.pad_random(initial_pos, &mut rng);

    let granules = granule_count(profile.hdr1, mono);
    let main_data_begin = match iteration % 4 {
        0 => 0,
        1 => 1,
        2 => 255,
        _ => rng.bits(if profile.hdr1 & 0x08 != 0 { 9 } else { 8 }),
    };
    if profile.hdr1 & 0x08 != 0 {
        writer.write(main_data_begin, 9);
        writer.write(rng.bits((7 + granules) as u32), 7 + granules);
    } else {
        writer.write(
            (main_data_begin << granules) | rng.bits(granules as u32),
            8 + granules,
        );
    }

    let mut part_23_sum = 0_usize;
    for granule in 0..granules {
        let part_23_length = match (iteration + granule) % 5 {
            0 => 0,
            1 => 1,
            2 => 4095,
            _ => rng.bits(12),
        };
        part_23_sum += part_23_length as usize;
        writer.write(part_23_length, 12);

        let big_values = match (iteration + granule) % 4 {
            0 => 0,
            1 => 288,
            _ => rng.next_u32() % 289,
        };
        writer.write(big_values, 9);
        writer.write(rng.bits(8), 8);

        let scalefac_width = if profile.hdr1 & 0x08 != 0 { 4 } else { 9 };
        let scalefac_compress = if scalefac_width == 9 {
            match (iteration + granule) % 4 {
                0 => 0,
                1 => 499,
                2 => 500,
                _ => rng.bits(9),
            }
        } else {
            rng.bits(4)
        };
        writer.write(scalefac_compress, scalefac_width);

        match shape {
            CodingShape::Normal => {
                writer.write(0, 1);
                writer.write(rng.bits(15), 15);
                writer.write(rng.bits(4), 4);
                writer.write(rng.bits(3), 3);
            }
            CodingShape::Switched1
            | CodingShape::Switched2Short
            | CodingShape::Switched2Mixed
            | CodingShape::Switched3 => {
                let (block_type, mixed) = match shape {
                    CodingShape::Switched1 => (1, rng.bits(1)),
                    CodingShape::Switched2Short => (2, 0),
                    CodingShape::Switched2Mixed => (2, 1),
                    CodingShape::Switched3 => (3, rng.bits(1)),
                    CodingShape::Normal => unreachable!(),
                };
                writer.write(1, 1);
                writer.write(block_type, 2);
                writer.write(mixed, 1);
                writer.write(rng.bits(10), 10);
                writer.write(rng.bits(3), 3);
                writer.write(rng.bits(3), 3);
                writer.write(rng.bits(3), 3);
            }
        }

        if profile.hdr1 & 0x08 != 0 {
            writer.write(rng.bits(1), 1);
        }
        writer.write(rng.bits(1), 1);
        writer.write(rng.bits(1), 1);
    }

    writer.pad_random(part_23_sum + (rng.next_u32() as usize & 31), &mut rng);
    let limit = writer.bit_len as c_int;
    Fixture {
        hdr: [
            rng.bits(8) as u8,
            profile.hdr1,
            profile.sample_rate_selector << 2,
            if mono { 0xc0 } else { 0x00 },
        ],
        bytes: writer.bytes,
        pos: initial_pos as c_int,
        limit,
        granules,
    }
}

fn initialized_granules() -> [L3GrInfo; 4] {
    let mut value = MaybeUninit::<[L3GrInfo; 4]>::uninit();
    unsafe {
        value
            .as_mut_ptr()
            .cast::<u8>()
            .write_bytes(0xa5, size_of::<[L3GrInfo; 4]>());
        value.assume_init()
    }
}

struct Outcome {
    result: c_int,
    bs: Bs,
    granules: [L3GrInfo; 4],
    _bytes: Vec<u8>,
}

unsafe fn invoke(api: &Api, fixture: &Fixture) -> Outcome {
    let bytes = fixture.bytes.clone();
    let mut bs = Bs {
        buf: bytes.as_ptr(),
        pos: fixture.pos,
        limit: fixture.limit,
    };
    let mut granules = initialized_granules();
    let result =
        unsafe { (api.read_side_info)(&mut bs, granules.as_mut_ptr(), fixture.hdr.as_ptr()) };
    Outcome {
        result,
        bs,
        granules,
        _bytes: bytes,
    }
}

fn compare_outcomes(c: &Outcome, rust: &Outcome, used_granules: usize, context: &str) {
    assert_eq!(c.result, rust.result, "{context}: return value");
    assert_eq!(c.bs.pos, rust.bs.pos, "{context}: bs.pos");
    assert_eq!(c.bs.limit, rust.bs.limit, "{context}: bs.limit");

    let pointer_size = size_of::<*const u8>();
    for index in 0..4 {
        let c_gr = &c.granules[index];
        let rust_gr = &rust.granules[index];
        let c_bytes = unsafe {
            std::slice::from_raw_parts(
                (c_gr as *const L3GrInfo).cast::<u8>().add(pointer_size),
                size_of::<L3GrInfo>() - pointer_size,
            )
        };
        let rust_bytes = unsafe {
            std::slice::from_raw_parts(
                (rust_gr as *const L3GrInfo).cast::<u8>().add(pointer_size),
                size_of::<L3GrInfo>() - pointer_size,
            )
        };
        assert_eq!(
            c_bytes, rust_bytes,
            "{context}: granule {index} bytes after sfbtab"
        );

        assert_eq!(
            c_gr.sfbtab.is_null(),
            rust_gr.sfbtab.is_null(),
            "{context}: granule {index} sfbtab nullness"
        );
        if c.result >= 0 && index < used_granules && !c_gr.sfbtab.is_null() {
            let table_len = if c_gr.n_short_sfb == 0 { 23 } else { 40 };
            let c_table = unsafe { std::slice::from_raw_parts(c_gr.sfbtab, table_len) };
            let rust_table = unsafe { std::slice::from_raw_parts(rust_gr.sfbtab, table_len) };
            assert_eq!(
                c_table, rust_table,
                "{context}: granule {index} sfbtab data"
            );
        }
    }
}

fn compare_fixture(c_api: &Api, rust_api: &Api, fixture: &Fixture, context: &str) {
    let c = unsafe { invoke(c_api, fixture) };
    let rust = unsafe { invoke(rust_api, fixture) };
    compare_outcomes(&c, &rust, fixture.granules, context);
}

fn load_apis() -> (Api, Api) {
    assert!(
        c_library_path().is_file(),
        "C shared library missing at {}",
        c_library_path().display()
    );
    assert!(
        rust_library_path().is_file(),
        "Rust shared library missing at {}",
        rust_library_path().display()
    );
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

#[test]
fn all_configuration_rows_match_randomized() {
    let (c_api, rust_api) = load_apis();
    let mut row = 0;
    for (profile_index, profile) in PROFILES.into_iter().enumerate() {
        for mono in [true, false] {
            for (shape_index, shape) in SHAPES.into_iter().enumerate() {
                row += 1;
                for iteration in 0..64 {
                    let seed = 0x6a09_e667_f3bc_c909_u64 ^ ((row as u64) << 32) ^ iteration as u64;
                    let fixture = valid_fixture(profile, mono, shape, iteration, seed);
                    compare_fixture(
                        &c_api,
                        &rust_api,
                        &fixture,
                        &format!(
                            "CONFIGS row {row}, profile {}, {}, shape {}, iteration {iteration}",
                            profile_index + 1,
                            if mono { "mono" } else { "non-mono" },
                            shape_index + 1,
                        ),
                    );
                }
            }
        }
    }
    assert_eq!(row, 90);
}

fn partial_fixture(
    hdr1: u8,
    mono: bool,
    initial_pos: usize,
    write_fields: impl FnOnce(&mut BitWriter),
) -> Fixture {
    let mut writer = BitWriter::new();
    writer.write(0, initial_pos);
    write_fields(&mut writer);
    writer.write(0, 64);
    Fixture {
        hdr: [0, hdr1, 0, if mono { 0xc0 } else { 0 }],
        limit: writer.bit_len as c_int,
        bytes: writer.bytes,
        pos: initial_pos as c_int,
        granules: granule_count(hdr1, mono),
    }
}

#[test]
fn error_surface_rows_match() {
    let (c_api, rust_api) = load_apis();

    let truncated = Fixture {
        hdr: [0, 0x10, 0, 0xc0],
        bytes: vec![0; 64],
        pos: 0,
        limit: 0,
        granules: 1,
    };
    let c = unsafe { invoke(&c_api, &truncated) };
    let rust = unsafe { invoke(&rust_api, &truncated) };
    compare_outcomes(&c, &rust, 1, "ERRORS row 1");
    assert_eq!(c.result, -1);

    let big_values = partial_fixture(0x10, true, 3, |writer| {
        writer.write(0, 9);
        writer.write(0, 12);
        writer.write(289, 9);
    });
    let c = unsafe { invoke(&c_api, &big_values) };
    let rust = unsafe { invoke(&rust_api, &big_values) };
    compare_outcomes(&c, &rust, 1, "ERRORS row 2");
    assert_eq!(c.result, -1);

    let block_type_zero = partial_fixture(0x10, true, 5, |writer| {
        writer.write(0, 9);
        writer.write(0, 12);
        writer.write(0, 9);
        writer.write(0, 8);
        writer.write(0, 9);
        writer.write(1, 1);
        writer.write(0, 2);
    });
    let c = unsafe { invoke(&c_api, &block_type_zero) };
    let rust = unsafe { invoke(&rust_api, &block_type_zero) };
    compare_outcomes(&c, &rust, 1, "ERRORS row 3");
    assert_eq!(c.result, -1);

    let mut final_sum = valid_fixture(PROFILES[3], true, CodingShape::Normal, 0, 7);
    // For this profile the first part_23_length occupies bits 9..20 and the
    // complete side-info payload ends at bit 72.
    final_sum.bytes[20 / 8] |= 1 << (7 - 20 % 8);
    final_sum.limit = 72;
    let c = unsafe { invoke(&c_api, &final_sum) };
    let rust = unsafe { invoke(&rust_api, &final_sum) };
    compare_outcomes(&c, &rust, 1, "ERRORS row 4");
    assert_eq!(c.bs.pos, 72);
    assert_eq!(c.result, -1);
}

#[test]
fn generic_length_boundaries_match() {
    let (c_api, rust_api) = load_apis();

    for limit in [-1, 0] {
        let fixture = Fixture {
            hdr: [0, 0x10, 0, 0xc0],
            bytes: vec![0; 64],
            pos: 0,
            limit,
            granules: 1,
        };
        compare_fixture(
            &c_api,
            &rust_api,
            &fixture,
            &format!("generic short limit {limit}"),
        );
    }

    let mut oversized = valid_fixture(PROFILES[3], true, CodingShape::Normal, 17, 0x1234_5678);
    oversized.limit = c_int::MAX;
    compare_fixture(&c_api, &rust_api, &oversized, "generic oversized limit");
}

#[test]
fn reserved_but_memory_safe_header_encodings_match() {
    let (c_api, rust_api) = load_apis();
    let profiles = [
        HeaderProfile {
            hdr1: 0x00,
            sample_rate_selector: 3,
        },
        HeaderProfile {
            hdr1: 0x08,
            sample_rate_selector: 0,
        },
        HeaderProfile {
            hdr1: 0x08,
            sample_rate_selector: 1,
        },
        HeaderProfile {
            hdr1: 0x08,
            sample_rate_selector: 2,
        },
        HeaderProfile {
            hdr1: 0x08,
            sample_rate_selector: 3,
        },
        HeaderProfile {
            hdr1: 0x10,
            sample_rate_selector: 3,
        },
    ];
    for (index, profile) in profiles.into_iter().enumerate() {
        for mono in [true, false] {
            for shape in SHAPES {
                for iteration in 0..16 {
                    let fixture = valid_fixture(
                        profile,
                        mono,
                        shape,
                        iteration,
                        0xbb67_ae85_84ca_a73b ^ (index as u64) << 32 ^ iteration as u64,
                    );
                    compare_fixture(
                        &c_api,
                        &rust_api,
                        &fixture,
                        &format!("reserved header profile {index}, iteration {iteration}"),
                    );
                }
            }
        }
    }
}

#[test]
fn null_pointer_child() {
    let Some(case) = std::env::var_os("DIFFERENTIAL_NULL_CASE") else {
        return;
    };
    let library = match std::env::var("DIFFERENTIAL_LIBRARY").as_deref() {
        Ok("c") => c_library_path(),
        Ok("rust") => rust_library_path(),
        other => panic!("unexpected DIFFERENTIAL_LIBRARY: {other:?}"),
    };
    let api = unsafe { Api::load(&library) };
    let bytes = [0_u8; 64];
    let hdr = [0_u8, 0x10, 0, 0xc0];
    let mut bs = Bs {
        buf: bytes.as_ptr(),
        pos: 0,
        limit: 512,
    };
    let mut granules = initialized_granules();
    unsafe {
        match case.to_str().unwrap() {
            "bs" => {
                (api.read_side_info)(std::ptr::null_mut(), granules.as_mut_ptr(), hdr.as_ptr());
            }
            "gr" => {
                (api.read_side_info)(&mut bs, std::ptr::null_mut(), hdr.as_ptr());
            }
            "hdr" => {
                (api.read_side_info)(&mut bs, granules.as_mut_ptr(), std::ptr::null());
            }
            other => panic!("unexpected null case: {other}"),
        }
    }
    panic!("null-pointer call unexpectedly returned");
}

#[cfg(unix)]
#[test]
fn null_pointer_process_failures_match() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().unwrap();
    for case in ["bs", "gr", "hdr"] {
        let run = |library: &str| {
            Command::new(&executable)
                .args(["--exact", "null_pointer_child", "--nocapture"])
                .env("DIFFERENTIAL_NULL_CASE", case)
                .env("DIFFERENTIAL_LIBRARY", library)
                .status()
                .unwrap()
        };
        let c = run("c");
        let rust = run("rust");
        assert!(!c.success(), "{case}: C unexpectedly accepted null");
        assert!(!rust.success(), "{case}: Rust unexpectedly accepted null");
        assert_eq!(
            c.signal(),
            rust.signal(),
            "{case}: process-termination signal differs"
        );
    }
}
