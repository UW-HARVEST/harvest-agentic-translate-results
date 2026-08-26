use libloading::Library;
use std::ffi::{c_int, c_void};
use std::mem::{self, MaybeUninit};
use std::path::PathBuf;
use std::process::Command;
use std::ptr;

#[repr(C)]
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

type ImaParse = unsafe extern "C" fn(*mut ImaInfo, *const c_void) -> c_int;

struct Libraries {
    _c: Library,
    _rust: Library,
    c_parse: ImaParse,
    rust_parse: ImaParse,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_parse = *c
                .get::<ImaParse>(b"ima_parse\0")
                .expect("C ima_parse export");
            let rust_parse = *rust
                .get::<ImaParse>(b"ima_parse\0")
                .expect("Rust ima_parse export");
            Self {
                _c: c,
                _rust: rust,
                c_parse,
                rust_parse,
            }
        }
    }
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    executable
        .parent()
        .expect("target profile dependency directory")
        .join("libima_parse_lib.so")
}

struct Output {
    value: MaybeUninit<ImaInfo>,
}

impl Output {
    fn sentinel() -> Self {
        let mut value = MaybeUninit::<ImaInfo>::zeroed();
        unsafe {
            let output = value.as_mut_ptr();
            ptr::addr_of_mut!((*output).blocks).write(usize::MAX as *const ImaBlock);
            ptr::addr_of_mut!((*output).size).write(0x1122_3344_5566_7788);
            ptr::addr_of_mut!((*output).sample_rate).write(f64::from_bits(0x7ff8_1234_5678_9abc));
            ptr::addr_of_mut!((*output).frame_count).write(0x8877_6655_4433_2211);
            ptr::addr_of_mut!((*output).channel_count).write(0xa5a5_5a5a);
        }
        Self { value }
    }

    fn as_mut_ptr(&mut self) -> *mut ImaInfo {
        self.value.as_mut_ptr()
    }

    fn bytes(&self) -> Vec<u8> {
        unsafe {
            std::slice::from_raw_parts(self.value.as_ptr().cast::<u8>(), mem::size_of::<ImaInfo>())
                .to_vec()
        }
    }

    fn blocks(&self) -> *const ImaBlock {
        unsafe { ptr::addr_of!((*self.value.as_ptr()).blocks).read() }
    }
}

struct AlignedInput {
    words: Vec<u64>,
    len: usize,
}

impl AlignedInput {
    fn new(bytes: &[u8]) -> Self {
        let mut words = vec![0_u64; bytes.len().div_ceil(mem::size_of::<u64>())];
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), words.as_mut_ptr().cast::<u8>(), bytes.len());
        }
        Self {
            words,
            len: bytes.len(),
        }
    }

    fn as_ptr(&self) -> *const c_void {
        self.words.as_ptr().cast()
    }

    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.words.as_ptr().cast::<u8>(), self.len) }
    }
}

#[derive(Clone)]
struct Description {
    sample_bits: u64,
    format_id: [u8; 4],
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
}

impl Description {
    fn valid(sample_bits: u64, channels: u32) -> Self {
        Self {
            sample_bits,
            format_id: *b"ima4",
            format_flags: 0,
            bytes_per_packet: 34,
            frames_per_packet: 64,
            channels_per_frame: channels,
            bits_per_channel: 16,
        }
    }
}

#[derive(Clone)]
struct PacketTable {
    packet_count: i64,
    frame_count: i64,
    priming_frames: i32,
    remainder_frames: i32,
}

impl PacketTable {
    fn with_frame_count(frame_count: i64) -> Self {
        Self {
            packet_count: 1,
            frame_count,
            priming_frames: 0,
            remainder_frames: 0,
        }
    }
}

struct CafBuilder {
    bytes: Vec<u8>,
    blocks_offset: Option<usize>,
}

