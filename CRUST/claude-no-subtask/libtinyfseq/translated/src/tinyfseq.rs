#[derive(Debug, PartialEq)]
pub enum TFError {
TF_OK = 0,
TF_EINVALID_MAGIC,
TF_EINVALID_COMPRESSION_TYPE,
TF_EINVALID_BUFFER_SIZE,
TF_EINVALID_VAR_SIZE,
}
impl TFError {
pub fn to_string(&self) -> &'static str {
match self {
TFError::TF_OK => "TF_OK (ok)",
TFError::TF_EINVALID_MAGIC => "TF_EINVALID_MAGIC (invalid magic file signature)",
TFError::TF_EINVALID_COMPRESSION_TYPE => "TF_EINVALID_COMPRESSION_TYPE (unknown compression identifier)",
TFError::TF_EINVALID_BUFFER_SIZE => "TF_EINVALID_BUFFER_SIZE (undersized data decoding buffer argument)",
TFError::TF_EINVALID_VAR_SIZE => "TF_EINVALID_VAR_SIZE (invalid variable size in header)",
}
}
}
#[derive(Debug)]
pub enum TFCompressionType {
TF_COMPRESSION_NONE,
TF_COMPRESSION_ZSTD,
TF_COMPRESSION_ZLIB,
}
#[derive(Debug)]
pub struct TFHeader {
pub channel_data_offset: u16,
pub minor_version: u8,
pub major_version: u8,
pub variable_data_offset: u16,
pub channel_count: u32,
pub frame_count: u32,
pub frame_step_time_millis: u8,
pub compression_type: TFCompressionType,
pub compression_block_count: u8,
pub channel_range_count: u8,
pub sequence_uid: u64,
}
#[derive(Debug)]
pub struct TFCompressionBlock {
pub first_frame_id: u32,
pub size: u32,
}
#[derive(Debug)]
pub struct TFVarHeader {
pub size: u16,
pub id: [u8; 2],
}
#[derive(Debug)]
pub struct TFChannelRange {
pub first_channel_number: u32,
pub channel_count: u32,
}
pub fn tf_var_header_read<'a>(
    bd: &'a [u8],
    var_header: &mut TFVarHeader,
    vd: &mut [u8],
    ep: Option<&mut &'a [u8]>,
) -> TFError {
    const VAR_HEADER_SIZE: usize = 4;

    // variable header requires 4 bytes and is NULL terminated
    // an empty variable should be at least 5 bytes
    if bd.len() <= VAR_HEADER_SIZE {
        return TFError::TF_EINVALID_BUFFER_SIZE;
    }

    var_header.size = u16::from_le_bytes([bd[0], bd[1]]);

    if (var_header.size as usize) <= VAR_HEADER_SIZE {
        return TFError::TF_EINVALID_VAR_SIZE;
    }

    var_header.id[0] = bd[2];
    var_header.id[1] = bd[3];

    // only attempt to read variable value if a decoding buffer (`vd`) is provided
    // The C code uses a NULL pointer to mean "skip"; in Rust we use an empty
    // slice to mean "skip" (matching how the test binaries pass an empty &mut []
    // when no variable data is desired).
    if !vd.is_empty() {
        // ensure the source buffer has enough data to read the variable value
        if bd.len() < var_header.size as usize {
            return TFError::TF_EINVALID_VAR_SIZE;
        }

        // ensure the destination buffer is large enough to store the variable value
        let value_size = var_header.size as usize - VAR_HEADER_SIZE;
        if vd.len() < value_size {
            return TFError::TF_EINVALID_BUFFER_SIZE;
        }

        vd[..value_size].copy_from_slice(&bd[VAR_HEADER_SIZE..VAR_HEADER_SIZE + value_size]);
    }

    if let Some(ep) = ep {
        *ep = &bd[var_header.size as usize..];
    }

    TFError::TF_OK
}
pub fn tf_header_read<'a>(
    bd: &'a [u8],
    header: &mut TFHeader,
    ep: Option<&mut &'a [u8]>,
) -> TFError {
    const HEADER_SIZE: usize = 32;

    if bd.len() < HEADER_SIZE {
        return TFError::TF_EINVALID_BUFFER_SIZE;
    }

    if bd[0] != b'P' || bd[1] != b'S' || bd[2] != b'E' || bd[3] != b'Q' {
        return TFError::TF_EINVALID_MAGIC;
    }

    header.channel_data_offset = u16::from_le_bytes([bd[4], bd[5]]);
    header.minor_version = bd[6];
    header.major_version = bd[7];
    header.variable_data_offset = u16::from_le_bytes([bd[8], bd[9]]);
    header.channel_count = u32::from_le_bytes([bd[10], bd[11], bd[12], bd[13]]);
    header.frame_count = u32::from_le_bytes([bd[14], bd[15], bd[16], bd[17]]);
    header.frame_step_time_millis = bd[18];

    // upper 4 bits contain additional compression block count data that is ignored
    let compression_type = bd[20] & 0xF;

    header.compression_type = match compression_type {
        0 => TFCompressionType::TF_COMPRESSION_NONE,
        1 => TFCompressionType::TF_COMPRESSION_ZSTD,
        2 => TFCompressionType::TF_COMPRESSION_ZLIB,
        _ => return TFError::TF_EINVALID_COMPRESSION_TYPE,
    };

    header.compression_block_count = bd[21];
    header.channel_range_count = bd[22];

    header.sequence_uid = u64::from_le_bytes([
        bd[24], bd[25], bd[26], bd[27], bd[28], bd[29], bd[30], bd[31],
    ]);

    if let Some(ep) = ep {
        *ep = &bd[HEADER_SIZE..];
    }

    TFError::TF_OK
}
pub fn tf_compression_block_read<'a>(
    bd: &'a [u8],
    block: &mut TFCompressionBlock,
    ep: Option<&mut &'a [u8]>,
) -> TFError {
    const COMPRESSION_BLOCK_SIZE: usize = 8;

    if bd.len() < COMPRESSION_BLOCK_SIZE {
        return TFError::TF_EINVALID_BUFFER_SIZE;
    }

    block.first_frame_id = u32::from_le_bytes([bd[0], bd[1], bd[2], bd[3]]);
    block.size = u32::from_le_bytes([bd[4], bd[5], bd[6], bd[7]]);

    if let Some(ep) = ep {
        *ep = &bd[COMPRESSION_BLOCK_SIZE..];
    }

    TFError::TF_OK
}

fn tf_uint24_read(bd: &[u8]) -> u32 {
    (bd[0] as u32) | ((bd[1] as u32) << 8) | ((bd[2] as u32) << 16)
}

pub fn tf_channel_range_read<'a>(
    bd: &'a [u8],
    channel_range: &mut TFChannelRange,
    ep: Option<&mut &'a [u8]>,
) -> TFError {
    const CHANNEL_RANGE_SIZE: usize = 6;

    if bd.len() < CHANNEL_RANGE_SIZE {
        return TFError::TF_EINVALID_BUFFER_SIZE;
    }

    channel_range.first_channel_number = tf_uint24_read(&bd[0..3]);
    channel_range.channel_count = tf_uint24_read(&bd[3..6]);

    if let Some(ep) = ep {
        *ep = &bd[CHANNEL_RANGE_SIZE..];
    }

    TFError::TF_OK
}
