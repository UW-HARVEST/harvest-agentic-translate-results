pub const FALSE: u8 = 0;
pub const TRUE: u8 = 1;
pub const UNSIGNED: u8 = 2;
pub const SIGNED: u8 = 3;
pub const INTEL: u8 = 4;
pub const MOTOROLA: u8 = 5;

/// Compute byte indexes of msb, lsb and the lsb bit offset from signal params.
/// Returns `(offset_lsb, byte_idx_lsb, byte_idx_msb)`.
fn compute_indexes(startbit: u16, len: u8, endianness: u8) -> (u16, u16, u16) {
    if endianness == MOTOROLA {
        let msb: u16 = startbit;
        let mut current_bit_nr: i32 = startbit as i32;
        let iters = (len as i32) - 1;
        let mut i = 0;
        while i < iters {
            if current_bit_nr % 8 == 0 {
                current_bit_nr += 15;
            } else {
                current_bit_nr -= 1;
            }
            i += 1;
        }
        let lsb = current_bit_nr as u16;
        (lsb % 8, lsb / 8, msb / 8)
    } else {
        let lsb = startbit;
        let msb = lsb + (len as u16) - 1;
        (lsb % 8, lsb / 8, msb / 8)
    }
}

/// Copy 8-byte window of frame data containing the signal into a u64 (little-endian).
/// For Motorola endianness, bytes are reversed (copy and swap).
fn frame_to_local(frame: &[u8], byte_idx_lsb: u16, byte_idx_msb: u16, endianness: u8) -> u64 {
    let mut bytes = [0u8; 8];
    bytes[0] = frame[byte_idx_lsb as usize];
    let mut dest_idx: usize = 0;
    if endianness == MOTOROLA {
        let mut i: i32 = byte_idx_lsb as i32 - 1;
        while i >= byte_idx_msb as i32 {
            dest_idx += 1;
            if dest_idx >= 8 {
                break;
            }
            bytes[dest_idx] = frame[i as usize];
            i -= 1;
        }
    } else {
        let mut i: u16 = byte_idx_lsb + 1;
        while i <= byte_idx_msb {
            dest_idx += 1;
            if dest_idx >= 8 {
                break;
            }
            bytes[dest_idx] = frame[i as usize];
            i += 1;
        }
    }
    u64::from_le_bytes(bytes)
}

/// Extract a signal value from a CAN frame.
///
/// * `frame`      – byte slice of the frame data.
/// * `startbit`   – signal start bit.
/// * `len`        – signal length (in bits).
/// * `signedness` – either UNSIGNED or SIGNED.
/// * `endianness` – either INTEL (little-endian) or MOTOROLA (big-endian).
///
/// Returns a u64 value (if the signal is signed, it is already sign‐extended).
pub fn extract(frame: &[u8], startbit: u16, len: u8, signedness: u8, endianness: u8) -> u64 {
    let (offset_lsb, byte_idx_lsb, byte_idx_msb) = compute_indexes(startbit, len, endianness);
    let mut target = frame_to_local(frame, byte_idx_lsb, byte_idx_msb, endianness);

    /* remove lower bits not in signal (lower than offset lsb) */
    target >>= offset_lsb as u32;

    /* clear higher bits not in signal (higher than msb) */
    let mask: u64 = if len >= 64 {
        u64::MAX
    } else {
        (1u64 << (len as u32)) - 1
    };
    target &= mask;

    /* sign extension */
    if signedness == SIGNED && len > 0 {
        let sign_bit = 1u64 << ((len - 1) as u32);
        if sign_bit & target != 0 {
            let sign_ext_mask = !mask;
            target |= sign_ext_mask;
        }
    }
    target
}

/// Insert a signal value into a CAN frame.
///
/// * `frame`     – mutable byte slice.
/// * `startbit`  – signal start bit.
/// * `len`       – signal length in bits.
/// * `can_value` – signal value (as raw bits).
/// * `endianness`– INTEL or MOTOROLA.
pub fn insert(frame: &mut [u8], startbit: u16, len: u8, can_value: u64, endianness: u8) {
    let (offset_lsb, byte_idx_lsb, byte_idx_msb) = compute_indexes(startbit, len, endianness);
    let mut target = frame_to_local(frame, byte_idx_lsb, byte_idx_msb, endianness);

    let mask: u64 = if len >= 64 {
        u64::MAX
    } else {
        (1u64 << (len as u32)) - 1
    };
    let erase_mask = !(mask << (offset_lsb as u32));

    /* erase and insert signal value into local data chunk */
    target = (target & erase_mask) | (can_value << (offset_lsb as u32));

    let bytes = target.to_le_bytes();
    let mut src_idx: usize = 0;
    if endianness == MOTOROLA {
        let mut i: i32 = byte_idx_lsb as i32;
        while i >= byte_idx_msb as i32 {
            if src_idx >= 8 {
                break;
            }
            frame[i as usize] = bytes[src_idx];
            src_idx += 1;
            i -= 1;
        }
    } else {
        let mut i: u16 = byte_idx_lsb;
        while i <= byte_idx_msb {
            if src_idx >= 8 {
                break;
            }
            frame[i as usize] = bytes[src_idx];
            src_idx += 1;
            i += 1;
        }
    }
}

/// Encode an unsigned 64‑bit physical value into the CAN frame.
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

/// Encode a signed 64‑bit physical value into the CAN frame.
pub fn encode_int64_t(
    frame: &mut [u8],
    phy_value: i64,
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) {
    let can_value = (((phy_value as f64) - offset) / factor) as i64 as u64;
    insert(frame, startbit, len, can_value, endianness);
}

/// Encode a double‑precision (f64) physical value.
pub fn encode_double(
    frame: &mut [u8],
    phy_value: f64,
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) {
    let can_value = ((phy_value - offset) / factor) as i64 as u64;
    insert(frame, startbit, len, can_value, endianness);
}

/// Encode a single‑precision (f32) physical value.
pub fn encode_float(
    frame: &mut [u8],
    phy_value: f32,
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) {
    let can_value = ((phy_value as f64 - offset) / factor) as i64 as u64;
    insert(frame, startbit, len, can_value, endianness);
}

/// Decode an unsigned 64‑bit value from the CAN frame.
pub fn decode_uint64_t(
    frame: &[u8],
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) -> u64 {
    let can_value = extract(frame, startbit, len, UNSIGNED, endianness);
    ((can_value as f64) * factor + offset) as u64
}

/// Decode a signed 64‑bit value from the CAN frame.
pub fn decode_int64_t(
    frame: &[u8],
    startbit: u16,
    len: u8,
    endianness: u8,
    factor: f64,
    offset: f64,
) -> i64 {
    let can_value = extract(frame, startbit, len, SIGNED, endianness) as i64;
    ((can_value as f64) * factor + offset) as i64
}

/// Decode a double‑precision (f64) physical value.
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

/// Decode a single‑precision (f32) physical value.
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
