use libtinyfseq::tinyfseq::*;

// ── TFError::to_string ──

#[test]
fn test_error_strings() {
    assert_eq!(TFError::TF_OK.to_string(), "TF_OK (ok)");
    assert_eq!(TFError::TF_EINVALID_MAGIC.to_string(), "TF_EINVALID_MAGIC (invalid magic file signature)");
    assert_eq!(TFError::TF_EINVALID_COMPRESSION_TYPE.to_string(), "TF_EINVALID_COMPRESSION_TYPE (unknown compression identifier)");
    assert_eq!(TFError::TF_EINVALID_BUFFER_SIZE.to_string(), "TF_EINVALID_BUFFER_SIZE (undersized data decoding buffer argument)");
    assert_eq!(TFError::TF_EINVALID_VAR_SIZE.to_string(), "TF_EINVALID_VAR_SIZE (invalid variable size in header)");
}

// ── Helper: build a valid 32-byte FSEQ header ──

fn make_header_buf() -> [u8; 32] {
    let mut b = [0u8; 32];
    // magic
    b[0] = b'P'; b[1] = b'S'; b[2] = b'E'; b[3] = b'Q';
    // channelDataOffset = 0x0020 (32) LE
    b[4] = 0x20; b[5] = 0x00;
    // minorVersion=1, majorVersion=2
    b[6] = 1; b[7] = 2;
    // variableDataOffset = 0x0040 (64) LE
    b[8] = 0x40; b[9] = 0x00;
    // channelCount = 100 LE
    b[10] = 100; b[11] = 0; b[12] = 0; b[13] = 0;
    // frameCount = 5000 LE
    b[14] = 0x88; b[15] = 0x13; b[16] = 0; b[17] = 0;
    // frameStepTimeMillis = 50
    b[18] = 50;
    // byte 19 unused (flags)
    // compressionType = 0 (none), lower 4 bits
    b[20] = 0;
    // compressionBlockCount = 3
    b[21] = 3;
    // channelRangeCount = 2
    b[22] = 2;
    // byte 23 unused
    // sequenceUid = 0x0102030405060708 LE
    b[24] = 0x08; b[25] = 0x07; b[26] = 0x06; b[27] = 0x05;
    b[28] = 0x04; b[29] = 0x03; b[30] = 0x02; b[31] = 0x01;
    b
}

