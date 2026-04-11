pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type tflac_u8 = u8;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;
pub type tflac_uint = u64;
// #[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac_bitwriter<'a1> {
    pub val: u64,
    pub bits: u32,
    pub pos: u32,
    pub len: u32,
    pub tot: u32,
    pub buffer: Option<&'a1 mut u8>,
}
impl<'a1> std::default::Default for tflac_bitwriter<'a1> {
    fn default() -> Self {
        tflac_bitwriter {
        val: u64::default(),
        bits: u32::default(),
        pos: u32::default(),
        len: u32::default(),
        tot: u32::default(),
        buffer: None
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn bitwriter_add<'a1, 'a2>(
    mut bw: Option<&'a1 mut crate::src::lib::tflac_bitwriter<'a2>>,
    mut bits: u32,
    mut val: u64,
) -> libc::unix::c_int {
    let mask: tflac_uint = (18446744073709551615 as tflac_uint) << 1 as libc::c_int;
    let mut b: tflac_u32 = 0;
    let mut r: libc::c_int = 0;
    val <<= (8 as usize)
        .wrapping_mul(std::mem::size_of::<tflac_uint>() as usize)
        .wrapping_sub(bits as usize);
    (*borrow_mut(&mut bw).unwrap()).tot = ((*borrow_mut(&mut bw).unwrap()).tot as libc::c_uint).wrapping_add(bits as libc::c_uint)
        as tflac_u32 as tflac_u32;
    let mut i: libc::c_int = 0 as libc::c_int;
    while (*borrow_mut(&mut bw).unwrap()).bits.wrapping_add(bits) as usize
        >= (8 as usize).wrapping_mul(std::mem::size_of::<tflac_uint>() as usize)
        && i < 100 as libc::c_int
    {
        b = (8 as usize)
            .wrapping_mul(std::mem::size_of::<tflac_uint>() as usize)
            .wrapping_sub((*borrow_mut(&mut bw).unwrap()).bits as usize)
            .wrapping_sub(1 as usize) as tflac_u32;
        b = if b > bits { bits } else { b };
        (*borrow_mut(&mut bw).unwrap()).val = ((*borrow(& bw).unwrap()).val as libc::c_ulong
            | (val >> (*borrow(& bw).unwrap()).bits) as libc::c_ulong) as tflac_uint;
        (*borrow_mut(&mut bw).unwrap()).bits = ((*borrow_mut(&mut bw).unwrap()).bits as libc::c_uint).wrapping_add(b as libc::c_uint)
            as tflac_u32 as tflac_u32;
        (*borrow_mut(&mut bw).unwrap()).val =
            ((*borrow(& bw).unwrap()).val as libc::c_ulong & mask as libc::c_ulong) as tflac_uint;
        val <<= b;
        bits = (bits as libc::c_uint).wrapping_sub(b as libc::c_uint) as tflac_u32
            as tflac_u32;
        i += 1;
    }
    (*borrow_mut(&mut bw).unwrap()).val = ((*borrow(& bw).unwrap()).val as libc::c_ulong | (val >> (*borrow(& bw).unwrap()).bits) as libc::c_ulong)
        as tflac_uint;
    (*borrow_mut(&mut bw).unwrap()).bits = ((*borrow_mut(&mut bw).unwrap()).bits as libc::c_uint).wrapping_add(bits as libc::c_uint)
        as tflac_u32 as tflac_u32;
    return 0 as libc::c_int;
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

