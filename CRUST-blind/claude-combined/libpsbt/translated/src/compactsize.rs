use crate::psbt::{PsbtResult, MAX_SERIALIZE_SIZE};

/// Returns the length in bytes of a compactsize whose first byte is `chsize`.
pub fn compactsize_peek_length(chsize: u8) -> u32 {
    if chsize < 253 {
        1 // sizeof(u8)
    } else if chsize == 253 {
        1 + 2 // sizeof(u8) + sizeof(u16)
    } else if chsize == 254 {
        1 + 4 // sizeof(u8) + sizeof(u32)
    } else {
        1 + 8 // sizeof(u8) + sizeof(u64)
    }
}

/// Returns the encoded length in bytes of `data` as a compactsize.
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

/// Write a compactsize-encoded value into `dest` starting at offset 0.
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

/// Read a compactsize-encoded value from `data`. Returns (value, result).
pub fn compactsize_read(data: &[u8]) -> (u64, PsbtResult) {
    if data.is_empty() {
        return (0, PsbtResult::CompactReadError);
    }
    let chsize = data[0];
    let ret_size: u64 = if chsize < 253 {
        chsize as u64
    } else if chsize == 253 {
        if data.len() < 3 {
            return (0, PsbtResult::CompactReadError);
        }
        let v = u16::from_le_bytes([data[1], data[2]]) as u64;
        if v < 253 {
            return (0, PsbtResult::CompactReadError);
        }
        v
    } else if chsize == 254 {
        if data.len() < 5 {
            return (0, PsbtResult::CompactReadError);
        }
        let v = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as u64;
        if v < 0x10000u64 {
            return (0, PsbtResult::CompactReadError);
        }
        v
    } else {
        if data.len() < 9 {
            return (0, PsbtResult::CompactReadError);
        }
        let v = u64::from_le_bytes([
            data[1], data[2], data[3], data[4],
            data[5], data[6], data[7], data[8],
        ]);
        if v < 0x100000000u64 {
            return (0, PsbtResult::CompactReadError);
        }
        v
    };

    if ret_size > MAX_SERIALIZE_SIZE as u64 {
        return (0, PsbtResult::CompactReadError);
    }

    (ret_size, PsbtResult::Ok)
}
