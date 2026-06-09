/// Translation of `hex2bin` from the C source.
///
/// Returns either:
///   `Ok((bin_pos, hex_pos))` on success — the number of bytes written and
///   the position the parser advanced to in `hex`.
///   `Err(hex_pos)` on failure — `hex_pos` is the position in `hex` where the
///   parser stopped.
///
/// `bin` must be a mutable slice of length >= `bin_maxlen`.
/// If `hex_end_provided` is `false`, then a non-empty unparsed remainder of
/// `hex` is treated as an error (matches the C's `hex_end_p == NULL` branch).
pub fn hex2bin(
    bin: &mut [u8],
    bin_maxlen: usize,
    hex: &[u8],
    hex_len: usize,
    ignore: Option<&[u8]>,
    hex_end_provided: bool,
) -> Result<(usize, usize), usize> {
    let mut bin_pos: usize = 0;
    let mut hex_pos: usize = 0;
    let mut ret: i32 = 0;
    let mut c_acc: u8 = 0;
    let mut state: u8 = 0;

    while hex_pos < hex_len {
        let c: u8 = hex[hex_pos];
        // Use wrapping arithmetic in u32 to mimic C's unsigned int promotion of
        // unsigned char operands.
        let c_u: u32 = c as u32;
        let c_num: u32 = c_u ^ 48u32;
        let c_num0: u32 = (c_num.wrapping_sub(10u32)) >> 8;
        let c_alpha: u32 = (c_u & !32u32).wrapping_sub(55u32);
        let c_alpha0: u32 =
            ((c_alpha.wrapping_sub(10u32)) ^ (c_alpha.wrapping_sub(16u32))) >> 8;

        if (c_num0 | c_alpha0) == 0 {
            if let Some(ignore_bytes) = ignore {
                if state == 0 && ignore_bytes.contains(&c) {
                    hex_pos += 1;
                    continue;
                }
            }
            break;
        }

        let c_val: u8 = ((c_num0 & c_num) | (c_alpha0 & c_alpha)) as u8;
        if bin_pos >= bin_maxlen {
            ret = -1;
            break;
        }
        if state == 0 {
            c_acc = c_val.wrapping_mul(16);
        } else {
            bin[bin_pos] = c_acc | c_val;
            bin_pos += 1;
        }
        state = !state;
        hex_pos += 1;
    }

    if state != 0 {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end_provided && hex_pos != hex_len {
        ret = -1;
    }
    if ret != 0 {
        return Err(hex_pos);
    }
    Ok((bin_pos, hex_pos))
}
