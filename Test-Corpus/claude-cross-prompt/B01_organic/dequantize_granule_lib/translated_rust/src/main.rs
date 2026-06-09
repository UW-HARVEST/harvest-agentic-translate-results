#![allow(dead_code)]

pub struct Bs<'a> {
    pub buf: &'a [u8],
    pub pos: i32,
    pub limit: i32,
}

pub struct L12ScaleInfo {
    pub scf: [f32; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

fn get_bits(bs: &mut Bs, n: i32) -> u32 {
    let s: u32 = (bs.pos & 7) as u32;
    let mut shl: i32 = n + s as i32;
    let mut p_idx: usize = (bs.pos >> 3) as usize;
    bs.pos += n;
    if bs.pos > bs.limit {
        return 0;
    }
    let mut next: u32 = (bs.buf[p_idx] & (255u8 >> s)) as u32;
    p_idx += 1;
    let mut cache: u32 = 0;
    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = bs.buf[p_idx] as u32;
        p_idx += 1;
    }
    cache | (next >> ((-shl) as u32))
}

pub fn dequantize_granule(
    grbuf: &mut [f32],
    bs: &mut Bs,
    sci: &mut L12ScaleInfo,
    group_size: i32,
) -> i32 {
    let mut choff: i32 = 576;
    for j in 0..4i32 {
        let mut dst_idx: i32 = group_size * j;
        let total = 2i32 * sci.total_bands as i32;
        for i in 0..total {
            let ba: i32 = sci.bitalloc[i as usize] as i32;
            if ba != 0 {
                if ba < 17 {
                    let half: i32 = (1i32 << (ba - 1)) - 1;
                    for k in 0..group_size {
                        let raw = get_bits(bs, ba) as i32;
                        let v = raw.wrapping_sub(half);
                        grbuf[(dst_idx + k) as usize] = v as f32;
                    }
                } else {
                    let mod_: u32 = (2u32 << (ba - 17)) + 1;
                    let mut code: u32 = get_bits(bs, (mod_ + 2 - (mod_ >> 3)) as i32);
                    for k in 0..group_size {
                        let raw = (code % mod_).wrapping_sub(mod_ / 2);
                        let v = raw as i32;
                        grbuf[(dst_idx + k) as usize] = v as f32;
                        code /= mod_;
                    }
                }
            }
            dst_idx += choff;
            choff = 18 - choff;
        }
    }
    group_size * 4
}

fn main() {
    // The original C code provides a library with no main entry point;
    // therefore the corresponding executable produces no output.
}