impl CafBuilder {
    fn new(flags: u16) -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"caff");
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&flags.to_be_bytes());
        Self {
            bytes,
            blocks_offset: None,
        }
    }

    fn raw_header(kind: [u8; 4], version: u16, flags: u16) -> AlignedInput {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&kind);
        bytes.extend_from_slice(&version.to_be_bytes());
        bytes.extend_from_slice(&flags.to_be_bytes());
        AlignedInput::new(&bytes)
    }

    fn chunk(&mut self, kind: [u8; 4], size: i64, payload: &[u8]) {
        assert_eq!(self.bytes.len() % 8, 0, "chunk must be 8-byte aligned");
        self.bytes.extend_from_slice(&kind);
        self.bytes.extend_from_slice(&[0; 4]);
        self.bytes.extend_from_slice(&size.to_be_bytes());
        self.bytes.extend_from_slice(payload);
    }

    fn description(&mut self, description: &Description) {
        let mut payload = Vec::with_capacity(32);
        payload.extend_from_slice(&description.sample_bits.to_be_bytes());
        payload.extend_from_slice(&description.format_id);
        payload.extend_from_slice(&description.format_flags.to_be_bytes());
        payload.extend_from_slice(&description.bytes_per_packet.to_be_bytes());
        payload.extend_from_slice(&description.frames_per_packet.to_be_bytes());
        payload.extend_from_slice(&description.channels_per_frame.to_be_bytes());
        payload.extend_from_slice(&description.bits_per_channel.to_be_bytes());
        assert_eq!(payload.len(), 32);
        self.chunk(*b"desc", 32, &payload);
    }

    fn packet_table(&mut self, packet_table: &PacketTable) {
        let mut payload = Vec::with_capacity(24);
        payload.extend_from_slice(&packet_table.packet_count.to_be_bytes());
        payload.extend_from_slice(&packet_table.frame_count.to_be_bytes());
        payload.extend_from_slice(&packet_table.priming_frames.to_be_bytes());
        payload.extend_from_slice(&packet_table.remainder_frames.to_be_bytes());
        assert_eq!(payload.len(), 24);
        self.chunk(*b"pakt", 24, &payload);
    }

    fn unknown(&mut self, payload: &[u8]) {
        assert_eq!(payload.len() % 8, 0);
        self.chunk(*b"free", payload.len() as i64, payload);
    }

    fn data(&mut self, declared_size: i64, edit_count: u32, block_bytes: &[u8]) {
        let mut payload = Vec::with_capacity(4 + block_bytes.len());
        payload.extend_from_slice(&edit_count.to_be_bytes());
        payload.extend_from_slice(block_bytes);
        self.blocks_offset = Some(self.bytes.len() + 16 + 4);
        self.chunk(*b"data", declared_size, &payload);
    }

    fn finish(self) -> (AlignedInput, usize) {
        let blocks_offset = self.blocks_offset.expect("data chunk");
        (AlignedInput::new(&self.bytes), blocks_offset)
    }
}

#[derive(Debug)]
struct Outcome {
    code: c_int,
    bytes: Vec<u8>,
    blocks: usize,
}

unsafe fn invoke(parse: ImaParse, input: &AlignedInput) -> Outcome {
    let mut output = Output::sentinel();
    let code = unsafe { parse(output.as_mut_ptr(), input.as_ptr()) };
    Outcome {
        code,
        bytes: output.bytes(),
        blocks: output.blocks() as usize,
    }
}

fn assert_valid_case(
    libraries: &Libraries,
    input: &AlignedInput,
    blocks_offset: usize,
    inspected_block_bytes: usize,
) {
    let c = unsafe { invoke(libraries.c_parse, input) };
    let rust = unsafe { invoke(libraries.rust_parse, input) };
    assert_eq!(c.code, 0, "C rejected generated valid input");
    assert_eq!(rust.code, c.code);
    assert_eq!(rust.bytes, c.bytes, "ima_info bytes differ");
    assert_eq!(c.blocks, input.as_ptr() as usize + blocks_offset);

    if inspected_block_bytes != 0 {
        let start = c.blocks - input.as_ptr() as usize;
        let end = start + inspected_block_bytes;
        assert_eq!(
            &input.bytes()[start..end],
            &input.bytes()[blocks_offset..blocks_offset + inspected_block_bytes]
        );
    }
}

fn assert_error_case(libraries: &Libraries, input: &AlignedInput, expected: c_int) {
    let sentinel = Output::sentinel().bytes();
    let c = unsafe { invoke(libraries.c_parse, input) };
    let rust = unsafe { invoke(libraries.rust_parse, input) };
    assert_eq!(c.code, expected);
    assert_eq!(rust.code, expected);
    assert_eq!(c.bytes, sentinel, "C modified info on error");
    assert_eq!(rust.bytes, sentinel, "Rust modified info on error");
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

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64() as u8).collect()
    }

    fn ordinary_sample_bits(&mut self) -> u64 {
        ((self.next_u64() % 10_000_000) as f64 / 8.0).to_bits()
    }
}

fn ordinary_case(rng: &mut Rng) -> (Description, PacketTable, i64, Vec<u8>) {
    (
        Description::valid(rng.ordinary_sample_bits(), rng.next_u32()),
        PacketTable::with_frame_count((rng.next_u64() & i64::MAX as u64) as i64),
        (rng.next_u64() & i64::MAX as u64) as i64,
        rng.bytes(68),
    )
}

