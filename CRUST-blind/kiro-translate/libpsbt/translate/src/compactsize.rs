use crate::psbt::{PsbtResult, MAX_SERIALIZE_SIZE};

pub fn compactsize_peek_length(chsize: u8) -> u32 {
    if chsize < 253 { 1 }
    else if chsize == 253 { 1 + 2 }
    else if chsize == 254 { 1 + 4 }
    else { 1 + 8 }
}

pub fn compactsize_length(data: u64) -> u32 {
    if data < 253 { 1 }
    else if data <= u16::MAX as u64 { 1 + 2 }
    else if data <= u32::MAX as u64 { 1 + 4 }
    else { 1 + 8 }
}

pub fn compactsize_write(dest: &mut [u8], size: u64) {
    if size < 253 {
        dest[0] = size as u8;
    } else if size <= u16::MAX as u64 {
        dest[0] = 253;
        dest[1..3].copy_from_slice(&(size as u16).to_le_bytes());
    } else if size <= u32::MAX as u64 {
        dest[0] = 254;
        dest[1..5].copy_from_slice(&(size as u32).to_le_bytes());
    } else {
        dest[0] = 255;
        dest[1..9].copy_from_slice(&size.to_le_bytes());
    }
}

pub fn compactsize_read(data: &[u8]) -> (u64, PsbtResult) {
    let chsize = data[0];
    let ret_size;

    if chsize < 253 {
        ret_size = chsize as u64;
    } else if chsize == 253 {
        ret_size = u16::from_le_bytes([data[1], data[2]]) as u64;
        if ret_size < 253 { return (u64::MAX, PsbtResult::CompactReadError); }
    } else if chsize == 254 {
        ret_size = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as u64;
        if ret_size < 0x10000 { return (u64::MAX, PsbtResult::CompactReadError); }
    } else {
        ret_size = u64::from_le_bytes([data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8]]);
        if ret_size < 0x100000000 { return (u64::MAX, PsbtResult::CompactReadError); }
    }

    if ret_size > MAX_SERIALIZE_SIZE as u64 {
        return (u64::MAX, PsbtResult::CompactReadError);
    }

    (ret_size, PsbtResult::Ok)
}
