use std::ptr;

#[repr(C)]
pub struct TflacMd5 {
    pub pos: u32,
    pub total: u64,
    pub buffer: [u8; 64 + 8],
}

#[repr(C)]
pub struct Tflac {
    pub md5_ctx: TflacMd5,
    pub cur_blocksize: u32,
    pub channels: u32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut u8, n: u64) {
    for offset in 0..8 {
        unsafe {
            d.add(offset).write((n >> (offset * 8)) as u8);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(m: *mut TflacMd5, bits: u32, val: u64) {
    let total = unsafe { ptr::addr_of_mut!((*m).total) };
    unsafe {
        total.write(total.read().wrapping_add(bits as u64));
    }

    let mut bytes = bits / 8;
    let pos = unsafe { ptr::addr_of_mut!((*m).pos) };
    let current_pos = unsafe { pos.read() };
    let buffer = unsafe { ptr::addr_of_mut!((*m).buffer).cast::<u8>() };

    unsafe {
        tflac_pack_u64le(buffer.add((current_pos % 64) as usize), val);
    }

    let new_pos = current_pos.wrapping_add(bytes);
    unsafe {
        pos.write(new_pos);
    }

    if new_pos >= 64 {
        let wrapped_pos = new_pos % 64;
        unsafe {
            pos.write(wrapped_pos);
        }
        bytes = wrapped_pos;
        while bytes != 0 {
            bytes = bytes.wrapping_sub(1);
            unsafe {
                buffer
                    .add(bytes as usize)
                    .write(buffer.add(64 + bytes as usize).read());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut Tflac, mut samples: *const i32) -> u32 {
    let cur_blocksize = unsafe { ptr::addr_of!((*t).cur_blocksize).read() };
    let channels = unsafe { ptr::addr_of!((*t).channels).read() };
    let mut b = cur_blocksize.wrapping_mul(channels);

    for _ in 0..=4 {
        let mut v = 0_u64;
        for offset in 0..8 {
            let sample = unsafe { samples.add(offset).read() };
            v |= ((sample as u64) & 0xff) << (offset * 8);
        }

        unsafe {
            tflac_md5_addsample(ptr::addr_of_mut!((*t).md5_ctx), 8 * size_of::<u64>() as u32, v);
            samples = samples.add(8 * size_of::<i32>());
        }
        b = b.wrapping_sub(size_of::<u64>() as u32);
    }

    b
}
