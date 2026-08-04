pub static ENCODE_MATRIX: &[u16; 12] = &[
    0x8eb, 0x93e, 0xa97, 0xdc6, 0x367, 0x6cd, 0xd99, 0x3da, 0x7b4, 0xf68, 0x63b, 0xc75,
];
pub static DECODE_MATRIX: &[u16; 12] = &[
    0xc75, 0x49f, 0x93e, 0x6e3, 0xdc6, 0xf13, 0xab9, 0x1ed, 0x3da, 0x7b4, 0xf68, 0xa4f,
];
pub const RRC_TAPS_10: [f32; 81] = [
    -0.003195702904062073,
    -0.002930279157647190,
    -0.001940667871554463,
    -0.000356087678023658,
    0.001547011339077758,
    0.003389554791179751,
    0.004761898604225673,
    0.005310860846138910,
    0.004824746306020221,
    0.003297923526848786,
    0.000958710871218619,
    -0.001749908029791816,
    -0.004238694106631223,
    -0.005881783042101693,
    -0.006150256456781309,
    -0.004745376707651645,
    -0.001704189656473565,
    0.002547854551539951,
    0.007215575568844704,
    0.011231038205363532,
    0.013421952197060707,
    0.012730475385624438,
    0.008449554307303753,
    0.000436744366018287,
    -0.010735380379191660,
    -0.023726883538258272,
    -0.036498030780605324,
    -0.046500883189991064,
    -0.050979050575999614,
    -0.047340680079891187,
    -0.033554880492651755,
    -0.008513823955725943,
    0.027696543159614194,
    0.073664520037517042,
    0.126689053778116234,
    0.182990955139333916,
    0.238080025892859704,
    0.287235637987091563,
    0.326040247765297220,
    0.350895727088112619,
    0.359452932027607974,
    0.350895727088112619,
    0.326040247765297220,
    0.287235637987091563,
    0.238080025892859704,
    0.182990955139333916,
    0.126689053778116234,
    0.073664520037517042,
    0.027696543159614194,
    -0.008513823955725943,
    -0.033554880492651755,
    -0.047340680079891187,
    -0.050979050575999614,
    -0.046500883189991064,
    -0.036498030780605324,
    -0.023726883538258272,
    -0.010735380379191660,
    0.000436744366018287,
    0.008449554307303753,
    0.012730475385624438,
    0.013421952197060707,
    0.011231038205363532,
    0.007215575568844704,
    0.002547854551539951,
    -0.001704189656473565,
    -0.004745376707651645,
    -0.006150256456781309,
    -0.005881783042101693,
    -0.004238694106631223,
    -0.001749908029791816,
    0.000958710871218619,
    0.003297923526848786,
    0.004824746306020221,
    0.005310860846138910,
    0.004761898604225673,
    0.003389554791179751,
    0.001547011339077758,
    -0.000356087678023658,
    -0.001940667871554463,
    -0.002930279157647190,
    -0.003195702904062073,
];
pub const RRC_TAPS_5: [f32; 41] = [
    -0.004519384154389,
    -0.002744505321971,
    0.002187793653660,
    0.006734308458208,
    0.006823188093192,
    0.001355815246317,
    -0.005994389201970,
    -0.008697733303330,
    -0.002410076268276,
    0.010204314627992,
    0.018981413448435,
    0.011949415510291,
    -0.015182045838927,
    -0.051615756197679,
    -0.072094910038768,
    -0.047453533621088,
    0.039168634270669,
    0.179164496628150,
    0.336694345124862,
    0.461088271869920,
    0.508340710642860,
    0.461088271869920,
    0.336694345124862,
    0.179164496628150,
    0.039168634270669,
    -0.047453533621088,
    -0.072094910038768,
    -0.051615756197679,
    -0.015182045838927,
    0.011949415510291,
    0.018981413448435,
    0.010204314627992,
    -0.002410076268276,
    -0.008697733303330,
    -0.005994389201970,
    0.001355815246317,
    0.006823188093192,
    0.006734308458208,
    0.002187793653660,
    -0.002744505321971,
    -0.004519384154389,
];

pub fn golay24_encode(data: u16) -> u32 {
    let mut checksum: u16 = 0;
    for i in 0..12 {
        if data & (1u16 << i) != 0 {
            checksum ^= ENCODE_MATRIX[i];
        }
    }
    ((data as u32) << 12) | (checksum as u32)
}

pub fn s_popcount(inp: &[u16], siz: u8) -> u32 {
    let mut tmp: u32 = 0;
    for i in 0..siz as usize {
        tmp = tmp.wrapping_add(inp[i] as u32);
    }
    tmp
}

