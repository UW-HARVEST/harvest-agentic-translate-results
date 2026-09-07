use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs;
use std::mem::{MaybeUninit, size_of};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[repr(C)]
#[derive(Clone, Copy)]
struct ImaBlock {
    preamble: u16,
    data: [u8; 32],
}

#[repr(C)]
struct ImaInfo {
    blocks: *const ImaBlock,
    size: u64,
    sample_rate: f64,
    frame_count: u64,
    channel_count: u32,
}

type Parse = unsafe extern "C" fn(*mut ImaInfo, *const c_void) -> c_int;

struct Loaded {
    _library: Library,
    parse: Parse,
}

impl Loaded {
    fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let parse = unsafe {
            *library
                .get::<Parse>(b"ima_parse\0")
                .unwrap_or_else(|error| panic!("missing ima_parse in {}: {error}", path.display()))
        };
        Self {
            _library: library,
            parse,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<_> = fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", build.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}",
        build.display()
    );
    candidates.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("IMA_RUST_SO") {
        return PathBuf::from(path);
    }
    manifest_dir().join("target/release/libima_parse_lib.so")
}

fn libraries() -> (Loaded, Loaded) {
    (
        Loaded::open(&c_library_path()),
        Loaded::open(&rust_library_path()),
    )
}

unsafe fn call_bytes(parse: Parse, info: *mut ImaInfo, data: *const u8) -> (i32, Vec<u8>) {
    let result = unsafe { parse(info, data.cast()) };
    let bytes =
        unsafe { std::slice::from_raw_parts(info.cast::<u8>(), size_of::<ImaInfo>()).to_vec() };
    (result, bytes)
}

fn compare_case(c: &Loaded, rust: &Loaded, bytes: &[u8], offset: usize, expected: i32) {
    assert!(offset < bytes.len());
    let data = unsafe { bytes.as_ptr().add(offset) };
    let mut c_info = MaybeUninit::<ImaInfo>::uninit();
    let mut rust_info = MaybeUninit::<ImaInfo>::uninit();
    unsafe {
        c_info
            .as_mut_ptr()
            .cast::<u8>()
            .write_bytes(0xa5, size_of::<ImaInfo>());
        rust_info
            .as_mut_ptr()
            .cast::<u8>()
            .write_bytes(0xa5, size_of::<ImaInfo>());
    }
    let c_result = unsafe { call_bytes(c.parse, c_info.as_mut_ptr(), data) };
    let rust_result = unsafe { call_bytes(rust.parse, rust_info.as_mut_ptr(), data) };
    assert_eq!(c_result.0, expected, "C returned an unexpected status");
    assert_eq!(rust_result.0, c_result.0, "return-code divergence");
    assert_eq!(rust_result.1, c_result.1, "ima_info byte divergence");
}

#[derive(Clone, Copy)]
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

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

#[derive(Clone)]
struct Meta {
    sample_bits: u64,
    format: [u8; 4],
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    channels: u32,
    bits_per_channel: u32,
    packet_count: i64,
    frame_count: i64,
    priming_frames: i32,
    remainder_frames: i32,
    edit_count: u32,
}

fn random_meta(rng: &mut Rng) -> Meta {
    Meta {
        sample_bits: 44_100.0f64.to_bits(),
        format: *b"ima4",
        format_flags: rng.next_u32(),
        bytes_per_packet: rng.next_u32(),
        frames_per_packet: rng.next_u32(),
        channels: rng.next_u32(),
        bits_per_channel: rng.next_u32(),
        packet_count: rng.next_u64() as i64,
        frame_count: rng.next_u64() as i64,
        priming_frames: rng.next_u32() as i32,
        remainder_frames: rng.next_u32() as i32,
        edit_count: rng.next_u32(),
    }
}

fn header(flags: u16) -> Vec<u8> {
    let mut bytes = Vec::from(*b"caff");
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&flags.to_be_bytes());
    bytes
}

fn description(meta: &Meta) -> Vec<u8> {
    let mut payload = Vec::with_capacity(32);
    payload.extend_from_slice(&meta.sample_bits.to_be_bytes());
    payload.extend_from_slice(&meta.format);
    payload.extend_from_slice(&meta.format_flags.to_be_bytes());
    payload.extend_from_slice(&meta.bytes_per_packet.to_be_bytes());
    payload.extend_from_slice(&meta.frames_per_packet.to_be_bytes());
    payload.extend_from_slice(&meta.channels.to_be_bytes());
    payload.extend_from_slice(&meta.bits_per_channel.to_be_bytes());
    payload
}

fn packet_table(meta: &Meta) -> Vec<u8> {
    let mut payload = Vec::with_capacity(24);
    payload.extend_from_slice(&meta.packet_count.to_be_bytes());
    payload.extend_from_slice(&meta.frame_count.to_be_bytes());
    payload.extend_from_slice(&meta.priming_frames.to_be_bytes());
    payload.extend_from_slice(&meta.remainder_frames.to_be_bytes());
    payload
}

