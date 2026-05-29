use libtinyfseq::tinyfseq::{
    tf_channel_range_read, tf_compression_block_read, tf_header_read, tf_var_header_read,
    TFChannelRange, TFCompressionBlock, TFCompressionType, TFError, TFHeader, TFVarHeader,
};

// ---------- TFError::to_string ----------

#[test]
fn test_tferror_string_ok() {
    assert_eq!(TFError::TF_OK.to_string(), "TF_OK (ok)");
}

#[test]
fn test_tferror_string_invalid_magic() {
    assert_eq!(
        TFError::TF_EINVALID_MAGIC.to_string(),
        "TF_EINVALID_MAGIC (invalid magic file signature)"
    );
}

#[test]
fn test_tferror_string_invalid_compression_type() {
    assert_eq!(
        TFError::TF_EINVALID_COMPRESSION_TYPE.to_string(),
        "TF_EINVALID_COMPRESSION_TYPE (unknown compression identifier)"
    );
}

#[test]
fn test_tferror_string_invalid_buffer_size() {
    assert_eq!(
        TFError::TF_EINVALID_BUFFER_SIZE.to_string(),
        "TF_EINVALID_BUFFER_SIZE (undersized data decoding buffer argument)"
    );
}

#[test]
fn test_tferror_string_invalid_var_size() {
    assert_eq!(
        TFError::TF_EINVALID_VAR_SIZE.to_string(),
        "TF_EINVALID_VAR_SIZE (invalid variable size in header)"
    );
}

// ---------- TFError discriminants ----------

#[test]
fn test_tferror_discriminants() {
    // Match the C `tf_err_t` enum values: TF_OK=0, etc.
    assert_eq!(TFError::TF_OK as i32, 0);
    assert_eq!(TFError::TF_EINVALID_MAGIC as i32, 1);
    assert_eq!(TFError::TF_EINVALID_COMPRESSION_TYPE as i32, 2);
    assert_eq!(TFError::TF_EINVALID_BUFFER_SIZE as i32, 3);
    assert_eq!(TFError::TF_EINVALID_VAR_SIZE as i32, 4);
}

// ---------- helpers for default structs ----------

