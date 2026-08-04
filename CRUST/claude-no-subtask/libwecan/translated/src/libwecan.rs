pub const FALSE: u8 = 0;
pub const TRUE: u8 = 1;
pub const UNSIGNED: u8 = 2;
pub const SIGNED: u8 = 3;
pub const INTEL: u8 = 4;
pub const MOTOROLA: u8 = 5;

/// Compute the byte indexes of msb, lsb and the lsb bit offset within its byte.
///
/// Returns (offset_lsb, byte_idx_lsb, byte_idx_msb).
fn compute_indexes(startbit: u16, len: u8, endianness: u8) -> (u16, u16, u16) {
    if endianness == MOTOROLA {
        // big-endian: starting from msb, walk len-1 bits using the DBC bit
        // numbering convention (right to left within a byte, jumping from
        // bit_0 of current byte to bit_7 of the next byte)
        let msb: i32 = startbit as i32;
        let mut current_bit_nr: i32 = startbit as i32;

        for _ in 0..(len as i32 - 1) {
            if current_bit_nr % 8 == 0 {
                current_bit_nr += 15;
            } else {
                current_bit_nr -= 1;
            }
        }

        let lsb = current_bit_nr;
        let byte_idx_lsb = (lsb / 8) as u16;
        let byte_idx_msb = (msb / 8) as u16;
        let offset_lsb = (lsb % 8) as u16;
        (offset_lsb, byte_idx_lsb, byte_idx_msb)
    } else {
        // little-endian
        let lsb: u16 = startbit;
        let msb: u16 = lsb + len as u16 - 1;
        let byte_idx_msb = msb / 8;
        let byte_idx_lsb = lsb / 8;
        let offset_lsb = lsb % 8;
        (offset_lsb, byte_idx_lsb, byte_idx_msb)
    }
}

/// Copy the 8-byte chunk containing the signal into a local u64 in
/// little-endian byte ordering, taking the frame endianness into account.
fn frame_to_local(frame: &[u8], byte_idx_lsb: u16, byte_idx_msb: u16, endianness: u8) -> u64 {
    let mut target_bytes = [0u8; 8];
    target_bytes[0] = frame[byte_idx_lsb as usize];
    let mut dest_idx: usize = 1;

    if endianness == MOTOROLA {
        // big-endian: reverse-copy bytes from frame
        let mut i: i32 = byte_idx_lsb as i32 - 1;
        while i >= byte_idx_msb as i32 {
            if dest_idx >= 8 {
                break;
            }
            target_bytes[dest_idx] = frame[i as usize];
            dest_idx += 1;
            i -= 1;
        }
    } else {
        // little-endian: forward-copy bytes from frame
        let mut i: u16 = byte_idx_lsb + 1;
        while i <= byte_idx_msb {
            if dest_idx >= 8 {
                break;
            }
            target_bytes[dest_idx] = frame[i as usize];
            dest_idx += 1;
            i += 1;
        }
    }

    u64::from_le_bytes(target_bytes)
}

/// Build a `len`-bit mask. Handles `len == 64` (where `1u64 << 64` would
/// overflow) by returning `u64::MAX`.
fn make_mask(len: u8) -> u64 {
    if len >= 64 {
        u64::MAX
    } else {
        (1u64 << len) - 1
    }
}

/// Extract a signal value from a CAN frame.
pub fn extract(frame: &[u8], startbit: u16, len: u8, signedness: u8, endianness: u8) -> u64 {
    let (offset_lsb, byte_idx_lsb, byte_idx_msb) =
        compute_indexes(startbit, len, endianness);

    let mut target = frame_to_local(frame, byte_idx_lsb, byte_idx_msb, endianness);

    // remove lower bits not in signal
    target >>= offset_lsb;

    // clear higher bits not in signal
    let mask = make_mask(len);
    target &= mask;

    // sign extension when needed
    if signedness == SIGNED && len > 0 {
        if (1u64 << (len - 1)) & target != 0 {
            target |= !mask;
        }
    }
    target
}

/// Insert a signal value into a CAN frame.
pub fn insert(frame: &mut [u8], startbit: u16, len: u8, can_value: u64, endianness: u8) {
    let (offset_lsb, byte_idx_lsb, byte_idx_msb) =
        compute_indexes(startbit, len, endianness);

    let mut target = frame_to_local(frame, byte_idx_lsb, byte_idx_msb, endianness);

    let mask = make_mask(len);
    let erase_mask = !(mask << offset_lsb);

    // erase the bits where the signal lives, then OR in the value
    target = (target & erase_mask) | (can_value << offset_lsb);

    let src = target.to_le_bytes();

    if endianness == MOTOROLA {
        let mut i: i32 = byte_idx_lsb as i32;
        let mut idx: usize = 0;
        while i >= byte_idx_msb as i32 {
            if idx >= 8 {
                break;
            }
            frame[i as usize] = src[idx];
            idx += 1;
            i -= 1;
        }
    } else {
        let mut i: u16 = byte_idx_lsb;
        let mut idx: usize = 0;
        while i <= byte_idx_msb {
            if idx >= 8 {
                break;
            }
            frame[i as usize] = src[idx];
            idx += 1;
            i += 1;
        }
    }
}

/// Encode an unsigned 64-bit physical value into the CAN frame.
pub fn encode_uint64_t(
    frame: &mut [u8],
    phy_value: u64,
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) {
    let can_value = ((phy_value as f64 - offset) / factor) as u64;
    insert(frame, startbit, len, can_value, endianness);
}

/// Encode a signed 64-bit physical value into the CAN frame.
pub fn encode_int64_t(
    frame: &mut [u8],
    phy_value: i64,
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) {
    let can_value = (((phy_value as f64 - offset) / factor) as i64) as u64;
    insert(frame, startbit, len, can_value, endianness);
}

/// Encode a double-precision (f64) physical value.
pub fn encode_double(
    frame: &mut [u8],
    phy_value: f64,
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) {
    let can_value = (((phy_value - offset) / factor) as i64) as u64;
    insert(frame, startbit, len, can_value, endianness);
}

/// Encode a single-precision (f32) physical value.
pub fn encode_float(
    frame: &mut [u8],
    phy_value: f32,
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) {
    let can_value = (((phy_value as f64 - offset) / factor) as i64) as u64;
    insert(frame, startbit, len, can_value, endianness);
}

/// Decode an unsigned 64-bit value from the CAN frame.
pub fn decode_uint64_t(
    frame: &[u8],
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) -> u64 {
    let can_value = extract(frame, startbit, len, UNSIGNED, endianness);
    ((can_value as f64 * factor) + offset) as u64
}

/// Decode a signed 64-bit value from the CAN frame.
pub fn decode_int64_t(
    frame: &[u8],
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) -> i64 {
    let can_value = extract(frame, startbit, len, SIGNED, endianness) as i64;
    ((can_value as f64 * factor) + offset) as i64
}

/// Decode a double-precision (f64) physical value.
pub fn decode_double(
    frame: &[u8],
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) -> f64 {
    let can_value = extract(frame, startbit, len, SIGNED, endianness) as i64;
    (can_value as f64) * factor + offset
}

/// Decode a single-precision (f32) physical value.
pub fn decode_float(
    frame: &[u8],
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) -> f32 {
    let can_value = extract(frame, startbit, len, SIGNED, endianness) as i64;
    ((can_value as f64) * factor + offset) as f32
}
