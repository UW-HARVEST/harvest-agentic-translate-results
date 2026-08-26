// Translation of c_src/src/lib.c and c_src/include/lib.h
// Produces byte-identical results to the original C code.

pub type TflacU8 = u8;
pub type TflacS32 = i32;
pub type TflacU32 = u32;
pub type TflacU64 = u64;

// In the original C code: typedef tflac_u64 tflac_uint;
pub type TflacUint = TflacU64;

#[derive(Clone, Copy)]
pub struct TflacMd5 {
    pub pos: TflacU32,
    pub total: TflacU64,
    pub buffer: [TflacU8; 64 + 8],
}

impl Default for TflacMd5 {
    fn default() -> Self {
        TflacMd5 {
            pos: 0,
            total: 0,
            buffer: [0u8; 72],
        }
    }
}

#[derive(Clone, Copy)]
pub struct Tflac {
    pub md5_ctx: TflacMd5,
    pub cur_blocksize: TflacU32,
    pub channels: TflacU32,
}

impl Default for Tflac {
    fn default() -> Self {
        Tflac {
            md5_ctx: TflacMd5::default(),
            cur_blocksize: 0,
            channels: 0,
        }
    }
}

pub fn tflac_pack_u64le(d: &mut [TflacU8], n: TflacU64) {
    d[0] = n as TflacU8;
    d[1] = (n >> 8) as TflacU8;
    d[2] = (n >> 16) as TflacU8;
    d[3] = (n >> 24) as TflacU8;
    d[4] = (n >> 32) as TflacU8;
    d[5] = (n >> 40) as TflacU8;
    d[6] = (n >> 48) as TflacU8;
    d[7] = (n >> 56) as TflacU8;
}

pub fn tflac_md5_addsample(m: &mut TflacMd5, bits: TflacU32, val: TflacUint) {
    let mut bytes: TflacU32;
    m.total = m.total.wrapping_add(bits as TflacU64);
    bytes = bits / 8;
    let pos2: TflacU32 = m.pos % 64;
    {
        let start = pos2 as usize;
        tflac_pack_u64le(&mut m.buffer[start..start + 8], val);
    }
    m.pos = m.pos.wrapping_add(bytes);
    if m.pos >= 64 {
        m.pos %= 64;
        bytes = m.pos;
        // Replicates: while (bytes--) { m->buffer[bytes] = m->buffer[64 + bytes]; }
        // Note: post-decrement means the loop runs while old value is non-zero,
        // and the index used inside is the decremented value.
        while bytes != 0 {
            bytes -= 1;
            m.buffer[bytes as usize] = m.buffer[64 + bytes as usize];
        }
    }
}

/// Translates: tflac_u32 update_md5(tflac *t, const tflac_s32 *samples)
///
/// NOTE: The original C code performs `samples += (8 * sizeof(tflac_s32))`, which
/// advances the pointer by 32 `tflac_s32` elements rather than 8. This is preserved
/// to retain byte-identical behavior with the C implementation. The caller therefore
/// must pass a slice that is large enough to be indexed up to (4 * 32) + 7.
pub fn update_md5(t: &mut Tflac, samples: &[TflacS32]) -> TflacU32 {
    let mut b: TflacU32 = t.cur_blocksize.wrapping_mul(t.channels);
    let step: TflacU32 = std::mem::size_of::<TflacUint>() as TflacU32; // 8
    let mut v: TflacUint;

    let mut offset: usize = 0;
    let mut i = 0;
    while i <= 4 {
        v = ((samples[offset] as TflacUint) & 0xFF) << 0;
        v |= ((samples[offset + 1] as TflacUint) & 0xFF) << 8;
        v |= ((samples[offset + 2] as TflacUint) & 0xFF) << 16;
        v |= ((samples[offset + 3] as TflacUint) & 0xFF) << 24;
        v |= ((samples[offset + 4] as TflacUint) & 0xFF) << 32;
        v |= ((samples[offset + 5] as TflacUint) & 0xFF) << 40;
        v |= ((samples[offset + 6] as TflacUint) & 0xFF) << 48;
        v |= ((samples[offset + 7] as TflacUint) & 0xFF) << 56;
        tflac_md5_addsample(
            &mut t.md5_ctx,
            (8 * std::mem::size_of::<TflacUint>()) as TflacU32,
            v,
        );
        b = b.wrapping_sub(step);
        // Match C bug: samples += (8 * sizeof(tflac_s32)) advances by 32 i32 elements.
        offset = offset.wrapping_add(8 * std::mem::size_of::<TflacS32>());
        i += 1;
    }
    b
}
