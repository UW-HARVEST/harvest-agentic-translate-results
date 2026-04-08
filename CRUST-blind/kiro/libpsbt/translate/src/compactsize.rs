use crate::psbt::{PsbtResult, MAX_SERIALIZE_SIZE};

pub fn compactsize_peek_length(_chsize: u8) -> u32 {
    if _chsize < 253 {
        1 // sizeof(u8)
    } else if _chsize == 253 {
        1 + 2 // sizeof(u8) + sizeof(u16)
    } else if _chsize == 254 {
        1 + 4 // sizeof(u8) + sizeof(u32)
    } else {
        1 + 8 // sizeof(u8) + sizeof(u64)
    }
}

pub fn compactsize_length(_data: u64) -> u32 {
    if _data < 253 {
        1
    } else if _data <= u16::MAX as u64 {
        1 + 2
    } else if _data <= u32::MAX as u64 {
        1 + 4
    } else {
        1 + 8
    }
}

pub fn compactsize_write(_dest: &mut [u8], _size: u64) {
    if _size < 253 {
        _dest[0] = _size as u8;
    } else if _size <= u16::MAX as u64 {
        _dest[0] = 253;
        _dest[1..3].copy_from_slice(&(_size as u16).to_le_bytes());
    } else if _size <= u32::MAX as u64 {
        _dest[0] = 254;
        _dest[1..5].copy_from_slice(&(_size as u32).to_le_bytes());
    } else {
        _dest[0] = 255;
        _dest[1..9].copy_from_slice(&_size.to_le_bytes());
    }
}

pub fn compactsize_read(_data: &[u8]) -> (u64, PsbtResult) {
    let chsize = _data[0];
    let ret_size: u64;

    if chsize < 253 {
        ret_size = chsize as u64;
    } else if chsize == 253 {
        ret_size = u16::from_le_bytes([_data[1], _data[2]]) as u64;
        if ret_size < 253 {
            return (u64::MAX, PsbtResult::CompactReadError);
        }
    } else if chsize == 254 {
        ret_size = u32::from_le_bytes([_data[1], _data[2], _data[3], _data[4]]) as u64;
        if ret_size < 0x10000 {
            return (u64::MAX, PsbtResult::CompactReadError);
        }
    } else {
        ret_size = u64::from_le_bytes([
            _data[1], _data[2], _data[3], _data[4], _data[5], _data[6], _data[7], _data[8],
        ]);
        if ret_size < 0x100000000 {
            return (u64::MAX, PsbtResult::CompactReadError);
        }
    }

    if ret_size > MAX_SERIALIZE_SIZE as u64 {
        return (u64::MAX, PsbtResult::CompactReadError);
    }

    (ret_size, PsbtResult::Ok)
}