fn data_payload(meta: &Meta, rng: &mut Rng, block_count: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(4 + block_count * size_of::<ImaBlock>());
    payload.extend_from_slice(&meta.edit_count.to_be_bytes());
    let old_len = payload.len();
    payload.resize(old_len + block_count * size_of::<ImaBlock>(), 0);
    rng.fill(&mut payload[old_len..]);
    payload
}

fn push_chunk(output: &mut Vec<u8>, kind: [u8; 4], size: i64, payload: &[u8]) {
    output.extend_from_slice(&kind);
    output.extend_from_slice(&[0u8; 4]);
    output.extend_from_slice(&size.to_be_bytes());
    output.extend_from_slice(payload);
}

fn valid_case(row: usize, iteration: usize, rng: &mut Rng) -> (Vec<u8>, usize) {
    let mut meta = random_meta(rng);
    match row {
        11 => {
            const EDGE_BITS: [u64; 8] = [
                0,
                1u64 << 63,
                1,
                0x7fefffffffffffff,
                0x7ff0000000000000,
                0xfff0000000000000,
                0x7ff8000000000001,
                0xffffffffffffffff,
            ];
            meta.sample_bits = EDGE_BITS
                .get(iteration)
                .copied()
                .unwrap_or_else(|| rng.next_u64());
        }
        12 => {
            const EDGES: [i64; 5] = [i64::MIN, -1, 0, 1, i64::MAX];
            meta.frame_count = EDGES
                .get(iteration)
                .copied()
                .unwrap_or_else(|| rng.next_u64() as i64);
        }
        13 => {
            const EDGES: [u32; 6] = [0, 1, 2, 8, u32::MAX - 1, u32::MAX];
            meta.channels = EDGES
                .get(iteration)
                .copied()
                .unwrap_or_else(|| rng.next_u32());
        }
        _ => {}
    }

    let desc = description(&meta);
    let pakt = packet_table(&meta);
    let blocks = if row == 1 {
        iteration % 6
    } else {
        (rng.next_u32() as usize % 4) + 1
    };
    let data = data_payload(&meta, rng, blocks);
    let normal_data_size = data.len() as i64;
    let data_size = match row {
        8 => 0,
        9 => {
            if iteration == 0 {
                -1
            } else {
                (rng.next_u64() as i64) | i64::MIN
            }
        }
        10 => {
            if iteration % 2 == 0 {
                i64::MIN
            } else {
                i64::MAX
            }
        }
        _ => normal_data_size,
    };

    let offset = if row == 14 { iteration % 8 } else { 0 };
    let mut output = vec![0x5a; offset];
    output.extend_from_slice(&header(rng.next_u32() as u16));

    match row {
        2 => {
            push_chunk(&mut output, *b"pakt", pakt.len() as i64, &pakt);
            push_chunk(&mut output, *b"desc", desc.len() as i64, &desc);
        }
        3 => {
            push_chunk(&mut output, *b"junk", 0, &[]);
            push_chunk(&mut output, *b"desc", desc.len() as i64, &desc);
            push_chunk(&mut output, *b"pakt", pakt.len() as i64, &pakt);
        }
        4 => {
            push_chunk(&mut output, *b"desc", desc.len() as i64, &desc);
            let mut unknown = vec![0; ((rng.next_u32() as usize % 4) + 1) * 8];
            rng.fill(&mut unknown);
            push_chunk(&mut output, *b"free", unknown.len() as i64, &unknown);
            push_chunk(&mut output, *b"pakt", pakt.len() as i64, &pakt);
        }
        5 => {
            let mut old_meta = random_meta(rng);
            old_meta.format = *b"nope";
            let old_desc = description(&old_meta);
            push_chunk(&mut output, *b"desc", old_desc.len() as i64, &old_desc);
            push_chunk(&mut output, *b"desc", desc.len() as i64, &desc);
            push_chunk(&mut output, *b"pakt", pakt.len() as i64, &pakt);
        }
        6 => {
            let old_pakt = packet_table(&random_meta(rng));
            push_chunk(&mut output, *b"pakt", old_pakt.len() as i64, &old_pakt);
            push_chunk(&mut output, *b"desc", desc.len() as i64, &desc);
            push_chunk(&mut output, *b"pakt", pakt.len() as i64, &pakt);
        }
        _ => {
            push_chunk(&mut output, *b"desc", desc.len() as i64, &desc);
            push_chunk(&mut output, *b"pakt", pakt.len() as i64, &pakt);
        }
    }

    push_chunk(&mut output, *b"data", data_size, &data);
    if row == 7 {
        let mut trailing = vec![0; 16 + (rng.next_u32() as usize % 64)];
        rng.fill(&mut trailing);
        output.extend_from_slice(&trailing);
    }
    (output, offset)
}