fn default_header() -> TFHeader {
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

// ── tf_header_read ──

#[test]
fn test_header_read_ok() {
    let buf = make_header_buf();
    let mut h = default_header();
    let err = tf_header_read(&buf, &mut h, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(h.channel_data_offset, 0x0020);
    assert_eq!(h.minor_version, 1);
    assert_eq!(h.major_version, 2);
    assert_eq!(h.variable_data_offset, 0x0040);
    assert_eq!(h.channel_count, 100);
    assert_eq!(h.frame_count, 5000);
    assert_eq!(h.frame_step_time_millis, 50);
    assert!(matches!(h.compression_type, TFCompressionType::TF_COMPRESSION_NONE));
    assert_eq!(h.compression_block_count, 3);
    assert_eq!(h.channel_range_count, 2);
    assert_eq!(h.sequence_uid, 0x0102030405060708);
}

#[test]
fn test_header_read_end_pointer() {
    let mut buf = [0u8; 40];
    buf[..32].copy_from_slice(&make_header_buf());
    buf[32..40].copy_from_slice(&[0xAA; 8]);
    let mut h = default_header();
    let mut ep: &[u8] = &[];
    let err = tf_header_read(&buf, &mut h, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(ep.len(), 8);
    assert_eq!(ep[0], 0xAA);
}

#[test]
fn test_header_read_buffer_too_small() {
    let buf = [0u8; 31];
    let mut h = default_header();
    assert_eq!(tf_header_read(&buf, &mut h, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_header_read_empty_buffer() {
    let buf: [u8; 0] = [];
    let mut h = default_header();
    assert_eq!(tf_header_read(&buf, &mut h, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_header_read_bad_magic() {
    let mut buf = make_header_buf();
    buf[0] = b'X';
    let mut h = default_header();
    assert_eq!(tf_header_read(&buf, &mut h, None), TFError::TF_EINVALID_MAGIC);
}

#[test]
fn test_header_read_invalid_compression_type() {
    let mut buf = make_header_buf();
    buf[20] = 0x03; // invalid compression type in lower nibble
    let mut h = default_header();
    assert_eq!(tf_header_read(&buf, &mut h, None), TFError::TF_EINVALID_COMPRESSION_TYPE);
}

#[test]
fn test_header_read_compression_zstd() {
    let mut buf = make_header_buf();
    buf[20] = 0x01;
    let mut h = default_header();
    assert_eq!(tf_header_read(&buf, &mut h, None), TFError::TF_OK);
    assert!(matches!(h.compression_type, TFCompressionType::TF_COMPRESSION_ZSTD));
}

#[test]
fn test_header_read_compression_zlib() {
    let mut buf = make_header_buf();
    buf[20] = 0x02;
    let mut h = default_header();
    assert_eq!(tf_header_read(&buf, &mut h, None), TFError::TF_OK);
    assert!(matches!(h.compression_type, TFCompressionType::TF_COMPRESSION_ZLIB));
}

#[test]
fn test_header_read_compression_upper_bits_masked() {
    // Upper 4 bits should be masked off; 0xF0 | 0x01 = 0xF1, lower nibble = 1 = ZSTD
    let mut buf = make_header_buf();
    buf[20] = 0xF1;
    let mut h = default_header();
    assert_eq!(tf_header_read(&buf, &mut h, None), TFError::TF_OK);
    assert!(matches!(h.compression_type, TFCompressionType::TF_COMPRESSION_ZSTD));
}

#[test]
fn test_header_read_exact_32_bytes() {
    let buf = make_header_buf();
    let mut h = default_header();
    assert_eq!(tf_header_read(&buf, &mut h, None), TFError::TF_OK);
}

// ── tf_compression_block_read ──

#[test]
fn test_compression_block_read_ok() {
    // firstFrameId=100 LE, size=2048 LE
    let buf: [u8; 8] = [100, 0, 0, 0, 0, 8, 0, 0];
    let mut block = TFCompressionBlock { first_frame_id: 0, size: 0 };
    let err = tf_compression_block_read(&buf, &mut block, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(block.first_frame_id, 100);
    assert_eq!(block.size, 2048);
}

#[test]
fn test_compression_block_read_end_pointer() {
    let buf: [u8; 12] = [1, 0, 0, 0, 2, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut block = TFCompressionBlock { first_frame_id: 0, size: 0 };
    let mut ep: &[u8] = &[];
    let err = tf_compression_block_read(&buf, &mut block, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(ep.len(), 4);
    assert_eq!(ep[0], 0xFF);
}

#[test]
fn test_compression_block_read_buffer_too_small() {
    let buf = [0u8; 7];
    let mut block = TFCompressionBlock { first_frame_id: 0, size: 0 };
    assert_eq!(tf_compression_block_read(&buf, &mut block, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_compression_block_read_empty() {
    let buf: [u8; 0] = [];
    let mut block = TFCompressionBlock { first_frame_id: 0, size: 0 };
    assert_eq!(tf_compression_block_read(&buf, &mut block, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_compression_block_read_max_values() {
    let buf: [u8; 8] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut block = TFCompressionBlock { first_frame_id: 0, size: 0 };
    assert_eq!(tf_compression_block_read(&buf, &mut block, None), TFError::TF_OK);
    assert_eq!(block.first_frame_id, u32::MAX);
    assert_eq!(block.size, u32::MAX);
}

// ── tf_var_header_read ──

#[test]
fn test_var_header_read_ok_with_data() {
    // size=7 (4 header + 3 value), id='m','f', value = "abc"
    let buf: [u8; 7] = [7, 0, b'm', b'f', b'a', b'b', b'c'];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 3];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 7);
    assert_eq!(vh.id, [b'm', b'f']);
    assert_eq!(&vd, b"abc");
}

#[test]
fn test_var_header_read_skip_value_with_empty_vd() {
    // When vd is empty slice, value copy is skipped (like NULL vd in C)
    let buf: [u8; 7] = [7, 0, b'x', b'y', 1, 2, 3];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd: [u8; 0] = [];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 7);
    assert_eq!(vh.id, [b'x', b'y']);
}

#[test]
fn test_var_header_read_end_pointer() {
    let buf: [u8; 10] = [7, 0, b'a', b'b', 1, 2, 3, 0xAA, 0xBB, 0xCC];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 3];
    let mut ep: &[u8] = &[];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(ep.len(), 3);
    assert_eq!(ep[0], 0xAA);
}

#[test]
fn test_var_header_read_buffer_too_small() {
    // C: bs <= VAR_HEADER_SIZE (4), so bs=4 fails
    let buf = [5, 0, b'a', b'b'];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd: [u8; 0] = [];
    assert_eq!(tf_var_header_read(&buf, &mut vh, &mut vd, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_var_header_read_buffer_empty() {
    let buf: [u8; 0] = [];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd: [u8; 0] = [];
    assert_eq!(tf_var_header_read(&buf, &mut vh, &mut vd, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_var_header_read_size_too_small() {
    // size field <= 4 should return TF_EINVALID_VAR_SIZE
    let buf: [u8; 5] = [4, 0, b'a', b'b', 0];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd: [u8; 0] = [];
    assert_eq!(tf_var_header_read(&buf, &mut vh, &mut vd, None), TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_var_header_read_bd_shorter_than_size() {
    // size=10 but bd only has 7 bytes; with non-empty vd this should fail
    let buf: [u8; 7] = [10, 0, b'a', b'b', 1, 2, 3];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 10];
    assert_eq!(tf_var_header_read(&buf, &mut vh, &mut vd, None), TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_var_header_read_vd_too_small() {
    // size=7 => valueSize=3, but vd only 2 bytes
    let buf: [u8; 7] = [7, 0, b'a', b'b', 1, 2, 3];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 2];
    assert_eq!(tf_var_header_read(&buf, &mut vh, &mut vd, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_var_header_read_minimum_valid() {
    // Minimum valid: size=5 (4 header + 1 value byte), bd must be > 4 so 5 bytes
    let buf: [u8; 5] = [5, 0, b'z', b'z', 42];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 1];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 5);
    assert_eq!(vh.id, [b'z', b'z']);
    assert_eq!(vd[0], 42);
}

// ── tf_channel_range_read ──

#[test]
fn test_channel_range_read_ok() {
    // firstChannelNumber = 0x010203 LE (bytes: 03, 02, 01)
    // channelCount = 0x040506 LE (bytes: 06, 05, 04)
    let buf: [u8; 6] = [0x03, 0x02, 0x01, 0x06, 0x05, 0x04];
    let mut cr = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    let err = tf_channel_range_read(&buf, &mut cr, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(cr.first_channel_number, 0x010203);
    assert_eq!(cr.channel_count, 0x040506);
}

#[test]
fn test_channel_range_read_end_pointer() {
    let buf: [u8; 10] = [1, 0, 0, 2, 0, 0, 0xDD, 0xEE, 0xFF, 0x00];
    let mut cr = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    let mut ep: &[u8] = &[];
    let err = tf_channel_range_read(&buf, &mut cr, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(ep.len(), 4);
    assert_eq!(ep[0], 0xDD);
}

#[test]
fn test_channel_range_read_buffer_too_small() {
    let buf = [0u8; 5];
    let mut cr = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    assert_eq!(tf_channel_range_read(&buf, &mut cr, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_channel_range_read_empty() {
    let buf: [u8; 0] = [];
    let mut cr = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    assert_eq!(tf_channel_range_read(&buf, &mut cr, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_channel_range_read_zeros() {
    let buf = [0u8; 6];
    let mut cr = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    assert_eq!(tf_channel_range_read(&buf, &mut cr, None), TFError::TF_OK);
    assert_eq!(cr.first_channel_number, 0);
    assert_eq!(cr.channel_count, 0);
}

#[test]
fn test_channel_range_read_max_u24() {
    let buf: [u8; 6] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut cr = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    assert_eq!(tf_channel_range_read(&buf, &mut cr, None), TFError::TF_OK);
    assert_eq!(cr.first_channel_number, 0x00FFFFFF);
    assert_eq!(cr.channel_count, 0x00FFFFFF);
}

fn main() {}
