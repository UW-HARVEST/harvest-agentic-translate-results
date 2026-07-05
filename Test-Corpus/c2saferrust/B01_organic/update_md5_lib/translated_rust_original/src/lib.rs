


pub type __uint8_t = u8;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type int32_t = __int32_t;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type tflac_u8 = uint8_t;
pub type tflac_s32 = int32_t;
pub type tflac_u32 = uint32_t;
pub type tflac_u64 = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac_md5 {
    pub pos: tflac_u32,
    pub total: tflac_u64,
    pub buffer: [tflac_u8; 72],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: tflac_u32,
    pub channels: tflac_u32,
}
pub type tflac_uint = tflac_u64;
#[no_mangle]
pub fn tflac_pack_u64le(d: &mut [tflac_u8], n: tflac_u64) {
    let bytes = n.to_le_bytes();
    d[..8].copy_from_slice(&bytes);
}

#[no_mangle]
pub unsafe extern "C" fn tflac_md5_addsample(
    m: *mut tflac_md5,
    bits: tflac_u32,
    val: tflac_uint,
) {
    let m = &mut *m;

    m.total = m.total.wrapping_add(bits as tflac_u64);

    let mut bytes = bits / 8;
    let pos2 = m.pos % 64;

    tflac_pack_u64le(&mut m.buffer[pos2 as usize..], val as tflac_u64);

    m.pos = m.pos.wrapping_add(bytes);

    if m.pos >= 64 {
        m.pos %= 64;
        bytes = m.pos;

        while bytes != 0 {
            bytes -= 1;
            m.buffer[bytes as usize] = m.buffer[(64 + bytes) as usize];
        }
    }
}

#[no_mangle]
pub fn update_md5(t: &mut tflac, samples: &[tflac_s32]) -> tflac_u32 {
    let mut b: tflac_u32 = t.cur_blocksize.wrapping_mul(t.channels);
    let step: tflac_u32 = core::mem::size_of::<tflac_uint>() as tflac_u32;

    for chunk in samples.chunks_exact(8).take(5) {
        let v: tflac_uint = ((chunk[0] as tflac_uint & 0xff) << 0)
            | ((chunk[1] as tflac_uint & 0xff) << 8)
            | ((chunk[2] as tflac_uint & 0xff) << 16)
            | ((chunk[3] as tflac_uint & 0xff) << 24)
            | ((chunk[4] as tflac_uint & 0xff) << 32)
            | ((chunk[5] as tflac_uint & 0xff) << 40)
            | ((chunk[6] as tflac_uint & 0xff) << 48)
            | ((chunk[7] as tflac_uint & 0xff) << 56);

        unsafe {
            tflac_md5_addsample(
                &mut t.md5_ctx,
                (8 * core::mem::size_of::<tflac_uint>()) as tflac_u32,
                v,
            );
        }

        b = b.wrapping_sub(step);
    }

    b
}