fn run_config_row(row: usize) {
    let (c, rust) = libraries();
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173 ^ row as u64);
    for iteration in 0..128 {
        let (bytes, offset) = valid_case(row, iteration, &mut rng);
        compare_case(&c, &rust, &bytes, offset, 0);
    }
}

macro_rules! config_test {
    ($name:ident, $row:expr) => {
        #[test]
        fn $name() {
            run_config_row($row);
        }
    };
}

config_test!(config_01_canonical, 1);
config_test!(config_02_reversed_metadata, 2);
config_test!(config_03_zero_unknown_chunk, 3);
config_test!(config_04_nonempty_unknown_chunk, 4);
config_test!(config_05_duplicate_description, 5);
config_test!(config_06_duplicate_packet_table, 6);
config_test!(config_07_data_terminates, 7);
config_test!(config_08_zero_data_size, 8);
config_test!(config_09_negative_data_size, 9);
config_test!(config_10_data_size_boundaries, 10);
config_test!(config_11_sample_rate_bit_patterns, 11);
config_test!(config_12_frame_count_boundaries, 12);
config_test!(config_13_channel_count_boundaries, 13);
config_test!(config_14_input_alignment, 14);

fn invalid_case(row: usize, iteration: usize, rng: &mut Rng) -> Vec<u8> {
    let mut meta = random_meta(rng);
    if row == 3 {
        let mut format = rng.next_u32().to_be_bytes();
        if format == *b"ima4" {
            format[0] ^= 1;
        }
        meta.format = format;
    }
    let mut output = header(rng.next_u32() as u16);
    if row == 1 {
        output[iteration % 4] ^= 1;
        return output;
    }
    if row == 2 {
        let mut version = rng.next_u32() as u16;
        if version == 1 {
            version = 2;
        }
        output[4..6].copy_from_slice(&version.to_be_bytes());
        return output;
    }
    let desc = description(&meta);
    let pakt = packet_table(&meta);
    let data = data_payload(&meta, rng, iteration % 4);
    push_chunk(&mut output, *b"desc", desc.len() as i64, &desc);
    push_chunk(&mut output, *b"pakt", pakt.len() as i64, &pakt);
    push_chunk(&mut output, *b"data", data.len() as i64, &data);
    output
}

fn run_error_row(row: usize, expected: i32) {
    let (c, rust) = libraries();
    let mut rng = Rng::new(0xa076_1d64_78bd_642f ^ row as u64);
    for iteration in 0..128 {
        let bytes = invalid_case(row, iteration, &mut rng);
        compare_case(&c, &rust, &bytes, 0, expected);
    }
}

#[test]
fn error_01_bad_file_type() {
    run_error_row(1, -1);
}

#[test]
fn error_02_bad_version() {
    run_error_row(2, -2);
}

#[test]
fn error_03_bad_format() {
    run_error_row(3, -3);
}

fn run_crash_child(library: &str, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("abi_crash_helper")
        .arg("--nocapture")
        .env("IMA_CRASH_LIBRARY", library)
        .env("IMA_CRASH_CASE", case)
        .status()
        .unwrap()
}

#[cfg(unix)]
fn assert_same_crash(case: &str) {
    use std::os::unix::process::ExitStatusExt;
    let c = run_crash_child("c", case);
    let rust = run_crash_child("rust", case);
    assert!(!c.success(), "C unexpectedly survived {case}");
    assert!(!rust.success(), "Rust unexpectedly survived {case}");
    assert_eq!(rust.code(), c.code(), "exit-code mismatch for {case}");
    assert_eq!(rust.signal(), c.signal(), "signal mismatch for {case}");
}

#[test]
fn abi_null_data_matches_process_rejection() {
    assert_same_crash("null_data");
}

#[test]
fn abi_null_info_matches_process_rejection() {
    assert_same_crash("null_info");
}

#[test]
fn abi_crash_helper() {
    let Some(library_name) = std::env::var_os("IMA_CRASH_LIBRARY") else {
        return;
    };
    let case = std::env::var("IMA_CRASH_CASE").unwrap();
    let path = if library_name == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let library = Loaded::open(&path);
    match case.as_str() {
        "null_data" => {
            let mut info = MaybeUninit::<ImaInfo>::uninit();
            unsafe {
                (library.parse)(info.as_mut_ptr(), std::ptr::null());
            }
        }
        "null_info" => {
            let mut rng = Rng::new(1);
            let (bytes, offset) = valid_case(1, 0, &mut rng);
            unsafe {
                (library.parse)(
                    std::ptr::null_mut(),
                    bytes.as_ptr().add(offset).cast::<c_void>(),
                );
            }
        }
        other => panic!("unknown crash case {other}"),
    }
}
