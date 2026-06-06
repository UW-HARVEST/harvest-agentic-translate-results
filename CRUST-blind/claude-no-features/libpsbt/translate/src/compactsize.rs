use crate::psbt::PsbtResult;

pub fn compactsize_read(data: &[u8]) -> (u64, PsbtResult) {
    if data.is_empty() {
        return (0, PsbtResult::CompactReadError);
    }
    let chsize = data[0];
    let ret_size: u64;

    if chsize < 253 {
        ret_size = chsize as u64;
    } else if chsize == 253 {
        if data.len() < 3 {
            return (0, PsbtResult::CompactReadError);
        }
        let mut bytes = [0u8; 2];
        bytes.copy_from_slice(&data[1..3]);
        ret_size = u16::from_le_bytes(bytes) as u64;
        if ret_size < 253 {
            return (0, PsbtResult::CompactReadError);
        }
    } else if chsize == 254 {
        if data.len() < 5 {
            return (0, PsbtResult::CompactReadError);
        }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&data[1..5]);
        ret_size = u32::from_le_bytes(bytes) as u64;
        if ret_size < 0x10000 {
            return (0, PsbtResult::CompactReadError);
        }
    } else {
        if data.len() < 9 {
            return (0, PsbtResult::CompactReadError);
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[1..9]);
        ret_size = u64::from_le_bytes(bytes);
        if ret_size < 0x100000000u64 {
            return (0, PsbtResult::CompactReadError);
        }
    }

    if ret_size > crate::psbt::MAX_SERIALIZE_SIZE as u64 {
        return (0, PsbtResult::CompactReadError);
    }

    (ret_size, PsbtResult::Ok)
}

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
