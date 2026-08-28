use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::mem::{ManuallyDrop, size_of};
use std::path::{Path, PathBuf};
use std::process::Command;

const RANDOM_CASES: usize = 128;
const INFO_SIZE: usize = size_of::<ImaInfo>();

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ImaBlock {
    preamble: u16,
    data: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ImaInfo {
    blocks: *const ImaBlock,
    size: u64,
    sample_rate: f64,
    frame_count: u64,
    channel_count: u32,
}

#[repr(C)]
union InfoStorage {
    info: ManuallyDrop<ImaInfo>,
    bytes: [u8; INFO_SIZE],
}

impl InfoStorage {
    fn initialized() -> Self {
        Self {
            bytes: [0xa5; INFO_SIZE],
        }
    }

    fn info_ptr(&mut self) -> *mut ImaInfo {
        (&raw mut self.info).cast::<ImaInfo>()
    }

    fn bytes(&self) -> [u8; INFO_SIZE] {
        unsafe { self.bytes }
    }

    fn info(&self) -> ImaInfo {
        unsafe { self.info_ptr_const().read_unaligned() }
    }

    fn info_ptr_const(&self) -> *const ImaInfo {
        (&raw const self.info).cast::<ImaInfo>()
    }
}

type ParseFn = unsafe extern "C" fn(*mut ImaInfo, *const c_void) -> c_int;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        Self {
            c: unsafe { Library::new(&c_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display())),
            rust: unsafe { Library::new(&rust_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display())),
        }
    }

    fn compare(&self, input: &[u8]) -> Comparison {
        let c_parse: Symbol<'_, ParseFn> =
            unsafe { self.c.get(b"ima_parse\0") }.expect("C ima_parse export");
        let rust_parse: Symbol<'_, ParseFn> =
            unsafe { self.rust.get(b"ima_parse\0") }.expect("Rust ima_parse export");
        let mut c_output = InfoStorage::initialized();
        let mut rust_output = InfoStorage::initialized();

        let c_result = unsafe { c_parse(c_output.info_ptr(), input.as_ptr().cast()) };
        let rust_result = unsafe { rust_parse(rust_output.info_ptr(), input.as_ptr().cast()) };

        assert_eq!(rust_result, c_result, "return code differs");
        assert_eq!(
            rust_output.bytes(),
            c_output.bytes(),
            "ImaInfo bytes differ for return code {c_result}"
        );

        Comparison {
            result: c_result,
            info: c_output.info(),
        }
    }
}

#[derive(Debug)]
struct Comparison {
    result: c_int,
    info: ImaInfo,
}

#[derive(Clone, Copy)]
struct Description {
    sample_rate_bits: u64,
    format_id: [u8; 4],
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
}

#[derive(Clone, Copy)]
struct PacketTable {
    packet_count: i64,
    frame_count: i64,
    priming_frames: i32,
    remainder_frames: i32,
}

struct CaseValues {
    header_flags: u16,
    first_desc: Description,
    second_desc: Description,
    first_pakt: PacketTable,
    second_pakt: PacketTable,
    edit_count: u32,
    blocks: Vec<u8>,
}

