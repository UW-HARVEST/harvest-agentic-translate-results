pub type __uint8_t = u8;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type int32_t = i32;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type tflac_u8 = u8;
pub type tflac_s32 = i32;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac_md5 {
    pub pos: u32,
    pub total: u64,
    pub buffer: [u8; 72],
}
impl std::default::Default for tflac_md5 {
    fn default() -> Self {
        tflac_md5 {
        pos: u32::default(),
        total: u64::default(),
        buffer: [u8::default(); 72]
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac {
    pub md5_ctx: crate::src::lib::tflac_md5,
    pub cur_blocksize: u32,
    pub channels: u32,
}
impl std::default::Default for tflac {
    fn default() -> Self {
        tflac {
        md5_ctx: crate::src::lib::tflac_md5::default(),
        cur_blocksize: u32::default(),
        channels: u32::default()
        }
    }
}

pub type tflac_uint = u64;
#[no_mangle]
pub unsafe extern "C" fn tflac_pack_u64le(mut d: *mut tflac_u8, mut n: tflac_u64) {
    *d.offset(0 as libc::c_int as isize) = n as tflac_u8;
    *d.offset(1 as libc::c_int as isize) = (n >> 8 as libc::c_int) as tflac_u8;
    *d.offset(2 as libc::c_int as isize) = (n >> 16 as libc::c_int) as tflac_u8;
    *d.offset(3 as libc::c_int as isize) = (n >> 24 as libc::c_int) as tflac_u8;
    *d.offset(4 as libc::c_int as isize) = (n >> 32 as libc::c_int) as tflac_u8;
    *d.offset(5 as libc::c_int as isize) = (n >> 40 as libc::c_int) as tflac_u8;
    *d.offset(6 as libc::c_int as isize) = (n >> 48 as libc::c_int) as tflac_u8;
    *d.offset(7 as libc::c_int as isize) = (n >> 56 as libc::c_int) as tflac_u8;
}
#[no_mangle]
pub unsafe extern "C" fn tflac_md5_addsample<'a1>(
    mut m: Option<&'a1 mut crate::src::lib::tflac_md5>,
    mut bits: u32,
    mut val: u64,
) {
    let mut bytes: tflac_u32 = 0;
    (*borrow_mut(&mut m).unwrap()).total = ((*borrow_mut(&mut m).unwrap()).total as libc::c_ulong)
        .wrapping_add(bits as tflac_u64 as libc::c_ulong) as tflac_u64
        as tflac_u64;
    bytes = bits.wrapping_div(8 as tflac_u32);
    let mut pos2: tflac_u32 = (*borrow_mut(&mut m).unwrap()).pos.wrapping_rem(64 as tflac_u32);
    tflac_pack_u64le(
        (&raw mut (*borrow_mut(&mut m).unwrap()).buffer as *mut tflac_u8).offset(pos2 as isize) as *mut tflac_u8,
        val as tflac_u64,
    );
    (*borrow_mut(&mut m).unwrap()).pos = ((*borrow_mut(&mut m).unwrap()).pos as libc::c_uint).wrapping_add(bytes as libc::c_uint)
        as tflac_u32 as tflac_u32;
    if (*borrow(& m).unwrap()).pos >= 64 as tflac_u32 {
        (*borrow_mut(&mut m).unwrap()).pos = ((*borrow_mut(&mut m).unwrap()).pos as libc::c_uint).wrapping_rem(64 as libc::c_uint)
            as tflac_u32 as tflac_u32;
        bytes = (*borrow_mut(&mut m).unwrap()).pos;
        loop {
            let fresh0 = bytes;
            bytes = bytes.wrapping_sub(1);
            if !(fresh0 != 0) {
                break;
            }
            (*borrow_mut(&mut m).unwrap()).buffer[bytes as usize] =
                (*borrow_mut(&mut m).unwrap()).buffer[(64 as tflac_u32).wrapping_add(bytes) as usize];
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn update_md5<'a1>(mut t: Option<&'a1 mut crate::src::lib::tflac>, mut samples: * const i32) -> u32 {
    let mut b: tflac_u32 = (*borrow_mut(&mut t).unwrap()).cur_blocksize.wrapping_mul((*borrow_mut(&mut t).unwrap()).channels);
    let step: tflac_u32 = std::mem::size_of::<tflac_uint>() as tflac_u32;
    let mut v: tflac_uint = 0;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i <= 4 as libc::c_int {
        v = (*samples.offset(0 as libc::c_int as isize) as tflac_uint & 0xff as tflac_uint)
            << 0 as libc::c_int;
        v = (v as libc::c_ulong
            | ((*samples.offset(1 as libc::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 8 as libc::c_int) as libc::c_ulong) as tflac_uint;
        v = (v as libc::c_ulong
            | ((*samples.offset(2 as libc::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 16 as libc::c_int) as libc::c_ulong) as tflac_uint;
        v = (v as libc::c_ulong
            | ((*samples.offset(3 as libc::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 24 as libc::c_int) as libc::c_ulong) as tflac_uint;
        v = (v as libc::c_ulong
            | ((*samples.offset(4 as libc::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 32 as libc::c_int) as libc::c_ulong) as tflac_uint;
        v = (v as libc::c_ulong
            | ((*samples.offset(5 as libc::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 40 as libc::c_int) as libc::c_ulong) as tflac_uint;
        v = (v as libc::c_ulong
            | ((*samples.offset(6 as libc::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 48 as libc::c_int) as libc::c_ulong) as tflac_uint;
        v = (v as libc::c_ulong
            | ((*samples.offset(7 as libc::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 56 as libc::c_int) as libc::c_ulong) as tflac_uint;
        tflac_md5_addsample(
            Some(&raw mut (*borrow_mut(&mut t).unwrap()).md5_ctx),
            (8 as usize).wrapping_mul(std::mem::size_of::<tflac_uint>() as usize) as tflac_u32,
            v,
        );
        b = (b as libc::c_uint).wrapping_sub(step as libc::c_uint) as tflac_u32
            as tflac_u32;
        samples = samples.offset(
            (8 as usize).wrapping_mul(std::mem::size_of::<tflac_s32>() as usize) as isize,
        );
        i += 1;
    }
    return b;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

