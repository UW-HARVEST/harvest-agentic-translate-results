type TflacU8 = u8;
type TflacS32 = i32;
type TflacU32 = u32;
type TflacU64 = u64;
type TflacUint = TflacU64;

#[repr(C)]
pub struct TflacMd5 {
    pos: TflacU32,
    total: TflacU64,
    buffer: [TflacU8; 64 + 8],
}

#[repr(C)]
pub struct Tflac {
    md5_ctx: TflacMd5,
    cur_blocksize: TflacU32,
    channels: TflacU32,
}

fn tflac_pack_u64le(d: &mut [TflacU8], n: TflacU64) {
    d[0] = n as TflacU8;
    d[1] = (n >> 8) as TflacU8;
    d[2] = (n >> 16) as TflacU8;
    d[3] = (n >> 24) as TflacU8;
    d[4] = (n >> 32) as TflacU8;
    d[5] = (n >> 40) as TflacU8;
    d[6] = (n >> 48) as TflacU8;
    d[7] = (n >> 56) as TflacU8;
}

fn tflac_md5_addsample(m: &mut TflacMd5, bits: TflacU32, val: TflacUint) {
    m.total = m.total.wrapping_add(bits as TflacU64);
    let bytes = bits / 8;
    let pos2 = (m.pos % 64) as usize;
    tflac_pack_u64le(&mut m.buffer[pos2..], val);
    m.pos = m.pos.wrapping_add(bytes);
    if m.pos >= 64 {
        m.pos %= 64;
        let mut b = m.pos as usize;
        while b > 0 {
            b -= 1;
            m.buffer[b] = m.buffer[64 + b];
        }
    }
}

/// # Safety
/// `t` must point to a valid `Tflac`. `samples` must point to enough readable `tflac_s32` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut Tflac, samples: *const TflacS32) -> TflacU32 {
    let t = unsafe { &mut *t };
    let mut b = t.cur_blocksize.wrapping_mul(t.channels);
    let step = std::mem::size_of::<TflacUint>() as TflacU32;
    let mut ptr = samples;
    for _i in 0..=4 {
        let s = unsafe { std::slice::from_raw_parts(ptr, 8) };
        let mut v: TflacUint = ((s[0] as TflacUint) & 0xFF) << 0;
        v |= ((s[1] as TflacUint) & 0xFF) << 8;
        v |= ((s[2] as TflacUint) & 0xFF) << 16;
        v |= ((s[3] as TflacUint) & 0xFF) << 24;
        v |= ((s[4] as TflacUint) & 0xFF) << 32;
        v |= ((s[5] as TflacUint) & 0xFF) << 40;
        v |= ((s[6] as TflacUint) & 0xFF) << 48;
        v |= ((s[7] as TflacUint) & 0xFF) << 56;
        tflac_md5_addsample(
            &mut t.md5_ctx,
            (8 * std::mem::size_of::<TflacUint>()) as TflacU32,
            v,
        );
        b = b.wrapping_sub(step);
        // C: samples += (8 * sizeof(tflac_s32)) — pointer arithmetic in units of tflac_s32
        ptr = unsafe { ptr.add(8 * std::mem::size_of::<TflacS32>()) };
    }
    b
}