impl CaseValues {
    fn generate(rng: &mut Rng, iteration: usize) -> Self {
        let sample_rate_bits = match iteration % 10 {
            0 => 0.0f64.to_bits(),
            1 => (-0.0f64).to_bits(),
            2 => 1.0f64.to_bits(),
            3 => (-44_100.5f64).to_bits(),
            4 => f64::INFINITY.to_bits(),
            5 => f64::NEG_INFINITY.to_bits(),
            6 => f64::NAN.to_bits(),
            _ => rng.next_u64(),
        };
        let second_sample_rate_bits = sample_rate_bits ^ 0x8040_0000_0000_0042;
        let block_count = iteration % 5;
        let mut blocks = vec![0u8; block_count * size_of::<ImaBlock>()];
        rng.fill(&mut blocks);

        Self {
            header_flags: rng.next_u16(),
            first_desc: Description {
                sample_rate_bits,
                format_id: *b"ima4",
                format_flags: rng.next_u32(),
                bytes_per_packet: rng.next_u32(),
                frames_per_packet: rng.next_u32(),
                channels_per_frame: rng.next_u32(),
                bits_per_channel: rng.next_u32(),
            },
            second_desc: Description {
                sample_rate_bits: second_sample_rate_bits,
                format_id: *b"ima4",
                format_flags: rng.next_u32(),
                bytes_per_packet: rng.next_u32(),
                frames_per_packet: rng.next_u32(),
                channels_per_frame: rng.next_u32(),
                bits_per_channel: rng.next_u32(),
            },
            first_pakt: PacketTable {
                packet_count: rng.next_u64() as i64,
                frame_count: rng.next_u64() as i64,
                priming_frames: rng.next_u32() as i32,
                remainder_frames: rng.next_u32() as i32,
            },
            second_pakt: PacketTable {
                packet_count: rng.next_u64() as i64,
                frame_count: rng.next_u64() as i64,
                priming_frames: rng.next_u32() as i32,
                remainder_frames: rng.next_u32() as i32,
            },
            edit_count: rng.next_u32(),
            blocks,
        }
    }
}

struct Caf {
    bytes: Vec<u8>,
}