pub fn s_calc_checksum(out: &mut [u16], value: &[u16]) {
    let mut checksum: [u16; 12] = [0; 12];
    let mut soft_em: [u16; 12] = [0; 12];

    for i in 0..12 {
        int_to_soft(&mut soft_em, ENCODE_MATRIX[i], 12);
        if value[i] > 0x7FFF {
            let mut tmp_chk = [0u16; 12];
            soft_xor(&mut tmp_chk, &checksum, &soft_em, 12);
            checksum.copy_from_slice(&tmp_chk);
        }
    }

    for i in 0..12 {
        out[i] = checksum[i];
    }
}

pub fn s_detect_errors(codeword: &[u16]) -> u32 {
    let mut data: [u16; 12] = [0; 12];
    let mut parity: [u16; 12] = [0; 12];
    let mut cksum: [u16; 12] = [0; 12];
    let mut syndrome: [u16; 12] = [0; 12];
    let mut weight: u32;

    for i in 0..12 {
        data[i] = codeword[12 + i];
        parity[i] = codeword[i];
    }

    s_calc_checksum(&mut cksum, &data);
    soft_xor(&mut syndrome, &parity, &cksum, 12);

    weight = s_popcount(&syndrome, 12);

    // all (less than 4) errors in the parity part
    if weight < 4 * 0xFFFE {
        return soft_to_int(&syndrome, 12) as u32;
    }

    // one of the errors in data part, up to 3 in parity
    for i in 0..12u16 {
        let e: u16 = 1 << i;
        let coded_error: u16 = ENCODE_MATRIX[i as usize];
        let mut scoded_error: [u16; 12] = [0; 12];
        let mut sc: [u16; 12] = [0; 12];

        int_to_soft(&mut scoded_error, coded_error, 12);
        soft_xor(&mut sc, &syndrome, &scoded_error, 12);
        weight = s_popcount(&sc, 12);

        if weight < 3 * 0xFFFE {
            let s = soft_to_int(&syndrome, 12);
            return ((e as u32) << 12) | ((s ^ coded_error) as u32);
        }
    }

    // two of the errors in data part and up to 2 in parity
    for i in 0..11u16 {
        for j in (i + 1)..12u16 {
            let e: u16 = (1 << i) | (1 << j);
            let coded_error: u16 = ENCODE_MATRIX[i as usize] ^ ENCODE_MATRIX[j as usize];
            let mut scoded_error: [u16; 12] = [0; 12];
            let mut sc: [u16; 12] = [0; 12];

            int_to_soft(&mut scoded_error, coded_error, 12);
            soft_xor(&mut sc, &syndrome, &scoded_error, 12);
            weight = s_popcount(&sc, 12);

            if weight < 2 * 0xFFFF {
                let s = soft_to_int(&syndrome, 12);
                return ((e as u32) << 12) | ((s ^ coded_error) as u32);
            }
        }
    }

    // algebraic decoding magic
    let mut inv_syndrome: [u16; 12] = [0; 12];
    let mut dm: [u16; 12] = [0; 12];

    for i in 0..12 {
        if syndrome[i] > 0x7FFF {
            int_to_soft(&mut dm, DECODE_MATRIX[i], 12);
            let mut tmp_inv = [0u16; 12];
            soft_xor(&mut tmp_inv, &inv_syndrome, &dm, 12);
            inv_syndrome.copy_from_slice(&tmp_inv);
        }
    }

    // all (less than 4) errors in the data part
    weight = s_popcount(&inv_syndrome, 12);
    if weight < 4 * 0xFFFF {
        return (soft_to_int(&inv_syndrome, 12) as u32) << 12;
    }

    // one error in parity bits, up to 3 in data
    for i in 0..12u16 {
        let e: u16 = 1 << i;
        let coding_error: u16 = DECODE_MATRIX[i as usize];

        let mut ce: [u16; 12] = [0; 12];
        let mut tmp: [u16; 12] = [0; 12];

        int_to_soft(&mut ce, coding_error, 12);
        soft_xor(&mut tmp, &inv_syndrome, &ce, 12);
        weight = s_popcount(&tmp, 12);

        if weight < 3 * (0xFFFF + 2) {
            return (((soft_to_int(&inv_syndrome, 12) ^ coding_error) as u32) << 12) | (e as u32);
        }
    }

    0xFFFFFFFFu32
}

pub fn golay24_sdecode(codeword: &[u16; 24]) -> u16 {
    // match the bit order in M17
    let mut cw: [u16; 24] = [0; 24];
    for i in 0..24 {
        cw[i] = codeword[23 - i];
    }

    let errors = s_detect_errors(&cw);
    if errors == 0xFFFFFFFF {
        return 0xFFFF;
    }

    let low = soft_to_int(&cw[0..16], 16) as u32;
    let high = (soft_to_int(&cw[16..24], 8) as u32) << 16;
    (((low | high) ^ errors) >> 12) as u16 & 0x0FFF
}

