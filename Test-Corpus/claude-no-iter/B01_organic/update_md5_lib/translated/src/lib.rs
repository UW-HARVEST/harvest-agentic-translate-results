use std::ffi::c_int;

#[allow(non_camel_case_types)]
type tflac_u8 = u8;
#[allow(non_camel_case_types)]
type tflac_s32 = i32;
#[allow(non_camel_case_types)]
type tflac_u32 = u32;
#[allow(non_camel_case_types)]
type tflac_u64 = u64;
#[allow(non_camel_case_types)]
type tflac_uint = tflac_u64;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct tflac_md5 {
    pub pos: tflac_u32,
    pub total: tflac_u64,
    pub buffer: [tflac_u8; 64 + 8],
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: tflac_u32,
    pub channels: tflac_u32,
}

fn tflac_pack_u64le(d: &mut [tflac_u8], n: tflac_u64) {
    d[0] = n as tflac_u8;
    d[1] = (n >> 8) as tflac_u8;
    d[2] = (n >> 16) as tflac_u8;
    d[3] = (n >> 24) as tflac_u8;
    d[4] = (n >> 32) as tflac_u8;
    d[5] = (n >> 40) as tflac_u8;
    d[6] = (n >> 48) as tflac_u8;
    d[7] = (n >> 56) as tflac_u8;
}

fn tflac_md5_addsample(m: &mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    let mut bytes: tflac_u32;
    m.total = m.total.wrapping_add(bits as tflac_u64);
    bytes = bits / 8;
    let pos2 = (m.pos % 64) as usize;
    tflac_pack_u64le(&mut m.buffer[pos2..pos2 + 8], val);
    m.pos = m.pos.wrapping_add(bytes);
    if m.pos >= 64 {
        m.pos %= 64;
        bytes = m.pos;
        // emulate `while (bytes--) { ... }`: post-decrement, runs while value before
        // decrement is non-zero. We iterate from `m.pos - 1` down to 0.
        loop {
            if bytes == 0 {
                break;
            }
            bytes -= 1;
            m.buffer[bytes as usize] = m.buffer[64 + bytes as usize];
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    let t = unsafe { &mut *t };
    let mut b: tflac_u32 = t.cur_blocksize.wrapping_mul(t.channels);
    let step: tflac_u32 = std::mem::size_of::<tflac_uint>() as tflac_u32;
    let mut v: tflac_uint;
    let mut samples_ptr = samples;
    let mut _i: c_int = 0;
    while _i <= 4 {
        v = ((unsafe { *samples_ptr.add(0) } as tflac_uint) & 0xFF) << 0;
        v |= ((unsafe { *samples_ptr.add(1) } as tflac_uint) & 0xFF) << 8;
        v |= ((unsafe { *samples_ptr.add(2) } as tflac_uint) & 0xFF) << 16;
        v |= ((unsafe { *samples_ptr.add(3) } as tflac_uint) & 0xFF) << 24;
        v |= ((unsafe { *samples_ptr.add(4) } as tflac_uint) & 0xFF) << 32;
        v |= ((unsafe { *samples_ptr.add(5) } as tflac_uint) & 0xFF) << 40;
        v |= ((unsafe { *samples_ptr.add(6) } as tflac_uint) & 0xFF) << 48;
        v |= ((unsafe { *samples_ptr.add(7) } as tflac_uint) & 0xFF) << 56;
        tflac_md5_addsample(
            &mut t.md5_ctx,
            (8 * std::mem::size_of::<tflac_uint>()) as tflac_u32,
            v,
        );
        b = b.wrapping_sub(step);
        // C: samples += (8 * sizeof(tflac_s32));
        // Pointer arithmetic on tflac_s32* adds (8 * sizeof(tflac_s32)) elements,
        // i.e., advances by 32 i32 elements.
        samples_ptr = unsafe { samples_ptr.add(8 * std::mem::size_of::<tflac_s32>()) };
        _i += 1;
    }
    b
}