impl Caf {
    fn new(flags: u16) -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"caff");
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&flags.to_be_bytes());
        Self { bytes }
    }

    fn description(&mut self, value: Description, padding: u32) {
        let mut payload = Vec::with_capacity(32);
        payload.extend_from_slice(&value.sample_rate_bits.to_be_bytes());
        payload.extend_from_slice(&value.format_id);
        payload.extend_from_slice(&value.format_flags.to_be_bytes());
        payload.extend_from_slice(&value.bytes_per_packet.to_be_bytes());
        payload.extend_from_slice(&value.frames_per_packet.to_be_bytes());
        payload.extend_from_slice(&value.channels_per_frame.to_be_bytes());
        payload.extend_from_slice(&value.bits_per_channel.to_be_bytes());
        assert_eq!(payload.len(), 32);
        self.chunk(*b"desc", payload.len() as i64, padding, &payload);
    }

    fn packet_table(&mut self, value: PacketTable, padding: u32) {
        let mut payload = Vec::with_capacity(24);
        payload.extend_from_slice(&value.packet_count.to_be_bytes());
        payload.extend_from_slice(&value.frame_count.to_be_bytes());
        payload.extend_from_slice(&value.priming_frames.to_be_bytes());
        payload.extend_from_slice(&value.remainder_frames.to_be_bytes());
        assert_eq!(payload.len(), 24);
        self.chunk(*b"pakt", payload.len() as i64, padding, &payload);
    }

    fn unknown(&mut self, payload: &[u8], padding: u32) {
        assert_eq!(payload.len() % 8, 0, "keep following C chunks aligned");
        self.chunk(*b"free", payload.len() as i64, padding, payload);
    }

    fn data(&mut self, declared_size: i64, edit_count: u32, blocks: &[u8], padding: u32) {
        let mut payload = Vec::with_capacity(4 + blocks.len());
        payload.extend_from_slice(&edit_count.to_be_bytes());
        payload.extend_from_slice(blocks);
        self.chunk(*b"data", declared_size, padding, &payload);
    }

    fn chunk(&mut self, kind: [u8; 4], size: i64, padding: u32, payload: &[u8]) {
        assert_eq!(self.bytes.len() % 8, 0, "CafChunk must remain aligned");
        self.bytes.extend_from_slice(&kind);
        self.bytes.extend_from_slice(&padding.to_ne_bytes());
        self.bytes.extend_from_slice(&size.to_be_bytes());
        self.bytes.extend_from_slice(payload);
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

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

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u16(&mut self) -> u16 {
        self.next_u64() as u16
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

fn run_random_cases(seed: u64, mut build: impl FnMut(&CaseValues, usize) -> Vec<u8>) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..RANDOM_CASES {
        let values = CaseValues::generate(&mut rng, iteration);
        let input = build(&values, iteration);
        let comparison = libraries.compare(&input);
        assert_eq!(comparison.result, 0, "valid case {iteration} was rejected");
        let input_start = input.as_ptr() as usize;
        let input_end = input_start + input.len();
        let blocks = comparison.info.blocks as usize;
        assert!(
            (input_start..=input_end).contains(&blocks),
            "blocks pointer is outside the input"
        );
    }
}

#[test]
fn config_01_minimal_desc_pakt_data() {
    run_random_cases(0x01d1_5ea5_e000_0001, |v, _| {
        let mut caf = Caf::new(v.header_flags);
        caf.description(v.first_desc, 0x1111_1111);
        caf.packet_table(v.first_pakt, 0x2222_2222);
        caf.data(
            (4 + v.blocks.len()) as i64,
            v.edit_count,
            &v.blocks,
            0x3333_3333,
        );
        caf.finish()
    });
}

#[test]
fn config_02_reversed_metadata_order() {
    run_random_cases(0x02d1_5ea5_e000_0002, |v, _| {
        let mut caf = Caf::new(v.header_flags);
        caf.packet_table(v.first_pakt, 0x1111_1111);
        caf.description(v.first_desc, 0x2222_2222);
        caf.data(
            (4 + v.blocks.len()) as i64,
            v.edit_count,
            &v.blocks,
            0x3333_3333,
        );
        caf.finish()
    });
}

#[test]
fn config_03_unknown_before_metadata() {
    run_random_cases(0x03d1_5ea5_e000_0003, |v, iteration| {
        let mut caf = Caf::new(v.header_flags);
        let opaque = vec![iteration as u8; (iteration % 5) * 8];
        caf.unknown(&opaque, 0x1111_1111);
        caf.description(v.first_desc, 0x2222_2222);
        caf.packet_table(v.first_pakt, 0x3333_3333);
        caf.data(
            (4 + v.blocks.len()) as i64,
            v.edit_count,
            &v.blocks,
            0x4444_4444,
        );
        caf.finish()
    });
}

#[test]
fn config_04_unknown_between_metadata() {
    run_random_cases(0x04d1_5ea5_e000_0004, |v, iteration| {
        let mut caf = Caf::new(v.header_flags);
        caf.description(v.first_desc, 0x1111_1111);
        let opaque = vec![iteration as u8; (iteration % 5) * 8];
        caf.unknown(&opaque, 0x2222_2222);
        caf.packet_table(v.first_pakt, 0x3333_3333);
        caf.data(
            (4 + v.blocks.len()) as i64,
            v.edit_count,
            &v.blocks,
            0x4444_4444,
        );
        caf.finish()
    });
}

#[test]
fn config_05_unknown_after_metadata() {
    run_random_cases(0x05d1_5ea5_e000_0005, |v, iteration| {
        let mut caf = Caf::new(v.header_flags);
        caf.description(v.first_desc, 0x1111_1111);
        caf.packet_table(v.first_pakt, 0x2222_2222);
        let opaque = vec![iteration as u8; (iteration % 5) * 8];
        caf.unknown(&opaque, 0x3333_3333);
        caf.data(
            (4 + v.blocks.len()) as i64,
            v.edit_count,
            &v.blocks,
            0x4444_4444,
        );
        caf.finish()
    });
}

#[test]
fn config_06_duplicate_descriptions_last_wins() {
    run_random_cases(0x06d1_5ea5_e000_0006, |v, _| {
        let mut caf = Caf::new(v.header_flags);
        caf.description(v.first_desc, 0x1111_1111);
        caf.description(v.second_desc, 0x2222_2222);
        caf.packet_table(v.first_pakt, 0x3333_3333);
        caf.data(
            (4 + v.blocks.len()) as i64,
            v.edit_count,
            &v.blocks,
            0x4444_4444,
        );
        caf.finish()
    });
}

#[test]
fn config_07_duplicate_packet_tables_last_wins() {
    run_random_cases(0x07d1_5ea5_e000_0007, |v, _| {
        let mut caf = Caf::new(v.header_flags);
        caf.packet_table(v.first_pakt, 0x1111_1111);
        caf.packet_table(v.second_pakt, 0x2222_2222);
        caf.description(v.first_desc, 0x3333_3333);
        caf.data(
            (4 + v.blocks.len()) as i64,
            v.edit_count,
            &v.blocks,
            0x4444_4444,
        );
        caf.finish()
    });
}

#[test]
fn config_08_unknown_positive_forward_jump() {
    run_random_cases(0x08d1_5ea5_e000_0008, |v, iteration| {
        let mut caf = Caf::new(v.header_flags);
        caf.description(v.first_desc, 0x1111_1111);
        let mut opaque = vec![0u8; 8 + (iteration % 8) * 8];
        for (index, byte) in opaque.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(37).wrapping_add(iteration as u8);
        }
        caf.unknown(&opaque, 0x2222_2222);
        caf.packet_table(v.first_pakt, 0x3333_3333);
        caf.data(
            (4 + v.blocks.len()) as i64,
            v.edit_count,
            &v.blocks,
            0x4444_4444,
        );
        caf.finish()
    });
}

