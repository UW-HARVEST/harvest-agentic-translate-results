use std::os::raw::c_uint;

pub type TflacU8 = u8;
pub type TflacS32 = i32;
pub type TflacU32 = u32;
pub type TflacU64 = u64;
type TflacUint = TflacU64;

#[repr(C)]
pub struct tflac_md5 {
    pub pos: TflacU32,
    pub total: TflacU64,
    pub buffer: [TflacU8; 64 + 8],
}

pub type tflac_md5_t = tflac_md5;

#[repr(C)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: TflacU32,
    pub channels: TflacU32,
}

pub type tflac_t = tflac;

#[unsafe(no_mangle)]
pub extern "C" fn tflac_pack_u64le(d: *mut TflacU8, n: TflacU64) {
    unsafe {
        *d.add(0) = n as TflacU8;
        *d.add(1) = (n >> 8) as TflacU8;
        *d.add(2) = (n >> 16) as TflacU8;
        *d.add(3) = (n >> 24) as TflacU8;
        *d.add(4) = (n >> 32) as TflacU8;
        *d.add(5) = (n >> 40) as TflacU8;
        *d.add(6) = (n >> 48) as TflacU8;
        *d.add(7) = (n >> 56) as TflacU8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tflac_md5_addsample(m: *mut tflac_md5, bits: TflacU32, val: TflacUint) {
    unsafe {
        (*m).total += bits as TflacU64;
        let mut bytes = bits / 8;
        let pos2 = (*m).pos % 64;
        tflac_pack_u64le((*m).buffer.as_mut_ptr().add(pos2 as usize), val);
        (*m).pos += bytes;
        if (*m).pos >= 64 {
            (*m).pos %= 64;
            bytes = (*m).pos;
            while bytes != 0 {
                bytes -= 1;
                (*m).buffer[bytes as usize] = (*m).buffer[(64 + bytes) as usize];
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn update_md5(t: *mut tflac, mut samples: *const TflacS32) -> TflacU32 {
    unsafe {
        let mut b = (*t).cur_blocksize.wrapping_mul((*t).channels);
        let step = std::mem::size_of::<TflacUint>() as TflacU32;
        let mut v: TflacUint;
        let mut i: c_uint = 0;
        while i <= 4 {
            v = ((*samples.add(0) as TflacUint) & 0xFF) << 0;
            v |= ((*samples.add(1) as TflacUint) & 0xFF) << 8;
            v |= ((*samples.add(2) as TflacUint) & 0xFF) << 16;
            v |= ((*samples.add(3) as TflacUint) & 0xFF) << 24;
            v |= ((*samples.add(4) as TflacUint) & 0xFF) << 32;
            v |= ((*samples.add(5) as TflacUint) & 0xFF) << 40;
            v |= ((*samples.add(6) as TflacUint) & 0xFF) << 48;
            v |= ((*samples.add(7) as TflacUint) & 0xFF) << 56;
            tflac_md5_addsample(t.cast::<u8>().cast::<tflac>().as_mut().unwrap().md5_ctx_ptr(), (8 * std::mem::size_of::<TflacUint>()) as TflacU32, v);
            b = b.wrapping_sub(step);
            samples = samples.add(8 * std::mem::size_of::<TflacS32>());
            i += 1;
        }
        b
    }
}

trait TflacExt {
    fn md5_ctx_ptr(&mut self) -> *mut tflac_md5;
}

impl TflacExt for tflac {
    fn md5_ctx_ptr(&mut self) -> *mut tflac_md5 {
        &mut self.md5_ctx
    }
}
