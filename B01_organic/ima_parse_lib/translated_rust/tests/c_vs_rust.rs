use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
struct ImaInfo {
    blocks: *const u8,
    size: u64,
    sample_rate: f64,
    frame_count: u64,
    channel_count: u32,
}

type ImaParseFunc = unsafe extern "C" fn(*mut ImaInfo, *const u8) -> i32;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libima_parse_lib.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/debug/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libima_parse_lib.so");
    p
}

/// Build a minimal valid CAF binary with desc, pakt, and data chunks.
/// Binary layout must match C struct layout (including padding).
fn build_caf_data() -> Vec<u8> {
    let mut buf = Vec::new();

    // caf_header (8 bytes): type(4) + version(2) + flags(2)
    buf.extend_from_slice(b"caff");
    buf.extend_from_slice(&1u16.to_be_bytes());
    buf.extend_from_slice(&0u16.to_be_bytes());

    // "desc" chunk: caf_chunk(16) + caf_audio_description(32)
    buf.extend_from_slice(b"desc");
    buf.extend_from_slice(&[0u8; 4]); // padding in caf_chunk
    buf.extend_from_slice(&32i64.to_be_bytes()); // chunk size
    buf.extend_from_slice(&44100.0f64.to_bits().to_be_bytes()); // sample_rate
    buf.extend_from_slice(b"ima4"); // format_id
    buf.extend_from_slice(&0u32.to_be_bytes()); // format_flags
    buf.extend_from_slice(&34u32.to_be_bytes()); // bytes_per_packet
    buf.extend_from_slice(&64u32.to_be_bytes()); // frames_per_packet
    buf.extend_from_slice(&2u32.to_be_bytes()); // channels_per_frame
    buf.extend_from_slice(&0u32.to_be_bytes()); // bits_per_channel

    // "pakt" chunk: caf_chunk(16) + caf_packet_table(24)
    buf.extend_from_slice(b"pakt");
    buf.extend_from_slice(&[0u8; 4]);
    buf.extend_from_slice(&24i64.to_be_bytes());
    buf.extend_from_slice(&100i64.to_be_bytes()); // packet_count
    buf.extend_from_slice(&6400i64.to_be_bytes()); // frame_count
    buf.extend_from_slice(&0i32.to_be_bytes()); // priming_frames
    buf.extend_from_slice(&0i32.to_be_bytes()); // remainder_frames

    // "data" chunk: caf_chunk(16) + caf_data(4) + 2 ima_blocks(34 each)
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&[0u8; 4]);
    let payload = 4 + 34 * 2; // caf_data + blocks
    buf.extend_from_slice(&(payload as i64).to_be_bytes());
    buf.extend_from_slice(&0u32.to_be_bytes()); // edit_count
    for i in 0..2u8 {
        buf.extend_from_slice(&(0x1234u16 + i as u16).to_ne_bytes());
        buf.extend_from_slice(&[i + 1; 32]);
    }

    buf
}

fn call_both(caf: &[u8]) -> (i32, ImaInfo, i32, ImaInfo) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let c_fn: Symbol<ImaParseFunc> = unsafe { c_lib.get(b"ima_parse").unwrap() };

    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let rust_fn: Symbol<ImaParseFunc> = unsafe { rust_lib.get(b"ima_parse").unwrap() };

    let mut c_info = ImaInfo { blocks: std::ptr::null(), size: 0, sample_rate: 0.0, frame_count: 0, channel_count: 0 };
    let mut r_info = ImaInfo { blocks: std::ptr::null(), size: 0, sample_rate: 0.0, frame_count: 0, channel_count: 0 };

    let c_ret = unsafe { c_fn(&mut c_info, caf.as_ptr()) };
    let r_ret = unsafe { rust_fn(&mut r_info, caf.as_ptr()) };

    (c_ret, c_info, r_ret, r_info)
}

#[test]
fn test_valid_caf() {
    let caf = build_caf_data();
    let (c_ret, c_info, r_ret, r_info) = call_both(&caf);

    assert_eq!(c_ret, r_ret, "return code: C={c_ret} Rust={r_ret}");
    assert_eq!(c_ret, 0, "expected success");
    assert_eq!(c_info.size, r_info.size, "size mismatch: C={} R={}", c_info.size, r_info.size);
    assert_eq!(c_info.sample_rate.to_bits(), r_info.sample_rate.to_bits(),
        "sample_rate mismatch: C={} R={}", c_info.sample_rate, r_info.sample_rate);
    assert_eq!(c_info.frame_count, r_info.frame_count, "frame_count mismatch");
    assert_eq!(c_info.channel_count, r_info.channel_count, "channel_count mismatch");

    let c_off = c_info.blocks as usize - caf.as_ptr() as usize;
    let r_off = r_info.blocks as usize - caf.as_ptr() as usize;
    assert_eq!(c_off, r_off, "blocks offset: C={c_off} R={r_off}");

    println!("OK: size={} sr={} fc={} cc={} blk_off={}",
        c_info.size, c_info.sample_rate, c_info.frame_count, c_info.channel_count, c_off);
}

#[test]
fn test_bad_magic() {
    let mut caf = build_caf_data();
    caf[0] = b'x';
    let (c_ret, _, r_ret, _) = call_both(&caf);
    assert_eq!(c_ret, r_ret, "bad magic: C={c_ret} R={r_ret}");
    assert_eq!(c_ret, -1);
}

#[test]
fn test_bad_version() {
    let mut caf = build_caf_data();
    caf[4..6].copy_from_slice(&99u16.to_be_bytes());
    let (c_ret, _, r_ret, _) = call_both(&caf);
    assert_eq!(c_ret, r_ret, "bad version: C={c_ret} R={r_ret}");
    assert_eq!(c_ret, -2);
}

#[test]
fn test_bad_format_id() {
    let mut caf = build_caf_data();
    // format_id at: header(8) + chunk_hdr(16) + sample_rate(8) = 32
    caf[32..36].copy_from_slice(b"xxxx");
    let (c_ret, _, r_ret, _) = call_both(&caf);
    assert_eq!(c_ret, r_ret, "bad format: C={c_ret} R={r_ret}");
    assert_eq!(c_ret, -3);
}