#[test]
fn config_09_unknown_negative_backward_jump() {
    run_random_cases(0x09d1_5ea5_e000_0009, |v, _| negative_jump_caf(v));
}

#[test]
fn config_10_zero_data_size() {
    run_random_cases(0x10d1_5ea5_e000_0010, |v, _| {
        let mut caf = Caf::new(v.header_flags);
        caf.description(v.first_desc, 0x1111_1111);
        caf.packet_table(v.first_pakt, 0x2222_2222);
        caf.data(0, v.edit_count, &v.blocks, 0x3333_3333);
        caf.finish()
    });
}

#[test]
fn config_11_negative_data_sizes() {
    run_random_cases(0x11d1_5ea5_e000_0011, |v, iteration| {
        let declared_size = match iteration {
            0 => -1,
            1 => i64::MIN,
            _ => -((iteration as i64) + 1),
        };
        let mut caf = Caf::new(v.header_flags);
        caf.description(v.first_desc, 0x1111_1111);
        caf.packet_table(v.first_pakt, 0x2222_2222);
        caf.data(declared_size, v.edit_count, &v.blocks, 0x3333_3333);
        caf.finish()
    });
}

#[test]
fn config_12_large_positive_data_sizes() {
    run_random_cases(0x12d1_5ea5_e000_0012, |v, iteration| {
        let declared_size = match iteration {
            0 => 1,
            1 => i64::MAX,
            _ => (iteration as i64) << 40 | 0x5a5a,
        };
        let mut caf = Caf::new(v.header_flags);
        caf.description(v.first_desc, 0x1111_1111);
        caf.packet_table(v.first_pakt, 0x2222_2222);
        caf.data(declared_size, v.edit_count, &v.blocks, 0x3333_3333);
        caf.finish()
    });
}