#[test]
fn config_01_minimal_desc_pakt_data() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0101_cafe_f00d_beef);
    for _ in 0..64 {
        let (description, packet_table, data_size, blocks) = ordinary_case(&mut rng);
        let mut builder = CafBuilder::new(rng.next_u32() as u16);
        builder.description(&description);
        builder.packet_table(&packet_table);
        builder.data(data_size, rng.next_u32(), &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_02_reversed_metadata_order() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0202_cafe_f00d_beef);
    for _ in 0..64 {
        let (description, packet_table, data_size, blocks) = ordinary_case(&mut rng);
        let mut builder = CafBuilder::new(0);
        builder.packet_table(&packet_table);
        builder.description(&description);
        builder.data(data_size, 0, &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_03_unknown_zero_payload_before_metadata() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0303_cafe_f00d_beef);
    for _ in 0..64 {
        let (description, packet_table, data_size, blocks) = ordinary_case(&mut rng);
        let mut builder = CafBuilder::new(0);
        builder.unknown(&[]);
        builder.description(&description);
        builder.packet_table(&packet_table);
        builder.data(data_size, 0, &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_04_unknown_positive_payload_between_metadata() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0404_cafe_f00d_beef);
    for iteration in 0..64 {
        let (description, packet_table, data_size, blocks) = ordinary_case(&mut rng);
        let unknown = rng.bytes([8, 16, 64, 256][iteration % 4]);
        let mut builder = CafBuilder::new(0);
        builder.description(&description);
        builder.unknown(&unknown);
        builder.packet_table(&packet_table);
        builder.data(data_size, 0, &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_05_unknown_after_metadata() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0505_cafe_f00d_beef);
    for _ in 0..64 {
        let (description, packet_table, data_size, blocks) = ordinary_case(&mut rng);
        let unknown = rng.bytes(24);
        let mut builder = CafBuilder::new(0);
        builder.description(&description);
        builder.packet_table(&packet_table);
        builder.unknown(&unknown);
        builder.data(data_size, 0, &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_06_repeated_description_last_wins() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0606_cafe_f00d_beef);
    for _ in 0..64 {
        let first = Description::valid(rng.ordinary_sample_bits(), rng.next_u32());
        let (last, packet_table, data_size, blocks) = ordinary_case(&mut rng);
        let mut builder = CafBuilder::new(0);
        builder.description(&first);
        builder.packet_table(&packet_table);
        builder.description(&last);
        builder.data(data_size, 0, &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_07_repeated_packet_table_last_wins() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0707_cafe_f00d_beef);
    for _ in 0..64 {
        let (description, last, data_size, blocks) = ordinary_case(&mut rng);
        let first = PacketTable::with_frame_count(rng.next_u64() as i64);
        let mut builder = CafBuilder::new(0);
        builder.packet_table(&first);
        builder.description(&description);
        builder.packet_table(&last);
        builder.data(data_size, 0, &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_08_ignored_fields_vary() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0808_cafe_f00d_beef);
    for _ in 0..64 {
        let mut description = Description::valid(rng.ordinary_sample_bits(), rng.next_u32());
        description.format_flags = rng.next_u32();
        description.bytes_per_packet = rng.next_u32();
        description.frames_per_packet = rng.next_u32();
        description.bits_per_channel = rng.next_u32();
        let packet_table = PacketTable {
            packet_count: rng.next_u64() as i64,
            frame_count: rng.next_u64() as i64,
            priming_frames: rng.next_u32() as i32,
            remainder_frames: rng.next_u32() as i32,
        };
        let blocks = rng.bytes(68);
        let mut builder = CafBuilder::new(rng.next_u32() as u16);
        builder.description(&description);
        builder.packet_table(&packet_table);
        builder.data(rng.next_u64() as i64, rng.next_u32(), &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_09_data_size_signed_boundaries() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0909_cafe_f00d_beef);
    let boundaries = [0, 1, i64::MAX, -1, i64::MIN];
    for iteration in 0..80 {
        let description = Description::valid(rng.ordinary_sample_bits(), rng.next_u32());
        let packet_table = PacketTable::with_frame_count(rng.next_u64() as i64);
        let size = if iteration < boundaries.len() {
            boundaries[iteration]
        } else {
            rng.next_u64() as i64
        };
        let blocks = rng.bytes(34);
        let mut builder = CafBuilder::new(0);
        builder.description(&description);
        builder.packet_table(&packet_table);
        builder.data(size, 0, &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_10_numeric_boundaries() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x1010_cafe_f00d_beef);
    let host_sample_bits = [
        0.0_f64.to_bits(),
        (-0.0_f64).to_bits(),
        1_u64,
        (1.5_f64).to_bits(),
        (-1.5_f64).to_bits(),
        f64::MAX.to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        f64::NAN.to_bits(),
    ];
    let channels = [0, 1, u32::MAX - 1, u32::MAX];
    let frames = [0, 1, i64::MAX, -1, i64::MIN];
    for iteration in 0..96 {
        let description = Description::valid(
            host_sample_bits[iteration % host_sample_bits.len()].swap_bytes(),
            channels[iteration % channels.len()],
        );
        let packet_table = PacketTable::with_frame_count(frames[iteration % frames.len()]);
        let blocks = rng.bytes(34);
        let mut builder = CafBuilder::new(0);
        builder.description(&description);
        builder.packet_table(&packet_table);
        builder.data(rng.next_u64() as i64, 0, &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn config_11_data_payload_shapes() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x1111_cafe_f00d_beef);
    let lengths = [0, 1, 33, 34, 68, 102];
    for iteration in 0..72 {
        let description = Description::valid(rng.ordinary_sample_bits(), rng.next_u32());
        let packet_table = PacketTable::with_frame_count(rng.next_u64() as i64);
        let blocks = rng.bytes(lengths[iteration % lengths.len()]);
        let mut builder = CafBuilder::new(0);
        builder.description(&description);
        builder.packet_table(&packet_table);
        builder.data(blocks.len() as i64, rng.next_u32(), &blocks);
        let (input, offset) = builder.finish();
        assert_valid_case(&libraries, &input, offset, blocks.len());
    }
}

#[test]
fn error_01_invalid_file_type() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xe101_cafe_f00d_beef);
    for _ in 0..64 {
        let mut kind = rng.next_u32().to_be_bytes();
        if kind == *b"caff" {
            kind[0] ^= 1;
        }
        let input = CafBuilder::raw_header(kind, rng.next_u32() as u16, 0);
        assert_error_case(&libraries, &input, -1);
    }
}

#[test]
fn error_02_invalid_version() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xe202_cafe_f00d_beef);
    for iteration in 0..64 {
        let mut version = if iteration < 3 {
            [0, 2, u16::MAX][iteration]
        } else {
            rng.next_u32() as u16
        };
        if version == 1 {
            version = 2;
        }
        let input = CafBuilder::raw_header(*b"caff", version, rng.next_u32() as u16);
        assert_error_case(&libraries, &input, -2);
    }
}

#[test]
fn error_03_invalid_format_id() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xe303_cafe_f00d_beef);
    for _ in 0..64 {
        let mut description = Description::valid(rng.ordinary_sample_bits(), 2);
        description.format_id = rng.next_u32().to_be_bytes();
        if description.format_id == *b"ima4" {
            description.format_id[0] ^= 1;
        }
        let packet_table = PacketTable::with_frame_count(64);
        let mut builder = CafBuilder::new(0);
        builder.description(&description);
        builder.packet_table(&packet_table);
        builder.data(34, 0, &[]);
        let (input, _) = builder.finish();
        assert_error_case(&libraries, &input, -3);
    }
}

#[test]
fn null_pointer_probe() {
    let Ok(probe) = std::env::var("IMA_NULL_PROBE") else {
        return;
    };
    let libraries = Libraries::load();
    let (implementation, pointer) = probe.split_once(':').expect("probe form");
    let parse = match implementation {
        "c" => libraries.c_parse,
        "rust" => libraries.rust_parse,
        _ => panic!("unknown implementation"),
    };

    if pointer == "data" {
        let mut output = Output::sentinel();
        unsafe {
            parse(output.as_mut_ptr(), ptr::null());
        }
        return;
    }

    let description = Description::valid(44_100.0_f64.to_bits(), 2);
    let packet_table = PacketTable::with_frame_count(64);
    let mut builder = CafBuilder::new(0);
    if pointer != "description" {
        builder.description(&description);
    }
    if pointer != "packet" {
        builder.packet_table(&packet_table);
    }
    builder.data(34, 0, &[]);
    let (input, _) = builder.finish();

    unsafe {
        if pointer == "info" {
            parse(ptr::null_mut(), input.as_ptr());
        } else {
            let mut output = Output::sentinel();
            parse(output.as_mut_ptr(), input.as_ptr());
        }
    }
}

#[test]
#[cfg(unix)]
fn generic_null_pointer_boundaries_match_process_failure() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().expect("current test executable");
    for pointer in ["data", "info", "description", "packet"] {
        let run = |implementation: &str| {
            Command::new(&executable)
                .args(["--exact", "null_pointer_probe", "--nocapture"])
                .env("IMA_NULL_PROBE", format!("{implementation}:{pointer}"))
                .status()
                .expect("run null probe")
        };
        let c = run("c");
        let rust = run("rust");
        assert!(!c.success(), "C unexpectedly accepted null {pointer}");
        assert!(!rust.success(), "Rust unexpectedly accepted null {pointer}");
        assert_eq!(
            (rust.code(), rust.signal()),
            (c.code(), c.signal()),
            "process failure differs for null {pointer}"
        );
    }
}
