/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let insize = input.len();
    let reqsize = (insize + 14) / 15 + insize;
    let mut out = Vec::with_capacity(reqsize);
    let mut pos = 0;

    while pos + 15 <= insize {
        let block: &[u8; 15] = input[pos..pos + 15].try_into().unwrap();
        hecc_encode_impl(block, &mut out);
        pos += 15;
    }

    if pos < insize {
        let mut buf = [0u8; 15];
        let remaining = insize - pos;
        buf[..remaining].copy_from_slice(&input[pos..]);
        let before = out.len();
        hecc_encode_impl(&buf, &mut out);
        let check = out[before + 15];
        out[before + remaining] = check;
        out.truncate(before + remaining + 1);
    }

    out
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let mut inpos = 0;
    let insize = 15 * 8;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[inpos];
            output.push(data);
            inpos += 1;
            p2 ^= (data.count_ones() & 1) as u8;
        }
        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;
        j += 1;
        i += 1;
    }

    p2 ^= (p1.count_ones() & 1) as u8;
    output.push(p1 + (p2 << 7));
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let insize = input.len();

    if insize % 16 == 1 {
        return None;
    }

    let reqsize = insize - (insize / 16 + if insize % 16 != 0 { 1 } else { 0 });
    let mut out = Vec::with_capacity(reqsize);
    let mut pos = 0;

    while pos + 16 <= insize {
        let block: &[u8; 16] = input[pos..pos + 16].try_into().unwrap();
        if !hecc_decode_impl(block, &mut out) {
            return None;
        }
        pos += 16;
    }

    let remaining = insize - pos;
    if remaining > 0 {
        let mut buf = [0u8; 16];
        buf[..remaining - 1].copy_from_slice(&input[pos..pos + remaining - 1]);
        buf[15] = input[pos + remaining - 1];
        let mut tmp = Vec::new();
        if !hecc_decode_impl(&buf, &mut tmp) {
            return None;
        }
        out.extend_from_slice(&tmp[..remaining - 1]);
    }

    Some(out)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let data_len: usize = 15;
    let check = input[data_len];
    let start = output.len();

    for i in 0..data_len {
        output.push(input[i]);
    }

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let mut inpos = 0;
    let bit_count = data_len * 8;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < bit_count {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[inpos];
            inpos += 1;
            p2 ^= (data.count_ones() & 1) as u8;
        }
        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;
        j += 1;
        i += 1;
    }

    p1 ^= check & 0x7f;

    if p1 != 0 {
        let pm = check;
        p2 ^= (pm.count_ones() & 1) as u8;

        if p2 == 0 {
            return false;
        }

        if (p1 & (p1 - 1)) != 0 {
            let error_pos = p1 - log2uint8(p1) - 2;
            let byte_idx = (error_pos / 8) as usize;
            let bit_idx = error_pos % 8;
            output[start + byte_idx] ^= 1 << bit_idx;
        }
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    7 - x.leading_zeros() as u8
}