fn negative_jump_caf(values: &CaseValues) -> Vec<u8> {
    const HIDDEN_PAKT: usize = 32;
    const DESC: usize = 88;
    const NORMAL_PAKT: usize = 136;
    const BACKWARD: usize = 176;
    const DATA: usize = 208;

    let mut bytes = vec![0u8; DATA];
    bytes[0..4].copy_from_slice(b"caff");
    bytes[4..6].copy_from_slice(&1u16.to_be_bytes());
    bytes[6..8].copy_from_slice(&values.header_flags.to_be_bytes());

    write_chunk_header(&mut bytes, 8, *b"free", 64, 0x1111_1111);
    write_chunk_header(
        &mut bytes,
        HIDDEN_PAKT,
        *b"pakt",
        (DATA - HIDDEN_PAKT - 16) as i64,
        0x2222_2222,
    );
    write_packet_payload(&mut bytes, HIDDEN_PAKT + 16, values.second_pakt);

    write_chunk_header(&mut bytes, DESC, *b"desc", 32, 0x3333_3333);
    write_description_payload(&mut bytes, DESC + 16, values.first_desc);
    write_chunk_header(&mut bytes, NORMAL_PAKT, *b"pakt", 24, 0x4444_4444);
    write_packet_payload(&mut bytes, NORMAL_PAKT + 16, values.first_pakt);
    write_chunk_header(
        &mut bytes,
        BACKWARD,
        *b"free",
        HIDDEN_PAKT as i64 - (BACKWARD + 16) as i64,
        0x5555_5555,
    );

    bytes.resize(DATA + 16, 0);
    write_chunk_header(
        &mut bytes,
        DATA,
        *b"data",
        (4 + values.blocks.len()) as i64,
        0x6666_6666,
    );
    bytes.extend_from_slice(&values.edit_count.to_be_bytes());
    bytes.extend_from_slice(&values.blocks);
    bytes
}

fn write_chunk_header(bytes: &mut Vec<u8>, offset: usize, kind: [u8; 4], size: i64, padding: u32) {
    if bytes.len() < offset + 16 {
        bytes.resize(offset + 16, 0);
    }
    bytes[offset..offset + 4].copy_from_slice(&kind);
    bytes[offset + 4..offset + 8].copy_from_slice(&padding.to_ne_bytes());
    bytes[offset + 8..offset + 16].copy_from_slice(&size.to_be_bytes());
}

fn write_description_payload(bytes: &mut [u8], offset: usize, value: Description) {
    bytes[offset..offset + 8].copy_from_slice(&value.sample_rate_bits.to_be_bytes());
    bytes[offset + 8..offset + 12].copy_from_slice(&value.format_id);
    bytes[offset + 12..offset + 16].copy_from_slice(&value.format_flags.to_be_bytes());
    bytes[offset + 16..offset + 20].copy_from_slice(&value.bytes_per_packet.to_be_bytes());
    bytes[offset + 20..offset + 24].copy_from_slice(&value.frames_per_packet.to_be_bytes());
    bytes[offset + 24..offset + 28].copy_from_slice(&value.channels_per_frame.to_be_bytes());
    bytes[offset + 28..offset + 32].copy_from_slice(&value.bits_per_channel.to_be_bytes());
}

fn write_packet_payload(bytes: &mut [u8], offset: usize, value: PacketTable) {
    bytes[offset..offset + 8].copy_from_slice(&value.packet_count.to_be_bytes());
    bytes[offset + 8..offset + 16].copy_from_slice(&value.frame_count.to_be_bytes());
    bytes[offset + 16..offset + 20].copy_from_slice(&value.priming_frames.to_be_bytes());
    bytes[offset + 20..offset + 24].copy_from_slice(&value.remainder_frames.to_be_bytes());
}

#[test]
fn error_01_invalid_file_type_returns_minus_one() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xe001_e001_e001_e001);
    for _ in 0..RANDOM_CASES {
        let mut input = [0u8; 8];
        rng.fill(&mut input);
        if input[0..4] == *b"caff" {
            input[0] ^= 0xff;
        }
        let comparison = libraries.compare(&input);
        assert_eq!(comparison.result, -1);
    }
}

#[test]
fn error_02_invalid_version_returns_minus_two() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xe002_e002_e002_e002);
    for _ in 0..RANDOM_CASES {
        let mut input = [0u8; 8];
        input[0..4].copy_from_slice(b"caff");
        let mut version = rng.next_u16();
        if version == 1 {
            version = 2;
        }
        input[4..6].copy_from_slice(&version.to_be_bytes());
        input[6..8].copy_from_slice(&rng.next_u16().to_be_bytes());
        let comparison = libraries.compare(&input);
        assert_eq!(comparison.result, -2);
    }
}

