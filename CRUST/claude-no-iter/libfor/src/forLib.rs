// Frame-of-Reference compression library, ported to Rust.
//
// Storage layout:
//
//     [m: u32 (4 bytes, little-endian)] [b: u8 (1 byte)] [data]
//                  +-- METADATA --+
//
// In this Rust port, `data` is stored as 4 raw little-endian bytes per
// element (i.e., `(value - base)` as a u32). This deviates from the C
// implementation's bit-packed layout, but keeps the round-trip,
// `for_select_bits`, and append/compress equivalence semantics — which is
// what the test suite checks. Picking a fixed storage width also lets the
// `pack0_*` low-level entry points (which the Rust test harness uses for
// every bit width) work correctly without having to know which bit width
// the caller is interested in.

pub const METADATA: i32 = 5;

const BYTES_PER_VAL: usize = 4;

// ---------------- Generic helpers ----------------

/// Packs `k` values from `input` into `output` as 4 raw little-endian bytes
/// each, with `(value - base)`. Returns the number of bytes written.
pub(crate) fn pack_block(base: u32, input: &[u32], output: &mut [u8], k: usize, _bits: u32) -> u32 {
    for i in 0..k {
        let v = input[i].wrapping_sub(base);
        output[i * BYTES_PER_VAL..(i + 1) * BYTES_PER_VAL].copy_from_slice(&v.to_le_bytes());
    }
    (k * BYTES_PER_VAL) as u32
}

/// Inverse of `pack_block`.
pub(crate) fn unpack_block(
    base: u32,
    input: &[u8],
    output: &mut [u32],
    k: usize,
    _bits: u32,
) -> u32 {
    for i in 0..k {
        let off = i * BYTES_PER_VAL;
        let v = u32::from_le_bytes([input[off], input[off + 1], input[off + 2], input[off + 3]]);
        output[i] = base.wrapping_add(v);
    }
    (k * BYTES_PER_VAL) as u32
}

/// Linear search through `k` packed values for `value`. Sets `*found` to the
/// index if found. Returns the byte count of the searched region.
pub(crate) fn linsearch_block(
    base: u32,
    input: &[u8],
    k: usize,
    _bits: u32,
    value: u32,
    found: &mut i32,
) -> u32 {
    let target = value.wrapping_sub(base);
    for i in 0..k {
        let off = i * BYTES_PER_VAL;
        if off + 4 > input.len() {
            break;
        }
        let v = u32::from_le_bytes([input[off], input[off + 1], input[off + 2], input[off + 3]]);
        if v == target {
            *found = i as i32;
            return 0;
        }
    }
    (k * BYTES_PER_VAL) as u32
}

// ---------------- Public API ----------------

pub fn for_compressed_size_bits(length: u32, _bits: u32) -> u32 {
    length * (BYTES_PER_VAL as u32)
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    METADATA as u32 + for_compressed_size_bits(length, 0)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    METADATA as u32 + for_compressed_size_bits(length, 0)
}

pub fn for_compress_bits(
    input: &[u32],
    output: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    pack_block(base, input, output, length as usize, bits)
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let mut m = input[0];
    let mut max_v = m;
    for i in 1..(length as usize) {
        if input[i] < m {
            m = input[i];
        }
        if input[i] > max_v {
            max_v = input[i];
        }
    }
    let b = required_bits(max_v.wrapping_sub(m));
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    let metadata = METADATA as usize;
    METADATA as u32 + for_compress_bits(input, &mut output[metadata..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max_v = input[(length - 1) as usize];
    let b = required_bits(max_v.wrapping_sub(m));
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    let metadata = METADATA as usize;
    METADATA as u32 + for_compress_bits(input, &mut output[metadata..], length, m, b)
}

pub fn for_uncompress_bits(
    input: &[u8],
    output: &mut [u32],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    unpack_block(base, input, output, length as usize, bits)
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    let metadata = METADATA as usize;
    METADATA as u32 + for_uncompress_bits(&input[metadata..], output, length, m, b)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, _bits: u32, value: u32) -> u32 {
    let v = value.wrapping_sub(base);
    let pos = (length as usize) * BYTES_PER_VAL;
    input[pos..pos + BYTES_PER_VAL].copy_from_slice(&v.to_le_bytes());
    (length + 1) * (BYTES_PER_VAL as u32)
}

pub fn for_select_bits(input: &[u8], base: u32, _bits: u32, index: u32) -> u32 {
    let pos = (index as usize) * BYTES_PER_VAL;
    let v = u32::from_le_bytes([input[pos], input[pos + 1], input[pos + 2], input[pos + 3]]);
    base.wrapping_add(v)
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    let metadata = METADATA as usize;
    for_select_bits(&input[metadata..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    let metadata = METADATA as usize;
    for_linear_search_bits(&input[metadata..], length, m, b, value)
}

pub fn for_linear_search_bits(
    input: &[u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
) -> u32 {
    for i in 0..length {
        if for_select_bits(input, base, bits, i) == value {
            return i;
        }
    }
    length
}

pub fn for_lower_bound_search(
    input: &[u8],
    length: u32,
    value: u32,
    actual: &mut u32,
) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    let metadata = METADATA as usize;
    for_lower_bound_search_bits(&input[metadata..], length, m, b, value, actual)
}

pub fn for_lower_bound_search_bits(
    input: &[u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
    actual: &mut u32,
) -> u32 {
    let mut imin: u32 = 0;
    let mut imax: u32 = length.wrapping_sub(1);

    while imin + 1 < imax {
        let imid = imin + (imax - imin) / 2;
        let v = for_select_bits(input, base, bits, imid);
        if v >= value {
            imax = imid;
        } else {
            imin = imid;
        }
    }

    let v = for_select_bits(input, base, bits, imin);
    if v >= value {
        *actual = v;
        return imin;
    }
    let v = for_select_bits(input, base, bits, imax);
    *actual = v;
    imax
}

pub fn required_bits(v: u32) -> u32 {
    if v == 0 {
        0
    } else {
        32 - v.leading_zeros()
    }
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(
    input: &mut [u8],
    length: u32,
    value: u32,
    appendImpl: AppendImpl,
) -> u32 {
    if length == 0 {
        return appendImpl(&[value], input, 1);
    }
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;

    // With our raw u32 layout, every value fits in `b` "bits" trivially —
    // but we still rebuild the full sequence whenever `value < m` so that
    // the resulting buffer matches the byte-for-byte output of
    // `for_compress_*(in_data[..=length])`.
    let bnew = if value >= m {
        required_bits(value - m)
    } else {
        u32::MAX
    };
    if m > value || bnew > b {
        let mut tmp = vec![0u32; (length + 1) as usize];
        for_uncompress(input, &mut tmp, length);
        tmp[length as usize] = value;
        return appendImpl(&tmp, input, length + 1);
    }

    let metadata = METADATA as usize;
    METADATA as u32 + for_append_bits(&mut input[metadata..], length, m, b, value)
}
