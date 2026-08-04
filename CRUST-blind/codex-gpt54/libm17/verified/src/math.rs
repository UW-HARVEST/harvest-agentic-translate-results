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
    let mut checksum = 0u16;

    for (i, &matrix) in ENCODE_MATRIX.iter().enumerate() {
        if (data & (1 << i)) != 0 {
            checksum ^= matrix;
        }
    }

    (u32::from(data) << 12) | u32::from(checksum)
}
pub fn s_popcount(inp: &[u16], siz: u8) -> u32 {
    inp.iter().take(siz as usize).map(|&v| u32::from(v)).sum()
}
pub fn s_calc_checksum(out: &mut [u16], value: &[u16]) {
    let mut checksum = [0u16; 12];
    let mut soft_em = [0u16; 12];

    for (i, &matrix) in ENCODE_MATRIX.iter().enumerate() {
        int_to_soft(&mut soft_em, matrix, 12);
        if value[i] > 0x7FFF {
            let prev = checksum;
            soft_xor(&mut checksum, &prev, &soft_em, 12);
        }
    }

    out[..12].copy_from_slice(&checksum);
}
pub fn s_detect_errors(codeword: &[u16]) -> u32 {
    let mut data = [0u16; 12];
    let mut parity = [0u16; 12];
    let mut cksum = [0u16; 12];
    let mut syndrome = [0u16; 12];

    data.copy_from_slice(&codeword[12..24]);
    parity.copy_from_slice(&codeword[..12]);

    s_calc_checksum(&mut cksum, &data);
    soft_xor(&mut syndrome, &parity, &cksum, 12);

    let mut weight = s_popcount(&syndrome, 12);
    if weight < 4 * 0xFFFE {
        return u32::from(soft_to_int(&syndrome, 12));
    }

    for (i, &coded_error) in ENCODE_MATRIX.iter().enumerate() {
        let e = 1u16 << i;
        let mut scoded_error = [0u16; 12];
        let mut sc = [0u16; 12];

        int_to_soft(&mut scoded_error, coded_error, 12);
        soft_xor(&mut sc, &syndrome, &scoded_error, 12);
        weight = s_popcount(&sc, 12);

        if weight < 3 * 0xFFFE {
            let s = soft_to_int(&syndrome, 12);
            return u32::from((e << 12) | (s ^ coded_error));
        }
    }

    for i in 0..11 {
        for j in (i + 1)..12 {
            let e = (1u16 << i) | (1u16 << j);
            let coded_error = ENCODE_MATRIX[i] ^ ENCODE_MATRIX[j];
            let mut scoded_error = [0u16; 12];
            let mut sc = [0u16; 12];

            int_to_soft(&mut scoded_error, coded_error, 12);
            soft_xor(&mut sc, &syndrome, &scoded_error, 12);
            weight = s_popcount(&sc, 12);

            if weight < 2 * 0xFFFF {
                let s = soft_to_int(&syndrome, 12);
                return u32::from((e << 12) | (s ^ coded_error));
            }
        }
    }

    let mut inv_syndrome = [0u16; 12];
    let mut dm = [0u16; 12];
    for (i, &matrix) in DECODE_MATRIX.iter().enumerate() {
        if syndrome[i] > 0x7FFF {
            int_to_soft(&mut dm, matrix, 12);
            let prev = inv_syndrome;
            soft_xor(&mut inv_syndrome, &prev, &dm, 12);
        }
    }

    weight = s_popcount(&inv_syndrome, 12);
    if weight < 4 * 0xFFFF {
        return u32::from(soft_to_int(&inv_syndrome, 12) << 12);
    }

    for (i, &coding_error) in DECODE_MATRIX.iter().enumerate() {
        let e = 1u16 << i;
        let mut ce = [0u16; 12];
        let mut tmp = [0u16; 12];

        int_to_soft(&mut ce, coding_error, 12);
        soft_xor(&mut tmp, &inv_syndrome, &ce, 12);
        weight = s_popcount(&tmp, 12);

        if weight < 3 * (0xFFFF + 2) {
            return u32::from(((soft_to_int(&inv_syndrome, 12) ^ coding_error) << 12) | e);
        }
    }

    0xFFFF_FFFF
}
pub fn golay24_sdecode(codeword: &[u16; 24]) -> u16 {
    let mut cw = [0u16; 24];
    for i in 0..24 {
        cw[i] = codeword[23 - i];
    }

    let errors = s_detect_errors(&cw);
    if errors == 0xFFFF_FFFF {
        return 0xFFFF;
    }

    ((((u32::from(soft_to_int(&cw[..16], 16))) | (u32::from(soft_to_int(&cw[16..], 8)) << 16)) ^ errors) >> 12
        & 0x0FFF) as u16
}
pub fn decode_LICH(outp: &[u8; 6], inp: [u16; 96]) {
    let mut decoded = [0u8; 6];

    let mut tmp = golay24_sdecode((&inp[0..24]).try_into().expect("slice length is fixed"));
    decoded[0] = ((tmp >> 4) & 0xFF) as u8;
    decoded[1] |= ((tmp & 0x0F) << 4) as u8;
    tmp = golay24_sdecode((&inp[24..48]).try_into().expect("slice length is fixed"));
    decoded[1] |= ((tmp >> 8) & 0x0F) as u8;
    decoded[2] = (tmp & 0xFF) as u8;
    tmp = golay24_sdecode((&inp[48..72]).try_into().expect("slice length is fixed"));
    decoded[3] = ((tmp >> 4) & 0xFF) as u8;
    decoded[4] |= ((tmp & 0x0F) << 4) as u8;
    tmp = golay24_sdecode((&inp[72..96]).try_into().expect("slice length is fixed"));
    decoded[4] |= ((tmp >> 8) & 0x0F) as u8;
    decoded[5] = (tmp & 0xFF) as u8;

    let _ = decoded;
    let _ = outp;
}
pub fn encode_LICH(inp: &[u8; 6]) -> [u8; 12] {
    let mut out = [0u8; 12];

    let mut val = golay24_encode((u16::from(inp[0]) << 4) | u16::from(inp[1] >> 4));
    out[0] = ((val >> 16) & 0xFF) as u8;
    out[1] = ((val >> 8) & 0xFF) as u8;
    out[2] = (val & 0xFF) as u8;

    val = golay24_encode((u16::from(inp[1] & 0x0F) << 8) | u16::from(inp[2]));
    out[3] = ((val >> 16) & 0xFF) as u8;
    out[4] = ((val >> 8) & 0xFF) as u8;
    out[5] = (val & 0xFF) as u8;

    val = golay24_encode((u16::from(inp[3]) << 4) | u16::from(inp[4] >> 4));
    out[6] = ((val >> 16) & 0xFF) as u8;
    out[7] = ((val >> 8) & 0xFF) as u8;
    out[8] = (val & 0xFF) as u8;

    val = golay24_encode((u16::from(inp[4] & 0x0F) << 8) | u16::from(inp[5]));
    out[9] = ((val >> 16) & 0xFF) as u8;
    out[10] = ((val >> 8) & 0xFF) as u8;
    out[11] = (val & 0xFF) as u8;

    out
}
pub fn q_abs_diff(v1: u16, v2: u16) -> u16 {
    v1.abs_diff(v2)
}
pub fn eucl_norm(in1: &[f32], in2: &[i8], n: u8) -> f32 {
    let mut tmp = 0.0f32;

    for i in 0..(n as usize) {
        let diff = in1[i] - f32::from(in2[i]);
        tmp += diff * diff;
    }

    tmp.sqrt()
}
pub fn int_to_soft(out: &mut [u16], input: u16, len: u8) {
    for (i, slot) in out.iter_mut().take(len as usize).enumerate() {
        *slot = if ((input >> i) & 1) != 0 { 0xFFFF } else { 0 };
    }
}
pub fn soft_to_int(input: &[u16], len: u8) -> u16 {
    let mut tmp = 0u16;

    for (i, &value) in input.iter().take(len as usize).enumerate() {
        if value > 0x7FFF {
            tmp |= 1 << i;
        }
    }

    tmp
}
pub fn add16(a: u16, b: u16) -> u16 {
    let r = u32::from(a) + u32::from(b);
    if r <= u32::from(u16::MAX) {
        r as u16
    } else {
        u16::MAX
    }
}
pub fn sub16(a: u16, b: u16) -> u16 {
    a.saturating_sub(b)
}
pub fn div16(a: u16, b: u16) -> u16 {
    if b == 0 {
        return u16::MAX;
    }

    let r = (u32::from(a) << 16) / u32::from(b);
    if r <= u32::from(u16::MAX) {
        r as u16
    } else {
        u16::MAX
    }
}
pub fn mul16(a: u16, b: u16) -> u16 {
    ((u32::from(a) * u32::from(b)) >> 16) as u16
}
pub fn soft_bit_xor(a: u16, b: u16) -> u16 {
    add16(mul16(a, sub16(0xFFFF, b)), mul16(b, sub16(0xFFFF, a)))
}
pub fn soft_bit_not(a: u16) -> u16 {
    0xFFFFu16.wrapping_sub(a)
}
pub fn soft_xor(out: &mut [u16], a: &[u16], b: &[u16], len: u8) {
    for i in 0..(len as usize) {
        out[i] = soft_bit_xor(a[i], b[i]);
    }
}