fn fresh_header() -> TFHeader {
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

fn fresh_block() -> TFCompressionBlock {
    TFCompressionBlock {
        first_frame_id: 0,
        size: 0,
    }
}

fn fresh_var_header() -> TFVarHeader {
    TFVarHeader {
        size: 0,
        id: [0, 0],
    }
}

fn fresh_channel_range() -> TFChannelRange {
    TFChannelRange {
        first_channel_number: 0,
        channel_count: 0,
    }
}

// ---------- tf_header_read ----------

#[test]
fn test_tf_header_read_valid() {
    let mut buf = [0u8; 40];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    buf[4] = 0x10;
    buf[5] = 0x02; // channelDataOffset = 528
    buf[6] = 0x00; // minor
    buf[7] = 0x02; // major
    buf[8] = 0x20;
    buf[9] = 0x00; // variableDataOffset = 32
    buf[10] = 0x00;
    buf[11] = 0x10;
    buf[12] = 0x00;
    buf[13] = 0x00; // channelCount = 4096
    buf[14] = 0xE8;
    buf[15] = 0x03;
    buf[16] = 0x00;
    buf[17] = 0x00; // frameCount = 1000
    buf[18] = 25; // frameStepTimeMillis
    buf[20] = 0x21; // compression byte: lower 4 bits = 1 (ZSTD), upper bits ignored
    buf[21] = 5; // compressionBlockCount
    buf[22] = 3; // channelRangeCount
    buf[24] = 0x01;
    buf[25] = 0x02;
    buf[26] = 0x03;
    buf[27] = 0x04;
    buf[28] = 0x05;
    buf[29] = 0x06;
    buf[30] = 0x07;
    buf[31] = 0x08;
    // sequenceUid = 0x0807060504030201

    let mut header = fresh_header();
    let mut ep: &[u8] = &[];
    let err = tf_header_read(&buf, &mut header, Some(&mut ep));

    assert_eq!(err, TFError::TF_OK);
    assert_eq!(header.channel_data_offset, 528);
    assert_eq!(header.minor_version, 0);
    assert_eq!(header.major_version, 2);
    assert_eq!(header.variable_data_offset, 32);
    assert_eq!(header.channel_count, 4096);
    assert_eq!(header.frame_count, 1000);
    assert_eq!(header.frame_step_time_millis, 25);
    assert!(matches!(
        header.compression_type,
        TFCompressionType::TF_COMPRESSION_ZSTD
    ));
    assert_eq!(header.compression_block_count, 5);
    assert_eq!(header.channel_range_count, 3);
    assert_eq!(header.sequence_uid, 0x0807060504030201u64);
    // ep should point to byte 32 of buf (8 bytes remaining)
    assert_eq!(ep.len(), 8);
    assert_eq!(ep.as_ptr(), unsafe { buf.as_ptr().add(32) });
}

#[test]
fn test_tf_header_read_small_buffer() {
    let buf = [0u8; 10];
    let mut header = fresh_header();
    let err = tf_header_read(&buf, &mut header, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_tf_header_read_invalid_magic() {
    let mut buf = [0u8; 32];
    buf[0] = b'X';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    let mut header = fresh_header();
    let err = tf_header_read(&buf, &mut header, None);
    assert_eq!(err, TFError::TF_EINVALID_MAGIC);
}

#[test]
fn test_tf_header_read_invalid_compression_type() {
    let mut buf = [0u8; 32];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    buf[20] = 0x05; // 5 is invalid (only 0,1,2 valid)
    let mut header = fresh_header();
    let err = tf_header_read(&buf, &mut header, None);
    assert_eq!(err, TFError::TF_EINVALID_COMPRESSION_TYPE);
}

#[test]
fn test_tf_header_read_compression_none_upper_bits_ignored() {
    let mut buf = [0u8; 32];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    buf[20] = 0xF0; // upper 4 bits set, lower 4 bits = 0 (NONE)
    let mut header = fresh_header();
    let err = tf_header_read(&buf, &mut header, None);
    assert_eq!(err, TFError::TF_OK);
    assert!(matches!(
        header.compression_type,
        TFCompressionType::TF_COMPRESSION_NONE
    ));
}

#[test]
fn test_tf_header_read_compression_zlib() {
    let mut buf = [0u8; 32];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    buf[20] = 0x02;
    let mut header = fresh_header();
    let err = tf_header_read(&buf, &mut header, None);
    assert_eq!(err, TFError::TF_OK);
    assert!(matches!(
        header.compression_type,
        TFCompressionType::TF_COMPRESSION_ZLIB
    ));
}

#[test]
fn test_tf_header_read_no_ep() {
    let mut buf = [0u8; 32];
    buf[0] = b'P';
    buf[1] = b'S';
    buf[2] = b'E';
    buf[3] = b'Q';
    let mut header = fresh_header();
    let err = tf_header_read(&buf, &mut header, None);
    assert_eq!(err, TFError::TF_OK);
}

// ---------- tf_compression_block_read ----------

#[test]
fn test_tf_compression_block_read_valid() {
    let buf: [u8; 10] = [0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44, 0xFF, 0xFF];
    let mut block = fresh_block();
    let mut ep: &[u8] = &[];
    let err = tf_compression_block_read(&buf, &mut block, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(block.first_frame_id, 0xDDCCBBAA);
    assert_eq!(block.size, 0x44332211);
    assert_eq!(ep.len(), 2);
    assert_eq!(ep.as_ptr(), unsafe { buf.as_ptr().add(8) });
}

#[test]
fn test_tf_compression_block_read_small_buffer() {
    let buf = [0u8; 7];
    let mut block = fresh_block();
    let err = tf_compression_block_read(&buf, &mut block, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_tf_compression_block_read_no_ep() {
    let buf: [u8; 8] = [1, 0, 0, 0, 2, 0, 0, 0];
    let mut block = fresh_block();
    let err = tf_compression_block_read(&buf, &mut block, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(block.first_frame_id, 1);
    assert_eq!(block.size, 2);
}

// ---------- tf_var_header_read ----------

#[test]
fn test_tf_var_header_read_valid_with_vd() {
    let buf: [u8; 10] = [0x07, 0x00, b'm', b'f', b'a', b'b', b'c', 0x00, 0xAA, 0xBB];
    let mut vh = fresh_var_header();
    let mut vd = [0u8; 16];
    let mut ep: &[u8] = &[];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 7);
    assert_eq!(vh.id, [b'm', b'f']);
    // size = 7, value bytes after the 4-byte header are 3 bytes: 'a','b','c'
    assert_eq!(&vd[..3], &[b'a', b'b', b'c']);
    assert_eq!(ep.len(), 3); // buf has 10 bytes, var size = 7, so 3 bytes remain
    assert_eq!(ep.as_ptr(), unsafe { buf.as_ptr().add(7) });
}

#[test]
fn test_tf_var_header_read_vd_empty() {
    let buf: [u8; 6] = [0x05, 0x00, b'm', b'f', b'X', 0xCC];
    let mut vh = fresh_var_header();
    let mut vd: [u8; 0] = [];
    let mut ep: &[u8] = &[];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 5);
    assert_eq!(vh.id, [b'm', b'f']);
    assert_eq!(ep.len(), 1);
    assert_eq!(ep.as_ptr(), unsafe { buf.as_ptr().add(5) });
}

#[test]
fn test_tf_var_header_read_buffer_too_small_eq_4() {
    // bs <= VAR_HEADER_SIZE (4) is an error.
    let buf = [0u8; 4];
    let mut vh = fresh_var_header();
    let mut vd: [u8; 0] = [];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_tf_var_header_read_size_le_4() {
    // varHeader.size <= 4 is invalid.
    let buf: [u8; 10] = [0x04, 0x00, b'm', b'f', b'a', b'b', b'c', 0x00, 0xAA, 0xBB];
    let mut vh = fresh_var_header();
    let mut vd: [u8; 0] = [];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, None);
    assert_eq!(err, TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_tf_var_header_read_buffer_smaller_than_size() {
    // size = 0x0A but buffer is only 6 bytes; should fail when vd is provided.
    let buf: [u8; 6] = [0x0A, 0x00, b'm', b'f', b'a', b'b'];
    let mut vh = fresh_var_header();
    let mut vd = [0u8; 16];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, None);
    assert_eq!(err, TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_tf_var_header_read_vs_too_small() {
    // size = 8 means 4 value bytes; but vd has only 2 bytes available.
    let buf: [u8; 10] = [0x08, 0x00, b'm', b'f', 1, 2, 3, 4, 0xAA, 0xBB];
    let mut vh = fresh_var_header();
    let mut vd = [0u8; 2];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_tf_var_header_read_no_ep_no_vd() {
    let buf: [u8; 6] = [0x05, 0x00, b'm', b'f', b'Z', 0xCC];
    let mut vh = fresh_var_header();
    let mut vd: [u8; 0] = [];
    let err = tf_var_header_read(&buf, &mut vh, &mut vd, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 5);
    assert_eq!(vh.id, [b'm', b'f']);
}

// ---------- tf_channel_range_read ----------

#[test]
fn test_tf_channel_range_read_valid() {
    let buf: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0xFF, 0xFF];
    let mut cr = fresh_channel_range();
    let mut ep: &[u8] = &[];
    let err = tf_channel_range_read(&buf, &mut cr, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(cr.first_channel_number, 0x030201);
    assert_eq!(cr.channel_count, 0x060504);
    assert_eq!(ep.len(), 2);
    assert_eq!(ep.as_ptr(), unsafe { buf.as_ptr().add(6) });
}

#[test]
fn test_tf_channel_range_read_small_buffer() {
    let buf = [0u8; 5];
    let mut cr = fresh_channel_range();
    let err = tf_channel_range_read(&buf, &mut cr, None);
    assert_eq!(err, TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_tf_channel_range_read_no_ep() {
    let buf: [u8; 6] = [0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x80];
    let mut cr = fresh_channel_range();
    let err = tf_channel_range_read(&buf, &mut cr, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(cr.first_channel_number, 0xFFFFFF);
    assert_eq!(cr.channel_count, 0x800000);
}

fn main() {}
