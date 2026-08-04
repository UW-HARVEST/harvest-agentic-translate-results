use libtinyfseq::tinyfseq::{
    tf_channel_range_read, tf_compression_block_read, tf_header_read, tf_var_header_read,
    TFChannelRange, TFCompressionBlock, TFCompressionType, TFError, TFHeader, TFVarHeader,
};

fn make_header() -> TFHeader {
    TFHeader {
        channel_data_offset: 0,
        minor_version: 0,
        major_version: 0,
        variable_data_offset: 0,
        channel_count: 0,
        frame_count: 0,
        frame_step_time_millis: 0,
        compression_type: TFCompressionType::TF_COMPRESSION_NONE,
        compression_block_count: 0,
        channel_range_count: 0,
        sequence_uid: 0,
    }
}

fn make_block() -> TFCompressionBlock {
    TFCompressionBlock {
        first_frame_id: 0,
        size: 0,
    }
}

fn make_var() -> TFVarHeader {
    TFVarHeader { size: 0, id: [0, 0] }
}

fn make_range() -> TFChannelRange {
    TFChannelRange {
        first_channel_number: 0,
        channel_count: 0,
    }
}

#[test]
fn test_error_strings() {
    assert_eq!(TFError::TF_OK.to_string(), "TF_OK (ok)");
    assert_eq!(
        TFError::TF_EINVALID_MAGIC.to_string(),
        "TF_EINVALID_MAGIC (invalid magic file signature)"
    );
    assert_eq!(
        TFError::TF_EINVALID_COMPRESSION_TYPE.to_string(),
        "TF_EINVALID_COMPRESSION_TYPE (unknown compression identifier)"
    );
    assert_eq!(
        TFError::TF_EINVALID_BUFFER_SIZE.to_string(),
        "TF_EINVALID_BUFFER_SIZE (undersized data decoding buffer argument)"
    );
    assert_eq!(
        TFError::TF_EINVALID_VAR_SIZE.to_string(),
        "TF_EINVALID_VAR_SIZE (invalid variable size in header)"
    );
}

#[test]
fn test_header_read_valid() {
    let buf: [u8; 40] = [
        b'P', b'S', b'E', b'Q',
        0x10, 0x00,
        0x02,
        0x02,
        0x20, 0x00,
        0x10, 0x00, 0x00, 0x00,
        0x05, 0x00, 0x00, 0x00,
        0x32,
        0x00,
        0x21, // compression byte: lower 4 bits = 1 (ZSTD), upper = 2 (ignored)
        0x03,
        0x02,
        0x00,
        0x78, 0x56, 0x34, 0x12, 0xAB, 0xCD, 0xEF, 0x01,
        0xDE, 0xAD, 0xBE, 0xEF,
        0x00, 0x00, 0x00, 0x00,
    ];
    let mut h = make_header();
    let mut ep_slice: &[u8] = &[];
    let err = tf_header_read(&buf, &mut h, Some(&mut ep_slice));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(h.channel_data_offset, 16u16);
    assert_eq!(h.minor_version, 2u8);
    assert_eq!(h.major_version, 2u8);
    assert_eq!(h.variable_data_offset, 32u16);
    assert_eq!(h.channel_count, 16u32);
    assert_eq!(h.frame_count, 5u32);
    assert_eq!(h.frame_step_time_millis, 50u8);
    assert_eq!(h.compression_type, TFCompressionType::TF_COMPRESSION_ZSTD);
    assert_eq!(h.compression_block_count, 3u8);
    assert_eq!(h.channel_range_count, 2u8);
    assert_eq!(h.sequence_uid, 139556248100296312u64);
    // ep should point to bd + 32, i.e. remaining slice has length 40 - 32 = 8
    assert_eq!(ep_slice.len(), 40 - 32);
    assert_eq!(ep_slice[0], 0xDE);
    assert_eq!(ep_slice[1], 0xAD);
    assert_eq!(ep_slice[2], 0xBE);
    assert_eq!(ep_slice[3], 0xEF);
}

