pub const FALSE: u8 = 0;
pub const TRUE: u8 = 1;
pub const UNSIGNED: u8 = 2;
pub const SIGNED: u8 = 3;
pub const INTEL: u8 = 4;
pub const MOTOROLA: u8 = 5;

fn mask_for_len(len: u8) -> u64 {
    if len >= 64 {
        u64::MAX
    } else if len == 0 {
        0
    } else {
        (1u64 << len) - 1
    }
}

fn compute_indexes(startbit: u16, len: u8, endianness: u8) -> (u16, u16, u16) {
    if len == 0 {
        let byte_idx = startbit / 8;
        return (startbit % 8, byte_idx, byte_idx);
    }

    if endianness == MOTOROLA {
        let msb = startbit;
        let mut current_bit_nr = startbit;

        for _ in 0..usize::from(len.saturating_sub(1)) {
            if current_bit_nr % 8 == 0 {
                current_bit_nr = current_bit_nr.saturating_add(15);
            } else {
                current_bit_nr -= 1;
            }
        }

        let lsb = current_bit_nr;
        (lsb % 8, lsb / 8, msb / 8)
    } else {
        let lsb = startbit;
        let msb = lsb.saturating_add(u16::from(len.saturating_sub(1)));
        (lsb % 8, lsb / 8, msb / 8)
    }
}

fn frame_to_local(frame: &[u8], byte_idx_lsb: u16, byte_idx_msb: u16, endianness: u8) -> u64 {
    let mut bytes = [0u8; 8];
    bytes[0] = frame[usize::from(byte_idx_lsb)];

    if endianness == MOTOROLA {
        for (dest_idx, src_idx) in (byte_idx_msb..byte_idx_lsb).rev().enumerate() {
            bytes[dest_idx + 1] = frame[usize::from(src_idx)];
        }
    } else {
        for (dest_idx, src_idx) in ((byte_idx_lsb + 1)..=byte_idx_msb).enumerate() {
            bytes[dest_idx + 1] = frame[usize::from(src_idx)];
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

    target >>= offset_lsb;

    let mask = mask_for_len(len);
    target &= mask;

    if signedness == SIGNED && len > 0 {
        let sign_bit = if len >= 64 {
            1u64 << 63
        } else {
            1u64 << (len - 1)
        };

        if target & sign_bit != 0 {
            target |= !mask;
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

    let mask = mask_for_len(len);
    let shifted_mask = mask.checked_shl(u32::from(offset_lsb)).unwrap_or(0);
    let erase_mask = !shifted_mask;

    target = (target & erase_mask) | can_value.wrapping_shl(u32::from(offset_lsb));

    let bytes = target.to_le_bytes();
    if endianness == MOTOROLA {
        for (src_idx, dst_idx) in (byte_idx_msb..=byte_idx_lsb).rev().enumerate() {
            frame[usize::from(dst_idx)] = bytes[src_idx];
        }
    } else {
        for (src_idx, dst_idx) in (byte_idx_lsb..=byte_idx_msb).enumerate() {
            frame[usize::from(dst_idx)] = bytes[src_idx];
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
    let can_value = ((phy_value as f64 - offset) / factor) as i64 as u64;
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
    let can_value = ((f64::from(phy_value) - offset) / factor) as i64 as u64;
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
    ((can_value as f64 * factor) + offset) as u64
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
    ((can_value as f64 * factor) + offset) as i64
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
    (can_value as f64 * factor) + offset
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
    ((can_value as f64 * factor) + offset) as f32
}
