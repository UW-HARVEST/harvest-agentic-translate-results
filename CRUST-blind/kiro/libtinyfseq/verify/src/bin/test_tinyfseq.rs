use libtinyfseq::tinyfseq::*;

// === TFError::to_string ===

#[test]
fn test_error_string_ok() {
    assert_eq!(TFError::TF_OK.to_string(), "TF_OK (ok)");
}

#[test]
fn test_error_string_magic() {
    assert_eq!(TFError::TF_EINVALID_MAGIC.to_string(), "TF_EINVALID_MAGIC (invalid magic file signature)");
}

#[test]
fn test_error_string_compression() {
    assert_eq!(TFError::TF_EINVALID_COMPRESSION_TYPE.to_string(), "TF_EINVALID_COMPRESSION_TYPE (unknown compression identifier)");
}

#[test]
fn test_error_string_buffer_size() {
    assert_eq!(TFError::TF_EINVALID_BUFFER_SIZE.to_string(), "TF_EINVALID_BUFFER_SIZE (undersized data decoding buffer argument)");
}

#[test]
fn test_error_string_var_size() {
    assert_eq!(TFError::TF_EINVALID_VAR_SIZE.to_string(), "TF_EINVALID_VAR_SIZE (invalid variable size in header)");
}

// === TFHeader_read ===

