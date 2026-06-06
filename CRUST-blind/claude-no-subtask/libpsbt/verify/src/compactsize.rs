use crate::psbt::{PsbtResult, MAX_SERIALIZE_SIZE};

/// Read a compactsize-encoded value from `data`. Returns the value and a result
/// status indicating whether the read was successful.
pub fn compactsize_read(data: &[u8]) -> (u64, PsbtResult) {
    if data.is_empty() {
        return (0, PsbtResult::CompactReadError);
    }
    let chsize = data[0];
    let ret_size: u64;

    if chsize < 253 {
        ret_size = chsize as u64;
    } else if chsize == 253 {
        if data.len() < 1 + 2 {
            return (0, PsbtResult::CompactReadError);
        }
        let v = u16::from_le_bytes([data[1], data[2]]) as u64;
        if v < 253 {
            return (0, PsbtResult::CompactReadError);
        }
        ret_size = v;
    } else if chsize == 254 {
        if data.len() < 1 + 4 {
            return (0, PsbtResult::CompactReadError);
        }
        let v = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as u64;
        if v < 0x10000 {
            return (0, PsbtResult::CompactReadError);
        }
        ret_size = v;
    } else {
        if data.len() < 1 + 8 {
            return (0, PsbtResult::CompactReadError);
        }
        let v = u64::from_le_bytes([
            data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
        ]);
        if v < 0x1_0000_0000 {
            return (0, PsbtResult::CompactReadError);
        }
        ret_size = v;
    }

    if ret_size > MAX_SERIALIZE_SIZE as u64 {
        return (0, PsbtResult::CompactReadError);
    }

    (ret_size, PsbtResult::Ok)
}

/// Number of bytes a compactsize-encoded value would consume.
pub fn compactsize_length(data: u64) -> u32 {
    if data < 253 {
        1
    } else if data <= u16::MAX as u64 {
        1 + 2
    } else if data <= u32::MAX as u64 {
        1 + 4
    } else {
        1 + 8
    }
}

/// Write a compactsize-encoded value to `dest`.
pub fn compactsize_write(dest: &mut [u8], size: u64) {
    if size < 253 {
        dest[0] = size as u8;
    } else if size <= u16::MAX as u64 {
        dest[0] = 253;
        let bytes = (size as u16).to_le_bytes();
        dest[1..3].copy_from_slice(&bytes);
    } else if size <= u32::MAX as u64 {
        dest[0] = 254;
        let bytes = (size as u32).to_le_bytes();
        dest[1..5].copy_from_slice(&bytes);
    } else {
        dest[0] = 255;
        let bytes = size.to_le_bytes();
        dest[1..9].copy_from_slice(&bytes);
    }
}

/// Number of bytes a compactsize value would consume given the first byte.
pub fn compactsize_peek_length(chsize: u8) -> u32 {
    if chsize < 253 {
        1
    } else if chsize == 253 {
        1 + 2
    } else if chsize == 254 {
        1 + 4
    } else {
        1 + 8
    }
}
