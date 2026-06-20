use crate::psbt::PsbtResult;
use crate::psbt::MAX_SERIALIZE_SIZE;

pub fn compactsize_read(data: &[u8]) -> (u64, PsbtResult) {
    let Some(&chsize) = data.first() else {
        return (0, PsbtResult::CompactReadError);
    };

    let ret_size = if chsize < 253 {
        chsize as u64
    } else if chsize == 253 {
        if data.len() < 3 {
            return (0, PsbtResult::CompactReadError);
        }
        let ret = u16::from_le_bytes([data[1], data[2]]) as u64;
        if ret < 253 {
            return (0, PsbtResult::CompactReadError);
        }
        ret
    } else if chsize == 254 {
        if data.len() < 5 {
            return (0, PsbtResult::CompactReadError);
        }
        let ret = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as u64;
        if ret < 0x10000 {
            return (0, PsbtResult::CompactReadError);
        }
        ret
    } else {
        if data.len() < 9 {
            return (0, PsbtResult::CompactReadError);
        }
        let ret = u64::from_le_bytes([
            data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
        ]);
        if ret < 0x1_0000_0000 {
            return (0, PsbtResult::CompactReadError);
        }
        ret
    };

    if ret_size > MAX_SERIALIZE_SIZE as u64 {
        return (0, PsbtResult::CompactReadError);
    }

    (ret_size, PsbtResult::Ok)
}
pub fn compactsize_length(data: u64) -> u32 {
    if data < 253 {
        1
    } else if data <= u16::MAX as u64 {
        3
    } else if data <= u32::MAX as u64 {
        5
    } else {
        9
    }
}
pub fn compactsize_write(dest: &mut [u8], size: u64) {
    if size < 253 {
        if !dest.is_empty() {
            dest[0] = size as u8;
        }
    } else if size <= u16::MAX as u64 {
        if dest.len() >= 2 {
            let bytes = (size as u16).to_le_bytes();
            dest[0] = bytes[0];
            dest[1] = bytes[1];
        }
    } else if size <= u32::MAX as u64 {
        if dest.len() >= 4 {
            let bytes = (size as u32).to_le_bytes();
            dest[..4].copy_from_slice(&bytes);
        }
    } else if dest.len() >= 8 {
        let bytes = size.to_le_bytes();
        dest[..8].copy_from_slice(&bytes);
    }
}
pub fn compactsize_peek_length(chsize: u8) -> u32 {
    if chsize < 253 {
        1
    } else if chsize == 253 {
        3
    } else if chsize == 254 {
        5
    } else {
        9
    }
}