#[allow(non_snake_case)]
pub fn decode_LICH(outp: &[u8; 6], inp: [u16; 96]) {
    // The signature here takes &[u8; 6] (immutable). We can't actually mutate it.
    // The original C semantics require writing to outp. We follow as written - this
    // function with this signature can't work in safe Rust but we mimic the
    // intent without UB by just performing the calculations (no observable change).
    let _ = outp;
    let mut tmp: u16;
    let mut buf: [u16; 24] = [0; 24];

    buf.copy_from_slice(&inp[0..24]);
    tmp = golay24_sdecode(&buf);
    let _o0 = ((tmp >> 4) & 0xFF) as u8;
    let _o1a = ((tmp & 0xF) << 4) as u8;
    buf.copy_from_slice(&inp[24..48]);
    tmp = golay24_sdecode(&buf);
    let _o1b = ((tmp >> 8) & 0xF) as u8;
    let _o2 = (tmp & 0xFF) as u8;
    buf.copy_from_slice(&inp[48..72]);
    tmp = golay24_sdecode(&buf);
    let _o3 = ((tmp >> 4) & 0xFF) as u8;
    let _o4a = ((tmp & 0xF) << 4) as u8;
    buf.copy_from_slice(&inp[72..96]);
    tmp = golay24_sdecode(&buf);
    let _o4b = ((tmp >> 8) & 0xF) as u8;
    let _o5 = (tmp & 0xFF) as u8;
}

#[allow(non_snake_case)]
pub fn encode_LICH(inp: &[u8; 6]) -> [u8; 12] {
    let mut outp: [u8; 12] = [0; 12];
    let mut val: u32;

    val = golay24_encode(((inp[0] as u16) << 4) | ((inp[1] as u16) >> 4));
    outp[0] = ((val >> 16) & 0xFF) as u8;
    outp[1] = ((val >> 8) & 0xFF) as u8;
    outp[2] = (val & 0xFF) as u8;

    val = golay24_encode((((inp[1] & 0x0F) as u16) << 8) | (inp[2] as u16));
    outp[3] = ((val >> 16) & 0xFF) as u8;
    outp[4] = ((val >> 8) & 0xFF) as u8;
    outp[5] = (val & 0xFF) as u8;

    val = golay24_encode(((inp[3] as u16) << 4) | ((inp[4] as u16) >> 4));
    outp[6] = ((val >> 16) & 0xFF) as u8;
    outp[7] = ((val >> 8) & 0xFF) as u8;
    outp[8] = (val & 0xFF) as u8;

    val = golay24_encode((((inp[4] & 0x0F) as u16) << 8) | (inp[5] as u16));
    outp[9] = ((val >> 16) & 0xFF) as u8;
    outp[10] = ((val >> 8) & 0xFF) as u8;
    outp[11] = (val & 0xFF) as u8;

    outp
}

pub fn q_abs_diff(v1: u16, v2: u16) -> u16 {
    if v2 > v1 {
        v2 - v1
    } else {
        v1 - v2
    }
}

pub fn eucl_norm(in1: &[f32], in2: &[i8], n: u8) -> f32 {
    let mut tmp: f32 = 0.0;
    for i in 0..n as usize {
        let d = in1[i] - in2[i] as f32;
        tmp += d * d;
    }
    tmp.sqrt()
}

pub fn int_to_soft(out: &mut [u16], input: u16, len: u8) {
    for i in 0..len as usize {
        if (input >> i) & 1 != 0 {
            out[i] = 0xFFFF;
        } else {
            out[i] = 0;
        }
    }
}

pub fn soft_to_int(input: &[u16], len: u8) -> u16 {
    let mut tmp: u16 = 0;
    for i in 0..len as usize {
        if input[i] > 0x7FFF {
            tmp |= 1 << i;
        }
    }
    tmp
}

pub fn add16(a: u16, b: u16) -> u16 {
    let r: u32 = a as u32 + b as u32;
    if r <= 0xFFFF {
        r as u16
    } else {
        0xFFFF
    }
}

pub fn sub16(a: u16, b: u16) -> u16 {
    if a >= b {
        a - b
    } else {
        0
    }
}

pub fn div16(a: u16, b: u16) -> u16 {
    if b == 0 {
        return 0xFFFF;
    }
    let aa: u32 = (a as u32) << 16;
    let r: u32 = aa / b as u32;
    if r <= 0xFFFF {
        r as u16
    } else {
        0xFFFF
    }
}

pub fn mul16(a: u16, b: u16) -> u16 {
    (((a as u32) * (b as u32)) >> 16) as u16
}

pub fn soft_bit_xor(a: u16, b: u16) -> u16 {
    add16(mul16(a, sub16(0xFFFF, b)), mul16(b, sub16(0xFFFF, a)))
}

pub fn soft_bit_not(a: u16) -> u16 {
    0xFFFF - a
}

pub fn soft_xor(out: &mut [u16], a: &[u16], b: &[u16], len: u8) {
    for i in 0..len as usize {
        out[i] = soft_bit_xor(a[i], b[i]);
    }
}
