use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone)]
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

type ImaParseFunc = unsafe extern "C" fn(*mut ImaInfo, *const std::ffi::c_void) -> i32;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libima_parse_lib.so")
}

/// Write a caf_chunk header: type(u32) + 4 bytes padding + size(i64) = 16 bytes
fn write_chunk_header(buf: &mut Vec<u8>, chunk_type: &[u8; 4], size: i64) {
    buf.extend_from_slice(chunk_type);
    buf.extend_from_slice(&[0u8; 4]); // padding between u32 type and i64 size
    buf.extend_from_slice(&size.to_be_bytes());
}

/// Build binary data matching the struct layout the C code reads via pointer casts.
/// caf_header(8) then chunks: each chunk = caf_chunk(16) + payload.
fn build_caf_data(sample_rate: f64, channels: u32, frame_count: i64, block_data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();

    // caf_header: type(u32 BE "caff") + version(u16 BE 1) + flags(u16 BE 0) = 8 bytes
    buf.extend_from_slice(b"caff");
    buf.extend_from_slice(&1u16.to_be_bytes());
    buf.extend_from_slice(&0u16.to_be_bytes());

    // "desc" chunk header (16 bytes) + CafAudioDescription payload (32 bytes)
    write_chunk_header(&mut buf, b"desc", 32);
    buf.extend_from_slice(&sample_rate.to_be_bytes());
    buf.extend_from_slice(b"ima4");
    buf.extend_from_slice(&0u32.to_be_bytes()); // format_flags
    buf.extend_from_slice(&34u32.to_be_bytes()); // bytes_per_packet
    buf.extend_from_slice(&64u32.to_be_bytes()); // frames_per_packet
    buf.extend_from_slice(&channels.to_be_bytes());
    buf.extend_from_slice(&16u32.to_be_bytes()); // bits_per_channel

    // "pakt" chunk header (16 bytes) + CafPacketTable payload (24 bytes)
    write_chunk_header(&mut buf, b"pakt", 24);
    buf.extend_from_slice(&100i64.to_be_bytes()); // packet_count
    buf.extend_from_slice(&frame_count.to_be_bytes());
    buf.extend_from_slice(&0i32.to_be_bytes()); // priming_frames
    buf.extend_from_slice(&0i32.to_be_bytes()); // remainder_frames

    // "data" chunk header (16 bytes) + CafData(4) + block_data
    let data_payload_size = 4 + block_data.len();
    write_chunk_header(&mut buf, b"data", data_payload_size as i64);
    buf.extend_from_slice(&0u32.to_be_bytes()); // edit_count
    buf.extend_from_slice(block_data);

    buf
}

fn compare_ima_parse(caf_data: &[u8], expect_rc: i32) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        let c_fn: Symbol<ImaParseFunc> = c_lib.get(b"ima_parse").expect("C ima_parse");
        let r_fn: Symbol<ImaParseFunc> = rust_lib.get(b"ima_parse").expect("Rust ima_parse");

        let mut c_info = std::mem::zeroed::<ImaInfo>();
        let mut r_info = std::mem::zeroed::<ImaInfo>();

        let c_rc = c_fn(&mut c_info, caf_data.as_ptr() as *const _);
        let r_rc = r_fn(&mut r_info, caf_data.as_ptr() as *const _);

        assert_eq!(c_rc, expect_rc, "C rc={c_rc} expected={expect_rc}");
        assert_eq!(c_rc, r_rc, "C rc={c_rc} vs Rust rc={r_rc}");

        if c_rc == 0 {
            assert_eq!(c_info.size, r_info.size, "size mismatch");
            assert_eq!(
                c_info.sample_rate.to_bits(), r_info.sample_rate.to_bits(),
                "sample_rate mismatch: C={} Rust={}", c_info.sample_rate, r_info.sample_rate
            );
            assert_eq!(c_info.frame_count, r_info.frame_count, "frame_count mismatch");
            assert_eq!(c_info.channel_count, r_info.channel_count, "channel_count mismatch");
            let c_off = c_info.blocks as usize - caf_data.as_ptr() as usize;
            let r_off = r_info.blocks as usize - caf_data.as_ptr() as usize;
            assert_eq!(c_off, r_off, "blocks pointer offset mismatch");
        }
    }
}

#[test]
fn test_valid_caf_basic() {
    let data = build_caf_data(44100.0, 2, 6400, &[0xABu8; 34]);
    compare_ima_parse(&data, 0);
}

#[test]
fn test_valid_caf_48k_mono() {
    let data = build_caf_data(48000.0, 1, 12800, &[0x00u8; 34]);
    compare_ima_parse(&data, 0);
}

#[test]
fn test_valid_caf_large_frame_count() {
    let data = build_caf_data(96000.0, 6, 0x7FFFFFFFFFFFFFFF, &[0x55u8; 34 * 4]);
    compare_ima_parse(&data, 0);
}

#[test]
fn test_bad_magic() {
    let mut data = build_caf_data(44100.0, 2, 6400, &[0u8; 34]);
    data[0] = b'x';
    compare_ima_parse(&data, -1);
}

#[test]
fn test_bad_version() {
    let mut data = build_caf_data(44100.0, 2, 6400, &[0u8; 34]);
    data[4..6].copy_from_slice(&2u16.to_be_bytes());
    compare_ima_parse(&data, -2);
}

#[test]
fn test_bad_format_id() {
    let mut data = build_caf_data(44100.0, 2, 6400, &[0u8; 34]);
    // desc payload starts at offset 8(header) + 16(chunk header) = 24
    // format_id is at +8 within desc payload (after sample_rate f64)
    let fmt_offset = 8 + 16 + 8;
    data[fmt_offset..fmt_offset + 4].copy_from_slice(b"aac ");
    compare_ima_parse(&data, -3);
}

#[test]
fn test_zero_sample_rate() {
    let data = build_caf_data(0.0, 1, 100, &[0u8; 34]);
    compare_ima_parse(&data, 0);
}

#[test]
fn test_multiple_blocks() {
    let data = build_caf_data(22050.0, 4, 640000, &[0xCDu8; 34 * 10]);
    compare_ima_parse(&data, 0);
}