#[test]
fn error_03_invalid_format_returns_minus_three() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xe003_e003_e003_e003);
    for iteration in 0..RANDOM_CASES {
        let values = CaseValues::generate(&mut rng, iteration);
        let mut bad_desc = values.first_desc;
        bad_desc.format_id = rng.next_u32().to_ne_bytes();
        if bad_desc.format_id == *b"ima4" {
            bad_desc.format_id[0] ^= 0xff;
        }
        let mut caf = Caf::new(values.header_flags);
        caf.description(bad_desc, 0x1111_1111);
        caf.packet_table(values.first_pakt, 0x2222_2222);
        caf.data(
            (4 + values.blocks.len()) as i64,
            values.edit_count,
            &values.blocks,
            0x3333_3333,
        );
        let comparison = libraries.compare(&caf.finish());
        assert_eq!(comparison.result, -3);
    }
}

#[test]
fn generic_null_pointer_behavior_matches_in_subprocesses() {
    if let Ok(library) = std::env::var("IMA_NULL_PROBE_LIBRARY") {
        run_null_probe(
            Path::new(&library),
            &std::env::var("IMA_NULL_PROBE_MODE").expect("probe mode"),
        );
        panic!("null probe unexpectedly returned");
    }

    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().expect("current test executable");
    for mode in ["data", "info"] {
        let c_status = Command::new(&executable)
            .args([
                "--exact",
                "generic_null_pointer_behavior_matches_in_subprocesses",
            ])
            .env("IMA_NULL_PROBE_LIBRARY", c_library_path())
            .env("IMA_NULL_PROBE_MODE", mode)
            .status()
            .expect("run C null probe");
        let rust_status = Command::new(&executable)
            .args([
                "--exact",
                "generic_null_pointer_behavior_matches_in_subprocesses",
            ])
            .env("IMA_NULL_PROBE_LIBRARY", rust_library_path())
            .env("IMA_NULL_PROBE_MODE", mode)
            .status()
            .expect("run Rust null probe");

        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "null {mode} process signal differs"
        );
        assert!(
            c_status.signal().is_some(),
            "C null {mode} probe did not terminate by signal: {c_status}"
        );
    }
}

fn run_null_probe(library_path: &Path, mode: &str) {
    let library = unsafe { Library::new(library_path) }.expect("load probe library");
    let parse: Symbol<'_, ParseFn> =
        unsafe { library.get(b"ima_parse\0") }.expect("load probe symbol");
    match mode {
        "data" => {
            let mut output = InfoStorage::initialized();
            unsafe { parse(output.info_ptr(), std::ptr::null()) };
        }
        "info" => {
            let values = CaseValues::generate(&mut Rng::new(42), 0);
            let mut caf = Caf::new(values.header_flags);
            caf.description(values.first_desc, 0);
            caf.packet_table(values.first_pakt, 0);
            caf.data(4, values.edit_count, &[], 0);
            let input = caf.finish();
            unsafe { parse(std::ptr::null_mut(), input.as_ptr().cast()) };
        }
        _ => panic!("unknown null probe mode"),
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libharvest-work-cstVVS.so")
        .canonicalize()
        .expect("C shared object; build c_src first")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let deps_dir = executable.parent().expect("test deps directory");
    let profile_dir = deps_dir.parent().expect("Cargo profile directory");
    let target_dir = profile_dir.parent().expect("Cargo target directory");
    for candidate in [
        profile_dir.join("libima_parse_lib.so"),
        deps_dir.join("libima_parse_lib.so"),
        target_dir.join("release/libima_parse_lib.so"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "Rust cdylib not found beside test executable under {}",
        profile_dir.display()
    );
}