#[test]
fn test_header_read_buffer_too_small() {
    let bd = [0u8; 31];
    let mut h = make_header();
    assert_eq!(tf_header_read(&bd, &mut h, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_header_read_bad_magic() {
    let mut bd = [0u8; 32];
    bd[0] = b'X';
    let mut h = make_header();
    assert_eq!(tf_header_read(&bd, &mut h, None), TFError::TF_EINVALID_MAGIC);
}

#[test]
fn test_header_read_bad_compression() {
    let mut bd = [0u8; 32];
    bd[0] = b'P'; bd[1] = b'S'; bd[2] = b'E'; bd[3] = b'Q';
    bd[20] = 0x0F;
    let mut h = make_header();
    assert_eq!(tf_header_read(&bd, &mut h, None), TFError::TF_EINVALID_COMPRESSION_TYPE);
}

#[test]
fn test_header_read_valid() {
    let mut bd = [0u8; 64];
    bd[0] = b'P'; bd[1] = b'S'; bd[2] = b'E'; bd[3] = b'Q';
    bd[4] = 0x40; bd[5] = 0x01; // channelDataOffset = 320
    bd[6] = 1; bd[7] = 2;       // minor=1, major=2
    bd[8] = 0x20; bd[9] = 0x00; // variableDataOffset = 32
    bd[10] = 0x00; bd[11] = 0x01; bd[12] = 0x00; bd[13] = 0x00; // channelCount = 256
    bd[14] = 0xE8; bd[15] = 0x03; bd[16] = 0x00; bd[17] = 0x00; // frameCount = 1000
    bd[18] = 50;                 // frameStepTimeMillis
    bd[20] = 0xA1;               // compression type zstd (lower 4 bits = 1)
    bd[21] = 3;                  // compressionBlockCount
    bd[22] = 2;                  // channelRangeCount
    // sequenceUid = 72623859790382856
    bd[24] = 0x08; bd[25] = 0x07; bd[26] = 0x06; bd[27] = 0x05;
    bd[28] = 0x04; bd[29] = 0x03; bd[30] = 0x02; bd[31] = 0x01;

    let mut h = make_header();
    let mut ep: &[u8] = &[];
    let err = tf_header_read(&bd, &mut h, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(h.channel_data_offset, 320);
    assert_eq!(h.minor_version, 1);
    assert_eq!(h.major_version, 2);
    assert_eq!(h.variable_data_offset, 32);
    assert_eq!(h.channel_count, 256);
    assert_eq!(h.frame_count, 1000);
    assert_eq!(h.frame_step_time_millis, 50);
    assert!(matches!(h.compression_type, TFCompressionType::TF_COMPRESSION_ZSTD));
    assert_eq!(h.compression_block_count, 3);
    assert_eq!(h.channel_range_count, 2);
    assert_eq!(h.sequence_uid, 72623859790382856);
    assert_eq!(ep.len(), 64 - 32); // ep points to offset 32
}

#[test]
fn test_header_read_compression_none_masked() {
    let mut bd = [0u8; 32];
    bd[0] = b'P'; bd[1] = b'S'; bd[2] = b'E'; bd[3] = b'Q';
    bd[20] = 0xF0; // upper bits set, lower 4 = 0 => NONE
    let mut h = make_header();
    assert_eq!(tf_header_read(&bd, &mut h, None), TFError::TF_OK);
    assert!(matches!(h.compression_type, TFCompressionType::TF_COMPRESSION_NONE));
}

#[test]
fn test_header_read_compression_zlib() {
    let mut bd = [0u8; 32];
    bd[0] = b'P'; bd[1] = b'S'; bd[2] = b'E'; bd[3] = b'Q';
    bd[20] = 0x02;
    let mut h = make_header();
    assert_eq!(tf_header_read(&bd, &mut h, None), TFError::TF_OK);
    assert!(matches!(h.compression_type, TFCompressionType::TF_COMPRESSION_ZLIB));
}

#[test]
fn test_header_read_exact_32() {
    let mut bd = [0u8; 32];
    bd[0] = b'P'; bd[1] = b'S'; bd[2] = b'E'; bd[3] = b'Q';
    let mut h = make_header();
    assert_eq!(tf_header_read(&bd, &mut h, None), TFError::TF_OK);
}

// === TFCompressionBlock_read ===

#[test]
fn test_compression_block_too_small() {
    let bd = [0u8; 7];
    let mut b = TFCompressionBlock { first_frame_id: 0, size: 0 };
    assert_eq!(tf_compression_block_read(&bd, &mut b, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_compression_block_valid() {
    let mut bd = [0u8; 16];
    bd[0] = 0x64; // firstFrameId = 100
    bd[4] = 0x00; bd[5] = 0x10; // size = 4096
    let mut b = TFCompressionBlock { first_frame_id: 0, size: 0 };
    let mut ep: &[u8] = &[];
    let err = tf_compression_block_read(&bd, &mut b, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(b.first_frame_id, 100);
    assert_eq!(b.size, 4096);
    assert_eq!(ep.len(), 16 - 8);
}

#[test]
fn test_compression_block_max_values() {
    let bd = [0xFFu8; 8];
    let mut b = TFCompressionBlock { first_frame_id: 0, size: 0 };
    let err = tf_compression_block_read(&bd, &mut b, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(b.first_frame_id, 4294967295);
    assert_eq!(b.size, 4294967295);
}

// === TFChannelRange_read ===

#[test]
fn test_channel_range_too_small() {
    let bd = [0u8; 5];
    let mut r = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    assert_eq!(tf_channel_range_read(&bd, &mut r, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_channel_range_valid() {
    let mut bd = [0u8; 12];
    bd[0] = 0x03; bd[1] = 0x02; bd[2] = 0x01; // firstChannelNumber = 66051
    bd[3] = 0x06; bd[4] = 0x05; bd[5] = 0x04; // channelCount = 263430
    let mut r = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    let mut ep: &[u8] = &[];
    let err = tf_channel_range_read(&bd, &mut r, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(r.first_channel_number, 66051);
    assert_eq!(r.channel_count, 263430);
    assert_eq!(ep.len(), 12 - 6);
}

#[test]
fn test_channel_range_max() {
    let bd = [0xFFu8; 6];
    let mut r = TFChannelRange { first_channel_number: 0, channel_count: 0 };
    let err = tf_channel_range_read(&bd, &mut r, None);
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(r.first_channel_number, 16777215);
    assert_eq!(r.channel_count, 16777215);
}

// === TFVarHeader_read ===

#[test]
fn test_var_header_too_small_4() {
    let bd = [0u8; 4];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    assert_eq!(tf_var_header_read(&bd, &mut vh, &mut [], None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_var_header_exact_4() {
    let bd = [0x0Au8, 0x00, b'a', b'b'];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    assert_eq!(tf_var_header_read(&bd, &mut vh, &mut [], None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_var_header_size_eq_4() {
    let bd = [0x04u8, 0x00, b'a', b'b', 0, 0, 0, 0];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    assert_eq!(tf_var_header_read(&bd, &mut vh, &mut [], None), TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_var_header_size_3() {
    let bd = [0x03u8, 0x00, b'x', b'y', 0, 0, 0, 0];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    assert_eq!(tf_var_header_read(&bd, &mut vh, &mut [], None), TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_var_header_size_0() {
    let bd = [0x00u8, 0x00, b'x', b'y', 0, 0, 0, 0];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    assert_eq!(tf_var_header_read(&bd, &mut vh, &mut [], None), TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_var_header_valid_no_vd() {
    let bd = [0x08u8, 0x00, b'a', b'b', b'h', b'e', b'l', b'l', 0, 0];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut ep: &[u8] = &[];
    let err = tf_var_header_read(&bd, &mut vh, &mut [], Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 8);
    assert_eq!(vh.id, [b'a', b'b']);
    assert_eq!(ep.len(), 10 - 8); // ep points to offset 8
}

#[test]
fn test_var_header_valid_with_vd() {
    let bd = [0x08u8, 0x00, b'a', b'b', b'D', b'A', b'T', b'A', 0, 0];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 10];
    let mut ep: &[u8] = &[];
    let err = tf_var_header_read(&bd, &mut vh, &mut vd, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 8);
    assert_eq!(vh.id, [b'a', b'b']);
    assert_eq!(&vd[..4], b"DATA");
    assert_eq!(ep.len(), 10 - 8);
}

#[test]
fn test_var_header_vd_too_small() {
    let bd = [0x08u8, 0x00, b'a', b'b', b'D', b'A', b'T', b'A', 0, 0];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 2];
    assert_eq!(tf_var_header_read(&bd, &mut vh, &mut vd, None), TFError::TF_EINVALID_BUFFER_SIZE);
}

#[test]
fn test_var_header_bd_too_small_for_value() {
    let bd = [0x08u8, 0x00, b'a', b'b', b'D', b'A'];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 10];
    assert_eq!(tf_var_header_read(&bd, &mut vh, &mut vd, None), TFError::TF_EINVALID_VAR_SIZE);
}

#[test]
fn test_var_header_size5_with_vd() {
    let bd = [0x05u8, 0x00, b'c', b'd', b'X', 0];
    let mut vh = TFVarHeader { size: 0, id: [0; 2] };
    let mut vd = [0u8; 4];
    let mut ep: &[u8] = &[];
    let err = tf_var_header_read(&bd, &mut vh, &mut vd, Some(&mut ep));
    assert_eq!(err, TFError::TF_OK);
    assert_eq!(vh.size, 5);
    assert_eq!(vh.id, [b'c', b'd']);
    assert_eq!(vd[0], b'X');
    assert_eq!(ep.len(), 6 - 5);
}

// === Helpers ===

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

fn main() {}
