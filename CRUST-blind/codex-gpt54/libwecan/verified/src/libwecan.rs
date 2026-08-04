pub const FALSE: u8 = 0;
pub const TRUE: u8 = 1;
pub const UNSIGNED: u8 = 2;
pub const SIGNED: u8 = 3;
pub const INTEL: u8 = 4;
pub const MOTOROLA: u8 = 5;

fn compute_indexes(startbit: u16, len: u8, endianness: u8) -> (u16, u16, u16) {
    if endianness == MOTOROLA {
        let msb = startbit;
        let mut current_bit_nr = startbit;

        for _ in 0..len.saturating_sub(1) {
            if current_bit_nr % 8 == 0 {
                current_bit_nr = current_bit_nr.wrapping_add(15);
            } else {
                current_bit_nr = current_bit_nr.wrapping_sub(1);
            }
        }

        let lsb = current_bit_nr;
        (lsb % 8, lsb / 8, msb / 8)
    } else {
        let lsb = startbit;
        let msb = lsb.wrapping_add(len.saturating_sub(1) as u16);
        (lsb % 8, lsb / 8, msb / 8)
    }
}

fn frame_to_local(frame: &[u8], byte_idx_lsb: u16, byte_idx_msb: u16, endianness: u8) -> u64 {
    let mut bytes = [0u8; 8];
    bytes[0] = frame[byte_idx_lsb as usize];

    if endianness == MOTOROLA {
        for (dest_idx, src_idx) in ((byte_idx_msb..byte_idx_lsb).rev()).enumerate() {
            bytes[dest_idx + 1] = frame[src_idx as usize];
        }
    } else {
        for (dest_idx, src_idx) in ((byte_idx_lsb + 1)..=byte_idx_msb).enumerate() {
            bytes[dest_idx + 1] = frame[src_idx as usize];
        }
    }

    u64::from_le_bytes(bytes)
}

fn write_local_to_frame(
    frame: &mut [u8],
    byte_idx_lsb: u16,
    byte_idx_msb: u16,
    endianness: u8,
    target: u64,
) {
    let bytes = target.to_le_bytes();

    if endianness == MOTOROLA {
        for (src_idx, dst_idx) in (byte_idx_msb..=byte_idx_lsb).rev().enumerate() {
            frame[dst_idx as usize] = bytes[src_idx];
        }
    } else {
        for (src_idx, dst_idx) in (byte_idx_lsb..=byte_idx_msb).enumerate() {
            frame[dst_idx as usize] = bytes[src_idx];
        }
    }
}

fn bitmask(len: u8) -> u64 {
    match len {
        0 => 0,
        64.. => u64::MAX,
        _ => (1u64 << len) - 1,
    }
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
    if len == 0 {
        return 0;
    }

    let (offset_lsb, byte_idx_lsb, byte_idx_msb) = compute_indexes(startbit, len, endianness);
    let mut target = frame_to_local(frame, byte_idx_lsb, byte_idx_msb, endianness);

    target >>= offset_lsb;

    let mask = bitmask(len);
    target &= mask;

    if signedness == SIGNED && len < 64 && (target & (1u64 << (len - 1))) != 0 {
        target |= !mask;
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
    if len == 0 {
        return;
    }

    let (offset_lsb, byte_idx_lsb, byte_idx_msb) = compute_indexes(startbit, len, endianness);
    let mut target = frame_to_local(frame, byte_idx_lsb, byte_idx_msb, endianness);

    let mask = bitmask(len);
    let erase_mask = !(mask << offset_lsb);

    target = (target & erase_mask) | (can_value << offset_lsb);

    write_local_to_frame(frame, byte_idx_lsb, byte_idx_msb, endianness, target);
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
    let can_value = (((phy_value as f64) - offset) / factor) as u64;
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
    let can_value = ((((phy_value as f64) - offset) / factor) as i64) as u64;
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
    let can_value = (((phy_value - offset) / factor) as i64) as u64;
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
    let can_value = ((((phy_value as f64) - offset) / factor) as i64) as u64;
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
