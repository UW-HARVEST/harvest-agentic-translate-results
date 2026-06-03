pub const FALSE: u8 = 0;
pub const TRUE: u8 = 1;
pub const UNSIGNED: u8 = 2;
pub const SIGNED: u8 = 3;
pub const INTEL: u8 = 4;
pub const MOTOROLA: u8 = 5;

/// Return (offset_lsb, byte_idx_lsb, byte_idx_msb) for a signal.
fn compute_indexes(startbit: u16, len: u8, endianness: u8) -> (u16, usize, usize) {
    let msb: u16;
    let lsb: u16;

    if endianness == MOTOROLA {
        // big-endian: walk down `len - 1` bits from msb using DBC bit numbering
        msb = startbit;
        let mut current_bit_nr = startbit;
        for _ in 0..(len as u16).saturating_sub(1) {
            if current_bit_nr % 8 == 0 {
                current_bit_nr += 15;
            } else {
                current_bit_nr -= 1;
            }
        }
        lsb = current_bit_nr;
    } else {
        // little-endian
        lsb = startbit;
        msb = lsb + len as u16 - 1;
    }

    let byte_idx_lsb = (lsb / 8) as usize;
    let byte_idx_msb = (msb / 8) as usize;
    let offset_lsb = lsb % 8;
    (offset_lsb, byte_idx_lsb, byte_idx_msb)
}

/// Copy the byte chunk that contains the signal into a `u64` in
/// little-endian byte order. For MOTOROLA the bytes are reversed
/// during the copy so that the resulting `u64` has the LSB byte at
/// position 0.
fn frame_to_local(frame: &[u8], byte_idx_lsb: usize, byte_idx_msb: usize, endianness: u8) -> u64 {
    let mut target_bytes = [0u8; 8];
    target_bytes[0] = frame[byte_idx_lsb];
    let mut dest_idx = 1usize;

    if endianness == MOTOROLA {
        // big-endian: reverse copy from byte_idx_lsb-1 down to byte_idx_msb
        let mut i = byte_idx_lsb as isize - 1;
        while i >= byte_idx_msb as isize {
            target_bytes[dest_idx] = frame[i as usize];
            dest_idx += 1;
            i -= 1;
        }
    } else {
        // little-endian: simple copy from byte_idx_lsb+1 up to byte_idx_msb
        let mut i = byte_idx_lsb + 1;
        while i <= byte_idx_msb {
            target_bytes[dest_idx] = frame[i];
            dest_idx += 1;
            i += 1;
        }
    }

    u64::from_le_bytes(target_bytes)
}

/// Build a `len`-bit mask. Handles the `len == 64` case safely.
fn make_mask(len: u8) -> u64 {
    if len >= 64 {
        u64::MAX
    } else {
        (1u64 << len) - 1
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
    let (offset_lsb, byte_idx_lsb, byte_idx_msb) = compute_indexes(startbit, len, endianness);

    let mut target = frame_to_local(frame, byte_idx_lsb, byte_idx_msb, endianness);

    // remove lower bits not in signal (lower than offset lsb)
    target >>= offset_lsb;

    // clear higher bits not in signal (higher than msb)
    let mask = make_mask(len);
    target &= mask;

    // sign extension
    if signedness == SIGNED && len > 0 {
        if (1u64 << (len - 1)) & target != 0 {
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

    // create masks
    let mask = make_mask(len);
    let erase_mask = !(mask << offset_lsb);

    // erase and insert signal value into local data chunk
    target = (target & erase_mask) | ((can_value & mask) << offset_lsb);

    // copy back data chunk into the frame
    let target_bytes = target.to_le_bytes();
    let mut src_idx = 0usize;

    if endianness == MOTOROLA {
        let mut i = byte_idx_lsb as isize;
        while i >= byte_idx_msb as isize {
            frame[i as usize] = target_bytes[src_idx];
            src_idx += 1;
            i -= 1;
        }
    } else {
        let mut i = byte_idx_lsb;
        while i <= byte_idx_msb {
            frame[i] = target_bytes[src_idx];
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
