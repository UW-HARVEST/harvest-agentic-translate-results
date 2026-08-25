use std::ffi::{c_int, c_uchar, c_uint, c_ulonglong};
use std::mem::size_of;
use std::ptr::{addr_of, addr_of_mut};

type TflacU8 = c_uchar;
type TflacS32 = c_int;
type TflacU32 = c_uint;
type TflacU64 = c_ulonglong;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut TflacU8, n: TflacU64) {
    let bytes = n.to_le_bytes();
    for (offset, byte) in bytes.into_iter().enumerate() {
        // SAFETY: The C contract requires d to point to at least eight writable bytes.
        unsafe {
            d.add(offset).write(byte);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(m: *mut TflacMd5, bits: TflacU32, val: TflacUint) {
    // Use raw field pointers so behavior follows the C accesses without creating
    // references whose validity would be stronger than the original contract.
    let total = unsafe { addr_of_mut!((*m).total) };
    unsafe {
        total.write(total.read().wrapping_add(TflacU64::from(bits)));
    }

    let mut bytes = bits / 8;
    let pos = unsafe { addr_of_mut!((*m).pos) };
    let pos2 = unsafe { pos.read() % 64 };
    let buffer = unsafe { addr_of_mut!((*m).buffer).cast::<TflacU8>() };
    unsafe {
        tflac_pack_u64le(buffer.add(pos2 as usize), val);
    }

    let new_pos = unsafe { pos.read().wrapping_add(bytes) };
    unsafe {
        pos.write(new_pos);
    }
    if new_pos >= 64 {
        let reduced_pos = new_pos % 64;
        unsafe {
            pos.write(reduced_pos);
        }
        bytes = reduced_pos;
        while bytes != 0 {
            bytes -= 1;
            let offset = bytes as usize;
            unsafe {
                buffer.add(offset).write(buffer.add(64 + offset).read());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut Tflac, mut samples: *const TflacS32) -> TflacU32 {
    let cur_blocksize = unsafe { addr_of!((*t).cur_blocksize).read() };
    let channels = unsafe { addr_of!((*t).channels).read() };
    let mut b = cur_blocksize.wrapping_mul(channels);
    let step = size_of::<TflacUint>() as TflacU32;

    for _ in 0..=4 {
        let mut v = 0;
        for offset in 0..8 {
            let sample = unsafe { samples.add(offset).read() };
            v |= ((sample as TflacUint) & 0xff) << (offset * 8);
        }

        unsafe {
            tflac_md5_addsample(addr_of_mut!((*t).md5_ctx), 8 * step, v);
        }
        b = b.wrapping_sub(step);
        samples = unsafe { samples.add(8 * size_of::<TflacS32>()) };
    }

    b
}
