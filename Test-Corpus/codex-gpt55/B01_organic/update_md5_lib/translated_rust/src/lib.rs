#[repr(C)]
pub struct tflac_md5 {
    pub pos: u32,
    pub total: u64,
    pub buffer: [u8; 64 + 8],
}

#[repr(C)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: u32,
    pub channels: u32,
}

type TflacUint = u64;

unsafe fn pack_u64le(d: *mut u8, n: u64) {
    unsafe {
        *d.add(0) = n as u8;
        *d.add(1) = (n >> 8) as u8;
        *d.add(2) = (n >> 16) as u8;
        *d.add(3) = (n >> 24) as u8;
        *d.add(4) = (n >> 32) as u8;
        *d.add(5) = (n >> 40) as u8;
        *d.add(6) = (n >> 48) as u8;
        *d.add(7) = (n >> 56) as u8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut u8, n: u64) {
    unsafe {
        pack_u64le(d, n);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(m: *mut tflac_md5, bits: u32, val: TflacUint) {
    unsafe {
        (*m).total = (*m).total.wrapping_add(bits as u64);
        let mut bytes = bits / 8;
        let pos2 = (*m).pos % 64;
        pack_u64le((*m).buffer.as_mut_ptr().add(pos2 as usize), val);
        (*m).pos = (*m).pos.wrapping_add(bytes);
        if (*m).pos >= 64 {
            (*m).pos %= 64;
            bytes = (*m).pos;
            while bytes != 0 {
                bytes = bytes.wrapping_sub(1);
                (*m).buffer[bytes as usize] = (*m).buffer[(64 + bytes) as usize];
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, mut samples: *const i32) -> u32 {
    unsafe {
        let mut b = (*t).cur_blocksize.wrapping_mul((*t).channels);
        let step = core::mem::size_of::<TflacUint>() as u32;
        for _ in 0..=4 {
            let mut v = ((*samples.add(0) as TflacUint) & 0xff) << 0;
            v |= ((*samples.add(1) as TflacUint) & 0xff) << 8;
            v |= ((*samples.add(2) as TflacUint) & 0xff) << 16;
            v |= ((*samples.add(3) as TflacUint) & 0xff) << 24;
            v |= ((*samples.add(4) as TflacUint) & 0xff) << 32;
            v |= ((*samples.add(5) as TflacUint) & 0xff) << 40;
            v |= ((*samples.add(6) as TflacUint) & 0xff) << 48;
            v |= ((*samples.add(7) as TflacUint) & 0xff) << 56;
            tflac_md5_addsample(
                core::ptr::addr_of_mut!((*t).md5_ctx),
                8 * core::mem::size_of::<TflacUint>() as u32,
                v,
            );
            b = b.wrapping_sub(step);
            samples = samples.add(8 * core::mem::size_of::<i32>());
        }
        b
    }
}