#[test]
fn test_header_read_compression_none() {
    let mut buf: [u8; 32] = [0; 32];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    buf[20] = 0x00; // compression NONE
    let mut h = make_header();
    let err = tf_header_read(&buf, &mut h, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(h.compression_type, TFCompressionType::TF_COMPRESSION_NONE);
}

#[test]
fn test_header_read_compression_zlib() {
    let mut buf: [u8; 32] = [0; 32];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    buf[20] = 0x02; // compression ZLIB
    let mut h = make_header();
    let err = tf_header_read(&buf, &mut h, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(h.compression_type, TFCompressionType::TF_COMPRESSION_ZLIB);
}

#[test]
fn test_header_invalid_magic() {
    let mut buf: [u8; 32] = [0; 32];
    buf[0] = b'X';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    let mut h = make_header();
    let err = tf_header_read(&buf, &mut h, None);
    assert_eq!(err, TFError::TF_EINVALID_MAGIC);
}

#[test]
fn test_header_buffer_too_small() {
    let buf: [u8; 10] = [0; 10];
    let mut h = make_header();
    let err = tf_header_read(&buf, &mut h, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_header_invalid_compression() {
    let mut buf: [u8; 32] = [0; 32];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    // lower 4 bits = 5 => invalid compression type
    buf[20] = 0x05;
    let mut h = make_header();
    let err = tf_header_read(&buf, &mut h, None);
    assert_eq!(err, TFError::TF_EINVALID_COMPRESSION_TYPE);
}

#[test]
fn test_header_compression_upper_bits_ignored() {
    // 0xF1 -> lower 4 bits = 1 (ZSTD), upper 4 bits = 0xF (ignored)
    let mut buf: [u8; 32] = [0; 32];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    buf[20] = 0xF1;
    let mut h = make_header();
    let err = tf_header_read(&buf, &mut h, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(h.compression_type, TFCompressionType::TF_COMPRESSION_ZSTD);
}

#[test]
fn test_compression_block_valid() {
    let buf: [u8; 10] = [0x01, 0x02, 0x03, 0x04, 0xAA, 0xBB, 0xCC, 0xDD, 0, 0];
    let mut b = make_block();
    let mut ep_slice: &[u8] = &[];
    let err = tf_compression_block_read(&buf, &mut b, Some(&mut ep_slice));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(b.first_frame_id, 67305985u32);
    assert_eq!(b.size, 3721182122u32);
    assert_eq!(ep_slice.len(), 10 - 8);
}

#[test]
fn test_compression_block_no_ep() {
    let buf: [u8; 8] = [0x10, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00];
    let mut b = make_block();
    let err = tf_compression_block_read(&buf, &mut b, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(b.first_frame_id, 16u32);
    assert_eq!(b.size, 32u32);
}

#[test]
fn test_compression_block_buffer_too_small() {
    let buf: [u8; 4] = [0; 4];
    let mut b = make_block();
    let err = tf_compression_block_read(&buf, &mut b, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_var_header_valid_with_value() {
    let buf: [u8; 8] = [0x08, 0x00, b'A', b'B', b'a', b'b', b'c', b'd'];
    let mut v = make_var();
    let mut vd = [0u8; 4];
    let mut ep_slice: &[u8] = &[];
    let err = tf_var_header_read(&buf, &mut v, &mut vd, Some(&mut ep_slice));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(v.size, 8u16);
    assert_eq!(v.id[0], 65u8);
    assert_eq!(v.id[1], 66u8);
    assert_eq!(vd, [97u8, 98, 99, 100]);
    // ep points to bd + size = bd + 8 (which is one past end)
    assert_eq!(ep_slice.len(), 0);
}

#[test]
fn test_var_header_no_vd() {
    // Pass an empty vd buffer to mimic NULL behavior.
    let buf: [u8; 8] = [0x08, 0x00, b'A', b'B', b'a', b'b', b'c', b'd'];
    let mut v = make_var();
    let mut empty: [u8; 0] = [];
    let mut ep_slice: &[u8] = &[];
    let err = tf_var_header_read(&buf, &mut v, &mut empty, Some(&mut ep_slice));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(v.size, 8u16);
    assert_eq!(v.id[0], 65u8);
    assert_eq!(v.id[1], 66u8);
    assert_eq!(ep_slice.len(), 0);
}

#[test]
fn test_var_header_buffer_too_small() {
    // bs = 4, header requires bs > 4
    let buf: [u8; 4] = [0x08, 0x00, b'A', b'B'];
    let mut v = make_var();
    let mut empty: [u8; 0] = [];
    let err = tf_var_header_read(&buf, &mut v, &mut empty, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_var_header_invalid_var_size_field() {
    // size in header = 4, which is <= VAR_HEADER_SIZE
    let buf: [u8; 8] = [0x04, 0x00, b'A', b'B', 0, 0, 0, 0];
    let mut v = make_var();
    let mut empty: [u8; 0] = [];
    let err = tf_var_header_read(&buf, &mut v, &mut empty, None);
    assert_eq!(err, TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_var_header_bs_lt_size_with_vd() {
    // size says 10 but only 8 bytes available
    let buf: [u8; 8] = [0x0A, 0x00, b'A', b'B', 1, 2, 3, 4];
    let mut v = make_var();
    let mut vd = [0u8; 6];
    let err = tf_var_header_read(&buf, &mut v, &mut vd, None);
    assert_eq!(err, TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_var_header_vd_too_small() {
    // size = 8 means value_size = 4; but vd only has 2 bytes
    let buf: [u8; 8] = [0x08, 0x00, b'A', b'B', 1, 2, 3, 4];
    let mut v = make_var();
    let mut vd = [0u8; 2];
    let err = tf_var_header_read(&buf, &mut v, &mut vd, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_channel_range_valid() {
    let buf: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0, 0];
    let mut r = make_range();
    let mut ep_slice: &[u8] = &[];
    let err = tf_channel_range_read(&buf, &mut r, Some(&mut ep_slice));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(r.first_channel_number, 197121u32);
    assert_eq!(r.channel_count, 394500u32);
    assert_eq!(ep_slice.len(), 8 - 6);
}

#[test]
fn test_channel_range_buffer_too_small() {
    let buf: [u8; 3] = [0, 0, 0];
    let mut r = make_range();
    let err = tf_channel_range_read(&buf, &mut r, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_channel_range_zero() {
    let buf: [u8; 6] = [0, 0, 0, 0, 0, 0];
    let mut r = make_range();
    let err = tf_channel_range_read(&buf, &mut r, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(r.first_channel_number, 0u32);
    assert_eq!(r.channel_count, 0u32);
}

#[test]
fn test_channel_range_max_24bit() {
    let buf: [u8; 6] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut r = make_range();
    let err = tf_channel_range_read(&buf, &mut r, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(r.first_channel_number, 0x00FFFFFFu32);
    assert_eq!(r.channel_count, 0x00FFFFFFu32);
}

fn main() {}
