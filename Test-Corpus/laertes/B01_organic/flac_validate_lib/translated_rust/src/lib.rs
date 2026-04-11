pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type tflac_u8 = u8;
pub type tflac_u32 = u32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac {
    pub blocksize: u32,
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub max_rice_value: u8,
    pub min_partition_order: u8,
    pub max_partition_order: u8,
    pub partition_order: u8,
    pub cur_blocksize: u32,
}
impl std::default::Default for tflac {
    fn default() -> Self {
        tflac {
        blocksize: u32::default(),
        samplerate: u32::default(),
        channels: u32::default(),
        bitdepth: u32::default(),
        channel_mode: u8::default(),
        max_rice_value: u8::default(),
        min_partition_order: u8::default(),
        max_partition_order: u8::default(),
        partition_order: u8::default(),
        cur_blocksize: u32::default()
        }
    }
}

pub const TFLAC_CHANNEL_INDEPENDENT: TFLAC_CHANNEL_MODE = 0;
pub type TFLAC_CHANNEL_MODE = libc::unix::c_uint;
pub const TFLAC_CHANNEL_MODE_COUNT: TFLAC_CHANNEL_MODE = 4;
pub const TFLAC_CHANNEL_MID_SIDE: TFLAC_CHANNEL_MODE = 3;
pub const TFLAC_CHANNEL_SIDE_RIGHT: TFLAC_CHANNEL_MODE = 2;
pub const TFLAC_CHANNEL_LEFT_SIDE: TFLAC_CHANNEL_MODE = 1;
#[no_mangle]
pub extern "C" fn tflac_size_memory(mut blocksize: tflac_u32) -> tflac_u32 {
    return (15 as libc::c_uint as tflac_u32).wrapping_add((5 as tflac_u32).wrapping_mul(
        (15 as tflac_u32).wrapping_add(blocksize.wrapping_mul(4 as tflac_u32))
            & 0xfffffff0 as tflac_u32,
    ));
}
#[no_mangle]
pub unsafe extern "C" fn flac_validate<'a1>(mut t: Option<&'a1 mut crate::src::lib::tflac>) -> libc::unix::c_int {
    if (*borrow(& t).unwrap()).blocksize < 16 as tflac_u32 {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).blocksize > 65535 as tflac_u32 {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).samplerate == 0 as tflac_u32 {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).samplerate > 655350 as tflac_u32 {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).channels == 0 as tflac_u32 {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).channels > 8 as tflac_u32 {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).bitdepth == 0 as tflac_u32 {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).bitdepth > 32 as tflac_u32 {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).channel_mode as libc::c_int != TFLAC_CHANNEL_INDEPENDENT as libc::c_int {
        if (*borrow(& t).unwrap()).channels != 2 as tflac_u32 || (*borrow(& t).unwrap()).bitdepth == 32 as tflac_u32 {
            (*borrow_mut(&mut t).unwrap()).channel_mode = TFLAC_CHANNEL_INDEPENDENT as libc::c_int as tflac_u8;
        }
    }
    if (*borrow(& t).unwrap()).max_rice_value as libc::c_int == 0 as libc::c_int {
        if (*borrow(& t).unwrap()).bitdepth <= 16 as tflac_u32 {
            (*borrow_mut(&mut t).unwrap()).max_rice_value = 14 as tflac_u8;
        } else {
            (*borrow_mut(&mut t).unwrap()).max_rice_value = 30 as tflac_u8;
        }
    } else if (*borrow(& t).unwrap()).max_rice_value as libc::c_int > 30 as libc::c_int {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).max_partition_order as libc::c_int > 15 as libc::c_int {
        return -(1 as libc::c_int);
    }
    if (*borrow(& t).unwrap()).min_partition_order as libc::c_int
        > (*borrow(& t).unwrap()).max_partition_order as libc::c_int
    {
        return -(1 as libc::c_int);
    }
    (*borrow_mut(&mut t).unwrap()).partition_order = (*borrow_mut(&mut t).unwrap()).min_partition_order;
    while (*borrow_mut(&mut t).unwrap()).blocksize.wrapping_rem(
        ((1 as libc::c_int)
            << (*borrow(& t).unwrap()).partition_order as libc::c_int + 1 as libc::c_int)
            as tflac_u32,
    ) == 0 as tflac_u32
        && ((*borrow(& t).unwrap()).partition_order as libc::c_int)
            < (*borrow(& t).unwrap()).max_partition_order as libc::c_int
    {
        (*borrow_mut(&mut t).unwrap()).partition_order = (*borrow_mut(&mut t).unwrap()).partition_order.wrapping_add(1);
    }
    (*borrow_mut(&mut t).unwrap()).cur_blocksize = (*borrow_mut(&mut t).unwrap()).blocksize;
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

